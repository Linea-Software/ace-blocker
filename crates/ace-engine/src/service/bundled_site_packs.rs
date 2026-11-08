//! Offline "bundled" site-pack import.
//!
//! The Windows installer ships a small set of site-pack artifacts (the packs
//! selected by the offline-bundling audit — the default Free packs) under
//! `[INSTALLFOLDER]\bundled-packs\` together with a versioned `manifest.json`
//! produced by the `export-site-pack-bundle` dev bin. This module makes those
//! packs functional on the first run **without internet access**:
//!
//! 1. It discovers and validates the manifest and per-artifact SHA-256
//!    checksums.
//! 2. It registers/imports only packs whose usable rules are present (a
//!    corrupt or missing artifact never produces a row or a data directory).
//! 3. It never overwrites a newer downloaded — or last-known-good — pack with
//!    an older bundled version.
//! 4. It is idempotent and converges: every startup re-runs the pass and the
//!    per-pack operations are atomic (stage → validate → directory swap →
//!    row update), so a crash mid-pass leaves either the previous state or a
//!    state the next pass completes.
//!
//! Precedence (per the offline-bundling spec):
//!
//! ```text
//! valid newer downloaded pack → valid bundled pack → unavailable
//! ```
//!
//! Concretely, for each manifest entry:
//! - no local row → register the row (id = remote_id = canonical id) and
//!   install the bundled data;
//! - local row marked `is_ignored` (the user deleted the pack from the UI) →
//!   skip, never resurrect;
//! - local data missing or invalid → fall back to the bundled copy;
//! - local data valid: keep downloaded/local data untouched; for previously
//!   bundled data (identified by the marker), refresh only when the canonical
//!   numeric three-component bundle version is strictly newer.
//! - no legacy version formats are supported by this pre-release build.
//!
//! Packs installed from the bundle are marked with a small marker file
//! (`.bundled`) next to `pack.db` so the API can report "bundled and ready"
//! vs "downloaded and ready". Any later engine update that swaps the pack
//! directory (downloaded refresh / user resync) replaces the whole directory
//! and therefore clears the marker — the label always reflects disk state.

use crate::{
    core::{
        BundleSchemaVersion, PackDbFormatVersion, RemoteSitePackId, Sha256Digest, SitePackId,
        SitePackVersion, network_block::site_pack::site_pack_db::SitePackDB,
    },
    db::EngineDB,
    settings::Settings,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use tracing::{info, warn};

/// Name of the bundled-packs directory under the install directory.
pub const BUNDLED_PACKS_DIR_NAME: &str = "bundled-packs";
/// File name of the versioned bundle manifest.
pub const BUNDLED_MANIFEST_FILE: &str = "manifest.json";
/// Marker file written next to `pack.db` when the data was installed from the
/// installer bundle (cleared automatically by any engine directory swap).
pub const BUNDLED_MARKER_FILE: &str = ".bundled";
/// Manifest schema version this engine understands.
pub const SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;

// ── Manifest model (mirrors `export_site_pack_bundle`'s output) ───────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BundledManifest {
    pub schema_version: BundleSchemaVersion,
    pub build_date: String,
    #[serde(default)]
    pub generated_at: Option<String>,
    pub pack_db_format_version: PackDbFormatVersion,
    pub packs: Vec<BundledPackEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BundledPackEntry {
    pub id: RemoteSitePackId,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub requires_pro: bool,
    pub content_version: SitePackVersion,
    pub sha256: Sha256Digest,
    pub size_bytes: u64,
    pub entry_count: u64,
    #[serde(default)]
    pub built_at: Option<String>,
    pub file: String,
}

/// What the pass did for one bundled pack.
#[derive(Debug, Clone, PartialEq)]
pub enum BundledPackAction {
    /// Data installed (row created or existing row refreshed).
    Installed,
    /// Kept the existing valid local data.
    KeptExisting,
    /// Skipped because the user marked the pack ignored.
    SkippedIgnored,
    /// Rejected: artifact/validation problem; nothing was changed.
    Rejected(String),
}

#[derive(Debug, Default)]
pub struct BundledImportReport {
    pub actions: Vec<(RemoteSitePackId, BundledPackAction)>,
}

impl BundledImportReport {
    pub fn installed(&self) -> Vec<&str> {
        self.actions
            .iter()
            .filter(|(_, action)| matches!(action, BundledPackAction::Installed))
            .map(|(id, _)| id.as_ref())
            .collect()
    }

    pub fn rejected(&self) -> Vec<(&str, &str)> {
        self.actions
            .iter()
            .filter_map(|(id, action)| match action {
                BundledPackAction::Rejected(reason) => Some((id.as_ref(), reason.as_str())),
                _ => None,
            })
            .collect()
    }
}

// ── Discovery ─────────────────────────────────────────────────────────

/// Directory the installer's bundled packs live in (`{install dir}/bundled-packs`).
pub fn get_bundled_packs_dir() -> PathBuf {
    Settings::get_bundled_packs_dir_override()
        .unwrap_or_else(|| Settings::get_install_dir().join(BUNDLED_PACKS_DIR_NAME))
}

/// Loads and structurally validates the bundled manifest.
///
/// Returns `None` (with a warning) when there is no bundle, the manifest is
/// unreadable, the schema version is unsupported, or the manifest contains
/// duplicate entries — a broken bundle must never break engine startup.
pub fn load_manifest(bundle_dir: &Path) -> Option<BundledManifest> {
    let manifest_path = bundle_dir.join(BUNDLED_MANIFEST_FILE);
    if !manifest_path.is_file() {
        return None;
    }

    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(content) => content,
        Err(error) => {
            warn!(
                "bundled site-pack manifest {} is unreadable: {error}",
                manifest_path.display()
            );
            return None;
        }
    };

    let manifest: BundledManifest = match serde_json::from_str(&content) {
        Ok(manifest) => manifest,
        Err(error) => {
            warn!(
                "bundled site-pack manifest {} is not valid JSON: {error}",
                manifest_path.display()
            );
            return None;
        }
    };

    if manifest.schema_version.into_inner() != SUPPORTED_MANIFEST_SCHEMA_VERSION {
        warn!(
            "bundled site-pack manifest schema version {} is not supported (expected {}); ignoring bundle",
            manifest.schema_version, SUPPORTED_MANIFEST_SCHEMA_VERSION
        );
        return None;
    }

    let mut seen_ids: BTreeSet<&str> = BTreeSet::new();
    for entry in &manifest.packs {
        if !seen_ids.insert(entry.id.as_ref()) {
            warn!(
                "bundled site-pack manifest contains duplicate or empty pack id {:?}; ignoring bundle",
                entry.id
            );
            return None;
        }
        if !is_safe_relative_path(&entry.file) {
            warn!(
                "bundled site-pack manifest entry {:?} has unsafe file path {:?}; ignoring bundle",
                entry.id, entry.file
            );
            return None;
        }
    }

    Some(manifest)
}

/// Rejects absolute paths, `..`, backslashes and drive prefixes in manifest
/// file fields (defense in depth — mirrors `stage_backup_archive`).
fn is_safe_relative_path(raw: &str) -> bool {
    !raw.is_empty()
        && !raw.starts_with('/')
        && !raw.starts_with('\\')
        && !raw.contains('\\')
        && !raw.contains(':')
        && !raw.split('/').any(|segment| segment == "..")
}

// ── Validation helpers ────────────────────────────────────────────────

/// True when a pack directory contains usable local rules: the `pack.db`
/// exists, opens, and reports a positive entry count. Missing or corrupt
/// files (including a dir without a database) are "not valid".
fn pack_dir_has_usable_rules(pack_dir: &Path) -> bool {
    let db_file = pack_dir.join("pack.db");
    if !db_file.is_file() {
        return false;
    }
    let db = match SitePackDB::from(&db_file) {
        Ok(db) => db,
        Err(_) => return false,
    };
    match db.count_entries() {
        Ok(count) => count > 0,
        Err(_) => false,
    }
}

// ── Import pass ───────────────────────────────────────────────────────

/// Runs the bundled-pack import pass. Idempotent; safe to run at every
/// startup. Operates purely on the database + the sites directory — it must
/// run before the [`crate::manager::NetworkManager`] is constructed so the
/// manager loads rows that already have data (no ghosts).
pub fn apply_bundled_packs(db: &EngineDB) -> BundledImportReport {
    let bundle_dir = get_bundled_packs_dir();
    let mut report = BundledImportReport::default();

    let Some(manifest) = load_manifest(&bundle_dir) else {
        return report;
    };

    info!(
        "Applying bundled site packs from {} ({} packs, build {})",
        bundle_dir.display(),
        manifest.packs.len(),
        manifest.build_date
    );

    for entry in &manifest.packs {
        let artifact_path = bundle_dir.join(&entry.file);
        let action = import_bundled_pack(db, &artifact_path, entry);
        report.actions.push((entry.id.clone(), action));
    }

    report
}

fn import_bundled_pack(
    db: &EngineDB,
    artifact_path: &Path,
    entry: &BundledPackEntry,
) -> BundledPackAction {
    // 1. Artifact must be present and checksum-verified before anything else.
    if !artifact_path.is_file() {
        warn!(
            "bundled pack {} artifact missing at {}",
            entry.id,
            artifact_path.display()
        );
        return BundledPackAction::Rejected("artifact file missing".to_string());
    }
    let bytes = match std::fs::read(artifact_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!("bundled pack {} artifact unreadable: {error}", entry.id);
            return BundledPackAction::Rejected("artifact unreadable".to_string());
        }
    };
    if bytes.len() as u64 != entry.size_bytes {
        warn!(
            "bundled pack {} size mismatch (expected {}, got {})",
            entry.id,
            entry.size_bytes,
            bytes.len()
        );
        return BundledPackAction::Rejected("artifact size mismatch".to_string());
    }
    if Sha256Digest::from_bytes(&bytes) != entry.sha256 {
        warn!("bundled pack {} sha256 mismatch — rejecting", entry.id);
        return BundledPackAction::Rejected("sha256 checksum mismatch".to_string());
    }

    // Avoid an open/retry loop on malformed non-SQLite payloads. The SHA-256
    // and file-length checks above have already verified artifact integrity.
    if !bytes.starts_with(b"SQLite format 3\0") {
        warn!("bundled pack {} has no SQLite header", entry.id);
        return BundledPackAction::Rejected("invalid SQLite database header".to_string());
    }

    // 2. The artifact must itself be a usable pack database (opens + non-zero
    //    entries + entry count consistent with the manifest).
    {
        let staged_check = match std::fs::read(artifact_path) {
            Ok(_) => artifact_path.to_path_buf(),
            Err(error) => {
                return BundledPackAction::Rejected(format!("artifact unreadable: {error}"));
            }
        };
        let db_check = match SitePackDB::from(&staged_check) {
            Ok(db) => db,
            Err(error) => {
                warn!(
                    "bundled pack {} database failed to open: {}",
                    entry.id, error
                );
                return BundledPackAction::Rejected("database does not open".to_string());
            }
        };
        match db_check.count_entries() {
            Ok(count) if count > 0 && count as u64 == entry.entry_count => {}
            Ok(count) => {
                warn!(
                    "bundled pack {} entry count mismatch (manifest {}, db {})",
                    entry.id, entry.entry_count, count
                );
                return BundledPackAction::Rejected("entry count mismatch".to_string());
            }
            Err(error) => {
                warn!("bundled pack {} database unreadable: {error}", entry.id);
                return BundledPackAction::Rejected("database unreadable".to_string());
            }
        }
    }

    // 3. Find the local row by remote_id (the canonical id).
    let existing = db.get_site_pack_by_remote_id(&entry.id).ok();

    // The user deleted this pack from the UI → it is marked ignored; never
    // resurrect it on the next startup.
    if let Some(row) = &existing
        && row.is_ignored
    {
        info!("bundled pack {} is ignored by the user; skipping", entry.id);
        return BundledPackAction::SkippedIgnored;
    }

    // 4. Decide: does the local state already satisfy the precedence?
    let path_component = existing
        .as_ref()
        .map(|row| row.id.as_str())
        .unwrap_or(entry.id.as_ref());
    let site_pack_id = match SitePackId::try_new(path_component.to_string()) {
        Ok(id) => id,
        Err(error) => {
            return BundledPackAction::Rejected(format!(
                "unsafe persisted site-pack path component: {error}"
            ));
        }
    };
    let live_dir = Settings::get_site_pack_path(&site_pack_id);
    // Only an *existing row* can hold "valid local data": a stray directory
    // without a row (e.g. a crash between the directory swap and the row
    // insert) must be adopted via a fresh install, not silently kept.
    let local_valid = existing.is_some() && pack_dir_has_usable_rules(&live_dir);

    if local_valid {
        let local_version = existing.as_ref().and_then(|row| row.version.clone());
        // Bundles may update only previous installer-bundled data, never
        // replace a valid online/local pack based on arbitrary version strings.
        let is_bundled = live_dir.join(BUNDLED_MARKER_FILE).is_file();
        let keep = if !is_bundled {
            true
        } else {
            match local_version {
                None => true, // Preserve a valid last-known-good without version metadata.
                Some(local) => SitePackVersion::try_new(local)
                    .map(|current| !entry.content_version.is_newer_than(&current))
                    .unwrap_or(false), // No legacy version recovery: replace invalid metadata.
            }
        };
        if keep {
            info!(
                "bundled pack {} already has valid local data; keeping it",
                entry.id
            );
            return BundledPackAction::KeptExisting;
        }
    }

    // 5. Install (or refresh) the bundled data with an atomic directory swap,
    //    then persist the row. Order: swap the data first, then the row, so a
    //    crash never leaves a row pointing at data that is missing; the next
    //    startup pass converges (idempotent).
    let sites_dir = Settings::get_site_packs_dir();
    if let Err(error) = std::fs::create_dir_all(&sites_dir) {
        warn!(
            "failed to create sites dir {}: {error}",
            sites_dir.display()
        );
        return BundledPackAction::Rejected(format!("sites dir unavailable: {error}"));
    }

    let unique = uuid::Uuid::new_v4();
    let staging_dir = sites_dir.join(format!(".bundled-staging-{unique}"));
    let old_dir = sites_dir.join(format!(".{}.bundled-old-{}", entry.id, unique));

    // Stage: copy the verified artifact into the staging dir as pack.db.
    if let Err(error) = std::fs::create_dir_all(&staging_dir) {
        warn!("failed to create staging dir: {error}");
        return BundledPackAction::Rejected(format!("staging failed: {error}"));
    }
    if let Err(error) = std::fs::copy(artifact_path, staging_dir.join("pack.db")) {
        let _ = std::fs::remove_dir_all(&staging_dir);
        warn!("failed to stage bundled pack {}: {error}", entry.id);
        return BundledPackAction::Rejected(format!("staging failed: {error}"));
    }

    // Swap: live dir → old dir, staging → live, roll back on failure.
    let live_exists = live_dir.exists();
    let swapped = (|| -> Result<(), String> {
        if live_exists {
            std::fs::rename(&live_dir, &old_dir).map_err(|error| {
                format!("failed to move the current pack directory aside: {error}")
            })?;
        }
        if let Err(error) = std::fs::rename(&staging_dir, &live_dir) {
            if live_exists {
                let _ = std::fs::rename(&old_dir, &live_dir);
            }
            return Err(format!("failed to activate the bundled pack: {error}"));
        }
        // The activated copy must open (it was validated from the artifact,
        // but the file moved — re-check once for full safety).
        if let Err(error) = SitePackDB::from(&live_dir.join("pack.db")) {
            let _ = std::fs::remove_dir_all(&live_dir);
            if live_exists {
                let _ = std::fs::rename(&old_dir, &live_dir);
            }
            return Err(format!("activated bundled pack failed to open: {error}"));
        }
        Ok(())
    })();
    if let Err(error) = swapped {
        let _ = std::fs::remove_dir_all(&staging_dir);
        let _ = std::fs::remove_dir_all(&old_dir);
        warn!("bundled pack {} install failed: {error}", entry.id);
        return BundledPackAction::Rejected(error);
    }
    let _ = std::fs::remove_dir_all(&old_dir);

    // Mark the live directory as bundle-origin.
    if let Err(error) = std::fs::write(live_dir.join(BUNDLED_MARKER_FILE), b"bundled\n") {
        warn!("failed to write bundled marker for {}: {error}", entry.id);
    }

    // 6. Persist the row.
    let row_result = if let Some(row) = &existing {
        // Row exists: refresh version + timestamp on the same row (id may be
        // a seeded catalog id or an engine uuid — both fine).
        let version = entry.content_version.clone();
        let built_at = entry
            .built_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|date| date.with_timezone(&chrono::Local));
        let version_result = db.update_site_pack_version(&row.id, &version);
        let timestamp_result = match built_at {
            Some(date) => db.update_site_pack_last_updated(&row.id, &date),
            None => Ok(()),
        };
        version_result
            .and(timestamp_result)
            .map(|_| row.id.clone())
            .map_err(|error| error.to_string())
    } else {
        // No row: register with id = remote_id = canonical id (the 0022 seed
        // pattern), so future join rows and updater matches stay stable.
        let version = entry.content_version.clone();
        db.add_remote_site_pack(
            &entry.id,
            &entry.id,
            &entry.name,
            entry.description.clone(),
            entry.requires_pro,
            &version,
        )
        .map(|_| entry.id.to_string())
        .map_err(|error| error.to_string())
    };

    if let Err(error) = row_result {
        warn!(
            "bundled pack {} data installed but row persistence failed: {error}",
            entry.id
        );
        return BundledPackAction::Rejected(format!("row persistence failed: {error}"));
    }

    info!(
        "Installed bundled site pack {} (content {})",
        entry.id, entry.content_version
    );
    BundledPackAction::Installed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network_block::site_pack::site_pack_db::{NewPackMetaModel, hash_host};
    use crate::core::network_block::uri_hash::UriHash;
    use crate::db::EngineDB;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use uuid::Uuid;

    // The `Settings` overrides are process-global, so all tests in this module
    // share one temp tree and run serially under a mutex, resetting the shared
    // directories at the start of every test (TESTINFRA-01 discipline).
    struct Shared {
        root: PathBuf,
        sites_dir: PathBuf,
        bundle_dir: PathBuf,
    }

    static SHARED: OnceLock<Shared> = OnceLock::new();
    static SERIAL: Mutex<()> = Mutex::new(());

    fn shared() -> &'static Shared {
        SHARED.get_or_init(|| {
            let root = std::env::temp_dir().join(format!("ace-bundled-test-{}", Uuid::new_v4()));
            let sites_dir = root.join("sites_db");
            let bundle_dir = root.join("bundled-packs");
            std::fs::create_dir_all(&sites_dir).expect("create shared sites dir");
            std::fs::create_dir_all(&bundle_dir).expect("create shared bundle dir");
            Settings::set_test_sites_dir(sites_dir.clone());
            Settings::set_test_bundled_packs_dir(bundle_dir.clone());
            Shared {
                root,
                sites_dir,
                bundle_dir,
            }
        })
    }

    struct TestEnv {
        sites_dir: PathBuf,
        bundle_dir: PathBuf,
        db: EngineDB,
        _serial: MutexGuard<'static, ()>,
    }

    fn setup() -> TestEnv {
        let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let shared = shared();
        let _ = std::fs::remove_dir_all(&shared.sites_dir);
        let _ = std::fs::remove_dir_all(&shared.bundle_dir);
        std::fs::create_dir_all(&shared.sites_dir).expect("recreate shared sites dir");
        std::fs::create_dir_all(&shared.bundle_dir).expect("recreate shared bundle dir");

        // Unique DB per test; migrations seed the two default free pack rows
        // (`pack-adult-free`, `pack-social-free`) exactly like a real install.
        let db_path = shared
            .root
            .join(format!("engine-{}.sqlite", Uuid::new_v4()));
        let db = EngineDB::from(&db_path).expect("open engine db");
        TestEnv {
            sites_dir: shared.sites_dir.clone(),
            bundle_dir: shared.bundle_dir.clone(),
            db,
            _serial,
        }
    }

    fn write_pack_db(dir: &Path, id: &str, hosts: &[&str]) -> Vec<u8> {
        std::fs::create_dir_all(dir).unwrap();
        let db_path = dir.join("pack.db");
        let db = SitePackDB::from(&db_path).unwrap();
        let hashes: Vec<UriHash> = hosts.iter().map(|host| hash_host(host)).collect();
        db.insert_entries(&hashes).unwrap();
        db.insert_pack_meta(NewPackMetaModel {
            id: id.to_string(),
            format_version: "1.0".to_string(),
            name: None,
            entry_count: hosts.len() as i32,
            built_at: "2026-09-02T00:00:00Z".to_string(),
        })
        .unwrap();
        std::fs::read(&db_path).unwrap()
    }

    fn entry(id: &str, version: &str, bytes: &[u8], entry_count: u64) -> BundledPackEntry {
        BundledPackEntry {
            id: RemoteSitePackId::try_new(id.to_string()).unwrap(),
            name: format!("Pack {id}"),
            description: Some("A test pack".to_string()),
            requires_pro: false,
            content_version: SitePackVersion::try_new(version.to_string()).unwrap(),
            sha256: Sha256Digest::from_bytes(bytes),
            size_bytes: bytes.len() as u64,
            entry_count,
            built_at: Some("2026-09-02T00:00:00Z".to_string()),
            file: format!("{id}/pack.db"),
        }
    }

    fn write_manifest(env: &TestEnv, packs: Vec<BundledPackEntry>) {
        let manifest = BundledManifest {
            schema_version: BundleSchemaVersion::try_new(SUPPORTED_MANIFEST_SCHEMA_VERSION)
                .unwrap(),
            build_date: "2026-09-02".to_string(),
            generated_at: None,
            pack_db_format_version: PackDbFormatVersion::try_new("1.0".to_string()).unwrap(),
            packs,
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(env.bundle_dir.join(BUNDLED_MANIFEST_FILE), bytes).unwrap();
    }

    fn db_contains(env: &TestEnv, id: &str, host: &str) -> bool {
        let db = SitePackDB::from(&env.sites_dir.join(id).join("pack.db")).unwrap();
        db.blocking_result(&hash_host(host)).unwrap().is_blocked()
    }

    #[test]
    fn empty_without_manifest() {
        let env = setup();
        let report = apply_bundled_packs(&env.db);
        assert!(report.actions.is_empty());
    }

    #[test]
    fn installs_new_unseeded_pack_and_is_idempotent() {
        let env = setup();
        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-gaming-free"),
            "pack-gaming-free",
            &["game-one.example.com", "game-two.example.com"],
        );
        write_manifest(
            &env,
            vec![entry("pack-gaming-free", "2026.09.02", &bytes, 2)],
        );

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.installed(), vec!["pack-gaming-free"]);
        let row = env
            .db
            .get_site_pack_by_remote_id("pack-gaming-free")
            .unwrap();
        assert_eq!(
            row.id, "pack-gaming-free",
            "row id must equal the canonical id"
        );
        assert_eq!(row.version.as_deref(), Some("2026.09.02"));
        assert!(!row.requires_pro);
        assert!(db_contains(
            &env,
            "pack-gaming-free",
            "game-one.example.com"
        ));
        assert!(env.sites_dir.join("pack-gaming-free/.bundled").is_file());

        // Second run: no duplicates, existing data kept.
        let report2 = apply_bundled_packs(&env.db);
        assert!(report2.installed().is_empty());
        assert_eq!(report2.actions[0].1, BundledPackAction::KeptExisting);
        let all = env.db.get_all_site_packs().unwrap();
        assert_eq!(
            all.len(),
            5,
            "four seeded rows + one bundled row — no duplicates"
        );
    }

    #[test]
    fn fills_gap_for_seeded_row_without_data() {
        // Mirrors a fresh offline install: migration 0021 seeded
        // pack-adult-free with no data dir. The bundle must fill the gap on
        // the SAME row and become usable.
        let env = setup();
        assert!(
            env.db.get_site_pack_by_remote_id("pack-adult-free").is_ok(),
            "0021 seeds pack-adult-free"
        );

        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-adult-free"),
            "pack-adult-free",
            &["porn.example.com", "adult.example.com"],
        );
        write_manifest(
            &env,
            vec![entry("pack-adult-free", "2026.09.02", &bytes, 2)],
        );

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.installed(), vec!["pack-adult-free"]);
        let row = env
            .db
            .get_site_pack_by_remote_id("pack-adult-free")
            .unwrap();
        assert_eq!(row.version.as_deref(), Some("2026.09.02"));
        assert!(db_contains(&env, "pack-adult-free", "porn.example.com"));
        assert!(env.sites_dir.join("pack-adult-free/.bundled").is_file());

        // Idempotent: a second pass keeps the now-valid data.
        let report2 = apply_bundled_packs(&env.db);
        assert_eq!(report2.actions[0].1, BundledPackAction::KeptExisting);
        assert_eq!(env.db.get_all_site_packs().unwrap().len(), 4);
    }

    #[test]
    fn corrupt_artifact_is_rejected_safely() {
        let env = setup();
        let dir = env.bundle_dir.join("pack-gaming-free");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pack.db"), b"not a sqlite database").unwrap();
        let bytes = std::fs::read(dir.join("pack.db")).unwrap();
        write_manifest(
            &env,
            vec![entry("pack-gaming-free", "2026.09.02", &bytes, 1)],
        );

        let report = apply_bundled_packs(&env.db);
        assert!(report.installed().is_empty());
        assert_eq!(report.rejected().len(), 1);
        assert!(
            env.db
                .get_site_pack_by_remote_id("pack-gaming-free")
                .is_err()
        );
        assert!(!env.sites_dir.join("pack-gaming-free").exists());
    }

    #[test]
    fn sha256_mismatch_is_rejected_safely() {
        let env = setup();
        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-gaming-free"),
            "pack-gaming-free",
            &["social.example.com"],
        );
        let mut bad = entry("pack-gaming-free", "2026.09.02", &bytes, 1);
        bad.sha256 = Sha256Digest::try_new("0".repeat(64)).unwrap(); // wrong checksum
        write_manifest(&env, vec![bad]);

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.rejected().len(), 1);
        assert!(
            env.db
                .get_site_pack_by_remote_id("pack-gaming-free")
                .is_err()
        );
    }

    #[test]
    fn unsupported_schema_version_is_ignored_safely() {
        let env = setup();
        let manifest = BundledManifest {
            schema_version: BundleSchemaVersion::try_new(999).unwrap(),
            build_date: "2026-09-02".to_string(),
            generated_at: None,
            pack_db_format_version: PackDbFormatVersion::try_new("1.0".to_string()).unwrap(),
            packs: vec![],
        };
        std::fs::write(
            env.bundle_dir.join(BUNDLED_MANIFEST_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let report = apply_bundled_packs(&env.db);
        assert!(report.actions.is_empty(), "unsupported schema must no-op");
    }

    #[test]
    fn newer_downloaded_pack_beats_older_bundled() {
        let env = setup();

        // Simulate a downloaded pack on the seeded row: newer bundle-style
        // version + valid data already present.
        env.db
            .update_site_pack_version("pack-adult-free", "2026.10.01")
            .unwrap();
        write_pack_db(
            &env.sites_dir.join("pack-adult-free"),
            "pack-adult-free",
            &["downloaded-newer.example.com"],
        );

        // A bundle with an OLDER content version arrives.
        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-adult-free"),
            "pack-adult-free",
            &["bundled-older.example.com"],
        );
        write_manifest(
            &env,
            vec![entry("pack-adult-free", "2026.09.02", &bytes, 1)],
        );

        let report = apply_bundled_packs(&env.db);
        assert_eq!(
            report.actions[0].1,
            BundledPackAction::KeptExisting,
            "a newer downloaded pack must win over an older bundled pack"
        );
        assert!(db_contains(
            &env,
            "pack-adult-free",
            "downloaded-newer.example.com"
        ));
        assert!(!db_contains(
            &env,
            "pack-adult-free",
            "bundled-older.example.com"
        ));
        assert!(!env.sites_dir.join("pack-adult-free/.bundled").exists());
        let row = env
            .db
            .get_site_pack_by_remote_id("pack-adult-free")
            .unwrap();
        assert_eq!(row.version.as_deref(), Some("2026.10.01"));
    }

    #[test]
    fn newer_bundle_refreshes_older_bundle_style_local_data() {
        let env = setup();

        // Local valid data recorded with an older bundle-style version.
        env.db
            .add_remote_site_pack(
                "pack-gaming-free",
                "pack-gaming-free",
                "Gaming Websites — Free",
                None,
                false,
                "2026.08.01",
            )
            .unwrap();
        write_pack_db(
            &env.sites_dir.join("pack-gaming-free"),
            "pack-gaming-free",
            &["old-game.example.com"],
        );
        // This row represents a previous installer bundle, not an online update.
        std::fs::write(env.sites_dir.join("pack-gaming-free/.bundled"), b"").unwrap();

        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-gaming-free"),
            "pack-gaming-free",
            &["new-game.example.com"],
        );
        write_manifest(
            &env,
            vec![entry("pack-gaming-free", "2026.09.02", &bytes, 1)],
        );

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.actions[0].1, BundledPackAction::Installed);
        assert!(db_contains(
            &env,
            "pack-gaming-free",
            "new-game.example.com"
        ));
        assert!(!db_contains(
            &env,
            "pack-gaming-free",
            "old-game.example.com"
        ));
        assert!(env.sites_dir.join("pack-gaming-free/.bundled").is_file());
        let row = env
            .db
            .get_site_pack_by_remote_id("pack-gaming-free")
            .unwrap();
        assert_eq!(row.version.as_deref(), Some("2026.09.02"));
    }

    #[test]
    fn semver_local_data_is_never_clobbered_by_a_bundle() {
        let env = setup();

        // Local valid data recorded with a site semver — not comparable with
        // the bundle's date version, so the bundle must only fill gaps.
        env.db
            .update_site_pack_version("pack-adult-free", "2.1.0")
            .unwrap();
        write_pack_db(
            &env.sites_dir.join("pack-adult-free"),
            "pack-adult-free",
            &["site-data.example.com"],
        );

        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-adult-free"),
            "pack-adult-free",
            &["bundle-data.example.com"],
        );
        write_manifest(
            &env,
            vec![entry("pack-adult-free", "2026.09.02", &bytes, 1)],
        );

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.actions[0].1, BundledPackAction::KeptExisting);
        assert!(db_contains(
            &env,
            "pack-adult-free",
            "site-data.example.com"
        ));
        assert!(!db_contains(
            &env,
            "pack-adult-free",
            "bundle-data.example.com"
        ));
        let row = env
            .db
            .get_site_pack_by_remote_id("pack-adult-free")
            .unwrap();
        assert_eq!(row.version.as_deref(), Some("2.1.0"));
    }

    #[test]
    fn ignored_pack_is_not_resurrected() {
        let env = setup();
        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-social-free"),
            "pack-social-free",
            &["social.example.com"],
        );
        write_manifest(
            &env,
            vec![entry("pack-social-free", "2026.09.02", &bytes, 1)],
        );

        // The user deleted the seeded pack from the UI → it is ignored (the
        // row + its data stay, exactly like the delete flow leaves it).
        env.db
            .set_site_pack_ignored("pack-social-free", true)
            .unwrap();
        write_pack_db(
            &env.sites_dir.join("pack-social-free"),
            "pack-social-free",
            &["whatever.example.com"],
        );

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.actions[0].1, BundledPackAction::SkippedIgnored);
        let row = env
            .db
            .get_site_pack_by_remote_id("pack-social-free")
            .unwrap();
        assert!(row.is_ignored, "ignored state must survive the pass");
    }

    #[test]
    fn unreadable_manifest_is_ignored_safely() {
        let env = setup();
        std::fs::write(
            env.bundle_dir.join(BUNDLED_MANIFEST_FILE),
            b"{ this is not json",
        )
        .unwrap();
        let report = apply_bundled_packs(&env.db);
        assert!(report.actions.is_empty());
    }

    #[test]
    fn missing_artifact_entry_is_rejected_without_touching_state() {
        let env = setup();
        let bytes = write_pack_db(
            &env.bundle_dir.join("pack-gaming-free"),
            "pack-gaming-free",
            &["host.example.com"],
        );
        let mut pack = entry("pack-gaming-free", "2026.09.02", &bytes, 1);
        pack.file = "pack-gaming-free/pack.db".to_string();
        write_manifest(&env, vec![pack]);
        // Remove the artifact so the manifest points at nothing.
        std::fs::remove_file(env.bundle_dir.join("pack-gaming-free/pack.db")).unwrap();

        let report = apply_bundled_packs(&env.db);
        assert_eq!(report.rejected().len(), 1);
        assert!(
            env.db
                .get_site_pack_by_remote_id("pack-gaming-free")
                .is_err()
        );
    }

    /// Byte-for-byte sample of `manifest.json` produced by the
    /// `export-site-pack-bundle` dev bin (committed payload under
    /// ace-installer/bundled-packs). This test pins the cross-repo contract:
    /// the engine loader must parse exactly what the exporter emits.
    const REAL_EXPORTER_MANIFEST: &str = r#"{
  "schema_version": 1,
  "build_date": "2026-09-02",
  "generated_at": "2026-09-02T15:56:25.903535800+00:00",
  "pack_db_format_version": "1.0",
  "packs": [
    {
      "id": "pack-adult-free",
      "name": "Adult Websites — Free",
      "description": "Blocks known adult content websites.",
      "requires_pro": false,
      "content_version": "2026.09.02",
      "sha256": "3b08eaffa5629e43fe72a24c966b8cb8114d4c3710329be423f9a8dd6944eb43",
      "size_bytes": 2834432,
      "entry_count": 182105,
      "built_at": "2026-09-02T00:00:00Z",
      "file": "pack-adult-free/pack.db"
    },
    {
      "id": "pack-gaming-free",
      "name": "Gaming Websites — Free",
      "description": "Blocks online gaming sites and services.",
      "requires_pro": false,
      "content_version": "2026.09.02",
      "sha256": "adcfa7fae10fe4c623cd836cdf64c997627a40c42af6e2234133c9aca0d9e69f",
      "size_bytes": 958464,
      "entry_count": 60849,
      "built_at": "2026-09-02T00:00:00Z",
      "file": "pack-gaming-free/pack.db"
    },
    {
      "id": "pack-social-free",
      "name": "Social Media Websites — Free",
      "description": "Blocks social media sites.",
      "requires_pro": false,
      "content_version": "2026.09.02",
      "sha256": "e4d77ef5d7d9bf5487ee54843491ef8e9d6484da7c0eef593c9d1c32aad7a3f8",
      "size_bytes": 1314816,
      "entry_count": 83271,
      "built_at": "2026-09-02T00:00:00Z",
      "file": "pack-social-free/pack.db"
    }
  ]
}"#;

    #[test]
    fn parses_manifest_exactly_as_the_exporter_emits_it() {
        let manifest: BundledManifest = serde_json::from_str(REAL_EXPORTER_MANIFEST)
            .expect("real exporter manifest must parse");
        assert_eq!(manifest.schema_version.into_inner(), 1);
        assert_eq!(manifest.pack_db_format_version.as_ref(), "1.0");
        assert_eq!(manifest.build_date, "2026-09-02");
        assert_eq!(manifest.packs.len(), 3);
        let ids: Vec<&str> = manifest.packs.iter().map(|pack| pack.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["pack-adult-free", "pack-gaming-free", "pack-social-free"]
        );
        let adult = &manifest.packs[0];
        assert_eq!(adult.content_version.as_ref(), "2026.09.02");
        assert!(!adult.requires_pro, "free packs must not require Pro");
        assert_eq!(adult.size_bytes, 2_834_432);
        assert_eq!(adult.entry_count, 182_105);
        assert_eq!(adult.file, "pack-adult-free/pack.db");
    }

    #[test]
    fn unsafe_manifest_file_path_is_rejected() {
        assert!(is_safe_relative_path("pack-adult-free/pack.db"));
        assert!(!is_safe_relative_path("../evil.db"));
        assert!(!is_safe_relative_path("C:/evil.db"));
        assert!(!is_safe_relative_path("/abs/pack.db"));
        assert!(!is_safe_relative_path("a\\b\\pack.db"));
        assert!(!is_safe_relative_path(""));
    }
}
