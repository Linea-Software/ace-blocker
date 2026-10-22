pub mod importer;
pub mod site_pack_db;

use crate::core::common::domain_types::{RemoteSitePackId, SitePackVersion};
use crate::core::network_block::site_pack::site_pack_db::SitePackDB;
use crate::core::network_block::{BlockingResult, UriHash};
pub use importer::{
    Importer, SitePackCreationStatus, SitePackFormat, SitePackId, SitePackImportError,
    SitePackImportStream, SitePackSource,
};

use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tracing::warn;
use zip::ZipArchive;

/// Name of the SQLite database file inside a site pack's directory.
const PACK_DB_FILE: &str = "pack.db";

#[derive(Debug)]
pub struct SitePack {
    id: SitePackId,
    remote_id: Option<RemoteSitePackId>,
    name: String,
    description: Option<String>,
    requires_pro: bool,
    version: Option<SitePackVersion>,
    icon: Option<String>,
    db_path: PathBuf,
    // None when the pack has never been downloaded/created yet (remote packs).
    db: Option<SitePackDB>,
    // Bounded, lazy cache shared by request and tunnel policy lookups.
    // It exists because calling the DB for each request may be
    membership_cache: moka::sync::Cache<UriHash, BlockingResult>,
    sources: Vec<SitePackSource>,
    last_updated_at: Option<chrono::DateTime<chrono::Local>>,
}

impl SitePack {
    pub fn get_id(&self) -> &SitePackId {
        &self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn requires_pro(&self) -> bool {
        self.requires_pro
    }

    pub fn is_remote(&self) -> bool {
        self.remote_id.is_some()
    }

    pub fn get_remote_id(&self) -> Option<&RemoteSitePackId> {
        self.remote_id.as_ref()
    }

    pub fn get_db_path(&self) -> &PathBuf {
        &self.db_path
    }

    pub fn get_sources(&self) -> &Vec<SitePackSource> {
        &self.sources
    }

    pub fn get_last_updated_at(&self) -> Option<chrono::DateTime<chrono::Local>> {
        self.last_updated_at
    }

    pub fn get_version(&self) -> Option<&SitePackVersion> {
        self.version.as_ref()
    }

    pub fn get_icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    pub fn set_version(&mut self, version: SitePackVersion) {
        self.version = Some(version);
    }

    // Constructor with explicit fields; a builder would add boilerplate for
    // no current benefit.
    #[expect(clippy::too_many_arguments)]
    pub fn from(
        id: SitePackId,
        remote_id: Option<RemoteSitePackId>,
        name: String,
        description: Option<String>,
        requires_pro: bool,
        version: Option<SitePackVersion>,
        icon: Option<String>,
        db_path: &Path,
        sources: Vec<SitePackSource>,
        last_updated_at: Option<chrono::DateTime<chrono::Local>>,
    ) -> SitePack {
        // Local packs get an empty database created eagerly. Remote packs
        // only get one once they have been downloaded for the first time, so
        // they stay unsynced until then (see `is_synced`).
        let db = if db_path.exists() || remote_id.is_none() {
            SitePackDB::from(&db_path.join(PACK_DB_FILE)).ok()
        } else {
            None
        };

        SitePack {
            id,
            remote_id,
            name,
            description,
            requires_pro,
            version,
            icon,
            db_path: db_path.to_path_buf(),
            db,
            membership_cache: moka::sync::Cache::builder()
                .max_capacity(crate::core::common::constants::RESULTS_CACHE_MAX_SIZE)
                .build(),
            sources,
            last_updated_at,
        }
    }

    fn pack_db_path(&self) -> PathBuf {
        self.db_path.join(PACK_DB_FILE)
    }

    pub fn blocking_result(&self, host: &str) -> BlockingResult {
        let Some(db) = self.db.as_ref() else {
            return BlockingResult::NotBlocked;
        };

        let uri_hash = site_pack_db::hash_host(host);
        // Cache hits and misses; coalesce concurrent queries for the same host.
        // Failed queries are not cached, so a transient DB error can recover.

        self.membership_cache
            .try_get_with(uri_hash.clone(), || db.blocking_result(&uri_hash))
            .unwrap_or(BlockingResult::NotBlocked)
    }

    pub fn get_entry_count(&self) -> u64 {
        self.db
            .as_ref()
            .map(|db| db.get_entry_count().unwrap_or(0) as u64)
            .unwrap_or(0)
    }

    /// A site pack is synced once its database is loaded. Remote packs have no
    /// database until they are downloaded for the first time, so they start
    /// unsynced; local packs are created empty and are therefore always synced.
    pub fn is_synced(&self) -> bool {
        self.db.is_some()
    }

    pub fn close_db(&mut self) {
        self.db = None;
        self.membership_cache.invalidate_all();
    }

    /// Reopens the database from disk.
    pub fn reload_db(&mut self) -> Result<(), String> {
        let db_result = SitePackDB::from(&self.pack_db_path());

        match db_result {
            Ok(db) => {
                self.db = Some(db);
                self.membership_cache.invalidate_all();
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Restores the pack from a zip archive containing the SQLite database
    /// file (error.g. the archives produced by the `gen-site-packs` tool).
    ///
    /// The archive is first staged into a sibling directory and validated;
    /// only then is the live directory swapped atomically. A failed,
    /// cancelled, or malicious restore can never truncate or empty the live
    /// pack database — the previous pack stays in place (and stays usable:
    /// the live DB is only closed immediately before the swap and re-opened
    /// on every failure path).
    pub fn restore_from_backup(
        &mut self,
        archive_path: PathBuf,
    ) -> Result<(), SitePackImportError> {
        let parent = self
            .db_path
            .parent()
            .ok_or_else(|| SitePackImportError::BackupFailure {
                message: "site pack has no parent directory".to_string(),
            })?
            .to_path_buf();

        let unique = uuid::Uuid::new_v4();
        let staging_dir = parent.join(format!(".{}.staging-{}", self.id, unique));

        // Stage + validate while the live DB is still open, so a rejected
        // archive never disturbs the running pack. `SitePackDB::from` alone
        // is not enough — it would create a fresh empty schema for a
        // missing/corrupt file.
        let staged_db_path = match stage_backup_archive(&archive_path, &staging_dir) {
            Ok(path) => path,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(error);
            }
        };
        let staged_db = match SitePackDB::from(&staged_db_path) {
            Ok(db) => db,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(backup_error(error));
            }
        };
        let entry_count = staged_db.count_entries();
        drop(staged_db);
        match entry_count {
            Ok(0) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(SitePackImportError::BackupFailure {
                    message: "backup archive contains an empty site list database".to_string(),
                });
            }
            Ok(_) => {}
            Err(error) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(backup_error(error));
            }
        }

        let result = self.install_staged_db(&staging_dir);
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&staging_dir);
        }
        result
    }

    /// Activates a completed database built beside the live pack.
    /// The caller removes the staging directory on failure or cancellation.
    pub(crate) fn install_staged_db(
        &mut self,
        staging_dir: &Path,
    ) -> Result<(), SitePackImportError> {
        let parent = self
            .db_path
            .parent()
            .ok_or_else(|| backup_error("missing pack parent"))?;
        let old_dir = parent.join(format!(".{}.old-{}", self.id, uuid::Uuid::new_v4()));

        // Rollback-capable swap: live dir -> old dir, staging dir -> live dir. On any
        // failure the old dir is renamed back so the previous pack survives.
        self.close_db();
        let live_exists = self.db_path.exists();
        if live_exists && let Err(error) = std::fs::rename(&self.db_path, &old_dir) {
            let _ = self.reload_db();
            return Err(backup_error(error));
        }
        if let Err(error) = std::fs::rename(&staging_dir, &self.db_path) {
            if live_exists {
                let _ = std::fs::rename(&old_dir, &self.db_path);
            }
            let _ = self.reload_db();
            return Err(backup_error(error));
        }

        self.reload_db().map_err(|error| {
            // Roll the swap back: the new database did not open.
            let _ = std::fs::remove_dir_all(&self.db_path);
            if live_exists {
                let _ = std::fs::rename(&old_dir, &self.db_path);
            }
            let _ = self.reload_db();
            SitePackImportError::BackupFailure { message: error }
        })?;

        // Best-effort cleanup of the superseded directory. `NotFound`
        // (os error 2) is an expected outcome of the idempotent swap — the
        // `.old-*` dir only exists when a live dir was renamed aside — so it
        // is handled silently; genuine failures still warn below.
        if let Err(error) = crate::core::utils::remove_superseded_dir(&old_dir) {
            warn!(
                "failed to delete superseded site list database at {}: {}",
                old_dir.display(),
                error
            );
        }

        self.set_last_updated_at(chrono::Local::now());

        Ok(())
    }

    pub fn add_source(&mut self, source: SitePackSource) {
        self.sources.push(source);
    }

    pub fn remove_source(&mut self, source_id: &importer::SitePackSourceId) {
        self.sources.retain(|s| s.id != *source_id);
    }

    pub fn set_last_updated_at(&mut self, last_updated_at: chrono::DateTime<chrono::Local>) {
        self.last_updated_at = Some(last_updated_at);
    }
}

fn backup_error(error: impl std::fmt::Display) -> SitePackImportError {
    SitePackImportError::BackupFailure {
        message: error.to_string(),
    }
}

/// Cap on a single site-pack backup archive.
const MAX_BACKUP_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Validates the entries of a site-pack backup archive and extracts the
/// SQLite database file (a single `*.db` file at the archive root) into
/// `staging_dir` as `pack.db`.
///
/// Entries with absolute paths, `..` segments, drive prefixes, or
/// backslashes are rejected outright (defense in depth against malicious
/// archives — see also `file_download::extract_zip_safely`), and the
/// extracted size is capped. Returns the path of the staged database file.
pub(crate) fn stage_backup_archive(
    archive_path: &Path,
    staging_dir: &Path,
) -> Result<PathBuf, SitePackImportError> {
    let archive_file = File::open(archive_path).map_err(backup_error)?;
    let mut archive = ZipArchive::new(archive_file).map_err(backup_error)?;

    std::fs::create_dir_all(staging_dir).map_err(backup_error)?;

    let mut staged_db_path: Option<PathBuf> = None;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(backup_error)?;
        let raw_name = entry.name().map_err(backup_error)?.to_string();

        // Strict raw-name validation. The zip crate's own sanitizer
        // *normalizes* absolute paths and `..` segments instead of rejecting
        // them, so check the raw name first.
        if raw_name.contains('\0')
            || raw_name.starts_with('/')
            || raw_name.starts_with('\\')
            || raw_name.contains(':')
            || raw_name.contains('\\')
            || raw_name.split('/').any(|segment| segment == "..")
        {
            return Err(SitePackImportError::BackupFailure {
                message: format!("backup archive contains an unsafe entry: {raw_name:?}"),
            });
        }

        // Only a single `*.db` file at the archive root is consumed; anything
        // else (READMEs, `__MACOSX` junk) is ignored.
        if !raw_name.contains('/') && raw_name.ends_with(".db") {
            if staged_db_path.is_some() {
                return Err(SitePackImportError::BackupFailure {
                    message: "backup archive contains multiple database files".to_string(),
                });
            }
            if entry.size() > MAX_BACKUP_ARCHIVE_BYTES {
                return Err(SitePackImportError::BackupFailure {
                    message: format!(
                        "backup archive database exceeds the {MAX_BACKUP_ARCHIVE_BYTES}-byte limit"
                    ),
                });
            }

            let target = staging_dir.join(PACK_DB_FILE);
            let mut db_file = File::create(&target).map_err(backup_error)?;
            std::io::copy(&mut entry, &mut db_file).map_err(backup_error)?;
            staged_db_path = Some(target);
        }
    }

    staged_db_path.ok_or_else(|| SitePackImportError::BackupFailure {
        message: "no database file found in the backup archive".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network_block::site_pack::site_pack_db::{NewPackMetaModel, hash_host};
    use chrono::Utc;

    fn make_valid_pack_db(db_path: &Path, hosts: &[&str]) {
        let db = SitePackDB::from(db_path).expect("create pack db");
        let hashes: Vec<UriHash> = hosts.iter().map(|host| hash_host(host)).collect();
        db.insert_entries(&hashes).unwrap();
        db.insert_pack_meta(NewPackMetaModel {
            id: "test".to_string(),
            format_version: "1.0".to_string(),
            name: None,
            entry_count: hosts.len() as i32,
            built_at: Utc::now().to_rfc3339(),
        })
        .unwrap();
    }

    fn pack_contains(pack: &SitePack, host: &str) -> bool {
        pack.blocking_result(host).is_blocked()
    }

    fn zip_file_bytes(name: &str, contents: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let cursor = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        writer
            .start_file(
                name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(contents).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn test_pack(db_path: &Path) -> SitePack {
        SitePack::from(
            SitePackId::new("test-pack".to_string()),
            None,
            "Test".to_string(),
            None,
            false,
            None,
            None,
            db_path,
            vec![],
            None,
        )
    }

    #[test]
    fn membership_is_lazy_and_invalidated_on_reload_and_close() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("pack.db");
        make_valid_pack_db(&db_path, &["old.example"]);
        let mut pack = test_pack(temp.path());
        let db = SitePackDB::from(&db_path).unwrap();
        assert!(!pack_contains(&pack, "new.example"));
        db.insert_entries(&[hash_host("new.example")]).unwrap();
        db.insert_entries(&[hash_host("uncached.example")]).unwrap();
        assert!(pack_contains(&pack, "old.example"));
        assert!(
            pack_contains(&pack, "uncached.example"),
            "uncached membership must query SQLite lazily"
        );
        assert!(
            !pack_contains(&pack, "new.example"),
            "negative results must be cached until reload"
        );
        pack.reload_db().unwrap();
        assert!(pack_contains(&pack, "new.example"));

        pack.close_db();
        assert!(!pack_contains(&pack, "old.example"));
        pack.membership_cache.run_pending_tasks();
        assert_eq!(pack.membership_cache.entry_count(), 0);
        pack.reload_db().unwrap();
        assert!(pack_contains(&pack, "old.example"));
        assert!(pack_contains(&pack, "new.example"));
    }

    #[test]
    fn stage_backup_archive_extracts_a_root_db_entry() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("pack.db");
        make_valid_pack_db(&db_path, &["example.com"]);
        let db_bytes = std::fs::read(&db_path).unwrap();
        let archive = temp.path().join("backup.zip");
        std::fs::write(&archive, zip_file_bytes("pack.db", &db_bytes)).unwrap();

        let staging = temp.path().join("staging");
        let staged = stage_backup_archive(&archive, &staging).unwrap();
        assert_eq!(staged, staging.join("pack.db"));

        // The staged database must be openable and non-empty.
        let db = SitePackDB::from(&staged).unwrap();
        assert_eq!(db.count_entries().unwrap(), 1);
        assert!(
            db.blocking_result(&hash_host("example.com"))
                .unwrap()
                .is_blocked()
        );
    }

    #[test]
    fn stage_backup_archive_rejects_traversal_entries() {
        let temp = tempfile::tempdir().unwrap();
        for evil_name in ["../evil.db", "sub/../../evil.db", "C:/evil.db", "\\evil.db"] {
            let archive = temp.path().join("evil.zip");
            std::fs::write(&archive, zip_file_bytes(evil_name, b"x")).unwrap();
            let staging = temp.path().join("staging");
            let result = stage_backup_archive(&archive, &staging);
            assert!(result.is_err(), "entry {evil_name:?} must be rejected");
            assert!(
                !temp.path().parent().unwrap().join("evil.db").exists(),
                "no file may escape the staging root"
            );
        }
    }

    #[test]
    fn stage_backup_archive_rejects_multiple_db_entries() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("multi.zip");
        let cursor = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for name in ["a.db", "b.db"] {
            writer.start_file(name, options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        std::fs::write(&archive, writer.finish().unwrap().into_inner()).unwrap();

        let staging = temp.path().join("staging");
        assert!(stage_backup_archive(&archive, &staging).is_err());
    }

    #[test]
    fn restore_from_backup_failure_preserves_the_previous_pack() {
        let temp = tempfile::tempdir().unwrap();
        let pack_dir = temp.path().join("pack");
        let db_path = pack_dir.join("pack.db");
        make_valid_pack_db(&db_path, &["old.example.com"]);

        let mut pack = test_pack(&pack_dir);
        assert!(pack_contains(&pack, "old.example.com"));

        // (a) An archive with no database entry must fail cleanly.
        let no_db = temp.path().join("no-db.zip");
        std::fs::write(&no_db, zip_file_bytes("readme.txt", b"not a db")).unwrap();
        assert!(pack.restore_from_backup(no_db).is_err());
        assert!(
            pack_contains(&pack, "old.example.com"),
            "previous pack must survive a missing-db archive"
        );

        // (b) An archive whose database is corrupt must fail cleanly.
        let corrupt = temp.path().join("corrupt.zip");
        std::fs::write(&corrupt, zip_file_bytes("pack.db", b"not a sqlite file")).unwrap();
        assert!(pack.restore_from_backup(corrupt).is_err());
        assert!(
            pack_contains(&pack, "old.example.com"),
            "previous pack must survive a corrupt archive"
        );

        // (c) An archive with an empty database must fail cleanly.
        let empty_db = temp.path().join("empty.db");
        SitePackDB::from(&empty_db).expect("create empty db");
        let empty = temp.path().join("empty.zip");
        std::fs::write(
            &empty,
            zip_file_bytes("pack.db", &std::fs::read(&empty_db).unwrap()),
        )
        .unwrap();
        assert!(pack.restore_from_backup(empty).is_err());
        assert!(
            pack_contains(&pack, "old.example.com"),
            "previous pack must survive an empty archive"
        );
    }

    #[test]
    fn restore_from_backup_swaps_in_the_new_database() {
        let temp = tempfile::tempdir().unwrap();
        let pack_dir = temp.path().join("pack");
        let db_path = pack_dir.join("pack.db");
        make_valid_pack_db(&db_path, &["old.example.com"]);

        let mut pack = test_pack(&pack_dir);
        assert!(pack_contains(&pack, "old.example.com"));

        let new_db = temp.path().join("new.db");
        make_valid_pack_db(&new_db, &["new.example.com"]);
        let archive = temp.path().join("new.zip");
        std::fs::write(
            &archive,
            zip_file_bytes("pack.db", &std::fs::read(&new_db).unwrap()),
        )
        .unwrap();

        pack.restore_from_backup(archive).unwrap();
        assert!(pack_contains(&pack, "new.example.com"));
        assert!(!pack_contains(&pack, "old.example.com"));

        // No staging/old litter may remain.
        let leftovers: Vec<String> = std::fs::read_dir(temp.path())
            .unwrap()
            .filter_map(|error| error.ok())
            .map(|error| error.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains("staging") || name.contains(".old-"))
            .collect();
        assert!(leftovers.is_empty(), "staging/old dirs must be cleaned up");
    }
}
