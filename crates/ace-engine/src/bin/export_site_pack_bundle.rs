//! `export-site-pack-bundle` — reproducible offline site-pack bundle exporter.
//!
//! Reads the canonical site-pack catalog (the id/name/tier/source registry,
//! default `ace-app/src/lib/data/site-packs.json`), builds the selected packs
//! into deterministic per-pack SQLite databases (`<out>/<id>/pack.db`), and
//! emits a versioned `manifest.json` with canonical IDs, content versions,
//! SHA-256 checksums, sizes and entry counts.
//!
//! The output directory is what the Windows installer packages under
//! `[INSTALLFOLDER]\bundled-packs` (files + `manifest.json`), and what the
//! engine's bundled-pack importer consumes at first run. One command therefore
//! drives both the installer payload and the engine's offline seed data, so no
//! duplicate handwritten pack definitions exist in the installer.
//!
//! Determinism contract:
//! - identical catalog, source overrides, `--build-date` and importer version
//!   produce byte-identical artifacts (pack_meta `built_at` is pinned to the
//!   build date instead of wall-clock time);
//! - unknown or duplicate catalog IDs are rejected;
//! - a selected pack whose content cannot be obtained (missing override file,
//!   failed download) fails the export loudly.
//!
//! Usage:
//! ```text
//! cargo run --features dev-tools --bin export-site-pack-bundle -- \
//!     --catalog ../ace-app/src/lib/data/site-packs.json \
//!     --out-dir ../ace-installer/bundled-packs \
//!     --packs pack-adult-free,pack-social-free,pack-gaming-free \
//!     --build-date 2026-09-02 \
//!     [--source-overrides path/to/overrides.json]
//! ```
//!
//! `--source-overrides` maps a catalog source URL to a local file path, which
//! allows building from pinned local snapshots (e.g. the ace-hosts output for
//! `pack-adult-pro`) without network access.

use ace_engine::core::importer::{
    Importer, SitePackCreationStatus, SitePackFormat, SitePackId, SitePackSource, SitePackSourceId,
    SitePackSourceType,
};
use ace_engine::core::{
    BundleSchemaVersion, PackDbFormatVersion, RemoteSitePackId, Sha256Digest, SitePackVersion,
    network_block::site_pack::site_pack_db::SitePackDB,
};
use ace_engine::schema_site_packs::pack_meta;
use ace_engine::service::bundled_site_packs::{
    BundledManifest as BundleManifest, BundledPackEntry as ManifestPackEntry,
    SUPPORTED_MANIFEST_SCHEMA_VERSION,
};
use diesel::prelude::*;
use eyre::{Context, Result, eyre};
use futures_util::StreamExt;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

/// Format version of the per-pack content database written by the importer
/// (`migrations_site_packs/0001_create_site_pack`).
pub const PACK_DB_FORMAT_VERSION: &str = "1.0";

const SOURCE_DATE_EPOCH_ENV: &str = "SOURCE_DATE_EPOCH";

// ── Catalog model (ace-app/src/lib/data/site-packs.json) ──────────────

#[derive(Debug, Clone, Deserialize)]
struct CatalogEntry {
    id: String,
    name: String,
    description: Option<String>,
    requires_pro: bool,
    #[serde(default)]
    sources: Vec<CatalogSource>,
}

#[derive(Debug, Clone, Deserialize)]
struct CatalogSource {
    source: String,
    #[serde(default = "default_source_type")]
    source_type: String,
    #[serde(default = "default_format")]
    format: String,
}

fn default_source_type() -> String {
    "url".to_string()
}

fn default_format() -> String {
    "standard".to_string()
}

/// Normalizes a catalog source-type token (`url` | `file`) to the engine
/// enum. The catalog registers URL sources today; `file` is accepted so a
/// source can point at a local path directly without an override map.
fn parse_source_type(token: &str) -> Result<SitePackSourceType> {
    match token.trim().to_ascii_lowercase().as_str() {
        "url" => Ok(SitePackSourceType::Url),
        "file" => Ok(SitePackSourceType::File),
        other => Err(eyre!(
            "unknown source_type {other:?} in catalog (expected url|file)"
        )),
    }
}

/// Normalizes a catalog format token (`standard` | `wildcarddomains` |
/// `wildcard` | `auto`) to the engine enum. The catalog historically used
/// `wildcarddomains`; engine code uses `wildcard`. Both are accepted so a
/// single canonical file stays authoritative.
fn parse_format(token: &str) -> Result<SitePackFormat> {
    match token.trim().to_ascii_lowercase().as_str() {
        "standard" => Ok(SitePackFormat::Standard),
        "wildcard" | "wildcarddomains" | "wildcard_domains" => Ok(SitePackFormat::WildCardDomains),
        "auto" => Ok(SitePackFormat::Auto),
        other => Err(eyre!(
            "unknown source format {other:?} in catalog (expected standard|wildcarddomains|auto)"
        )),
    }
}

// ── CLI ───────────────────────────────────────────────────────────────

struct Args {
    catalog: PathBuf,
    out_dir: PathBuf,
    packs: Option<BTreeSet<String>>,
    build_date: String,
    source_overrides: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut catalog = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/ace-app/src/lib/data/site-packs.json");
    let mut out_dir = PathBuf::from("bundled-packs");
    let mut packs: Option<BTreeSet<String>> = None;
    let mut build_date: Option<String> = None;
    let mut source_overrides: Option<PathBuf> = None;

    let mut raw = std::env::args().skip(1);
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--catalog" => {
                catalog = PathBuf::from(
                    raw.next()
                        .ok_or_else(|| eyre!("--catalog requires a path"))?,
                );
            }
            "--out-dir" => {
                out_dir = PathBuf::from(
                    raw.next()
                        .ok_or_else(|| eyre!("--out-dir requires a path"))?,
                );
            }
            "--packs" => {
                let list = raw
                    .next()
                    .ok_or_else(|| eyre!("--packs requires a comma-separated list"))?;
                packs = Some(
                    list.split(',')
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .collect(),
                );
            }
            "--build-date" => {
                build_date = Some(
                    raw.next()
                        .ok_or_else(|| eyre!("--build-date requires YYYY-MM-DD"))?,
                );
            }
            "--source-overrides" => {
                source_overrides =
                    Some(PathBuf::from(raw.next().ok_or_else(|| {
                        eyre!("--source-overrides requires a path")
                    })?));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(eyre!("unknown argument {other:?}")),
        }
    }

    let build_date = match build_date {
        Some(date) => date,
        None => {
            // SOURCE_DATE_EPOCH keeps builds reproducible across machines.
            if let Ok(epoch) = std::env::var(SOURCE_DATE_EPOCH_ENV) {
                if let Ok(seconds) = epoch.trim().parse::<i64>() {
                    let date = chrono::DateTime::from_timestamp(seconds, 0)
                        .ok_or_else(|| eyre!("invalid {SOURCE_DATE_EPOCH_ENV}"))?;
                    date.format("%Y-%m-%d").to_string()
                } else {
                    return Err(eyre!("invalid {SOURCE_DATE_EPOCH_ENV}: {epoch:?}"));
                }
            } else {
                chrono::Utc::now().format("%Y-%m-%d").to_string()
            }
        }
    };

    validate_date(&build_date)?;

    Ok(Args {
        catalog,
        out_dir,
        packs,
        build_date,
        source_overrides,
    })
}

fn validate_date(date: &str) -> Result<()> {
    if date.len() != 10
        || date.as_bytes()[4] != b'-'
        || date.as_bytes()[7] != b'-'
        || !date.as_bytes()[..4].iter().all(u8::is_ascii_digit)
        || !date.as_bytes()[5..7].iter().all(u8::is_ascii_digit)
        || !date.as_bytes()[8..10].iter().all(u8::is_ascii_digit)
    {
        return Err(eyre!(
            "invalid --build-date {date:?}: expected YYYY-MM-DD with numeric fields"
        ));
    }
    Ok(())
}

fn print_help() {
    println!(
        "export-site-pack-bundle — reproducible offline site-pack bundle exporter\n\
         \n\
         USAGE:\n\
         \x20 export-site-pack-bundle [OPTIONS]\n\
         \n\
         OPTIONS:\n\
         \x20 --catalog <path>            Canonical pack catalog JSON (default apps/ace-app/src/lib/data/site-packs.json)\n\
         \x20 --out-dir <path>            Output directory for <id>/pack.db + manifest.json (default ./bundled-packs)\n\
         \x20 --packs <id1,id2,...>        Restrict export to these catalog ids (default: all)\n\
         \x20 --build-date <YYYY-MM-DD>   Content date stamped into pack_meta and content_version (default: today or SOURCE_DATE_EPOCH)\n\
         \x20 --source-overrides <path>   JSON mapping catalog source URLs to local file paths\n\
         \x20 -h, --help                   Print this help"
    );
}

// ── Source overrides ──────────────────────────────────────────────────

fn load_overrides(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read source overrides at {}", path.display()))?;
    serde_json::from_str(&content)
        .wrap_err_with(|| format!("invalid source overrides JSON at {}", path.display()))
}

// ── Deflate measurement ───────────────────────────────────────────────

/// Raw deflate size of `bytes` (level 9) — a reproducible proxy for MSZIP
/// cabinet compression. The installer's real per-file cost is measured with
/// LZX at build time (see the offline-bundling report); deflate is reported
/// here because it is deterministic across platforms and needs no external
/// tooling.
fn deflate_size(bytes: &[u8]) -> usize {
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("deflate write cannot fail");
    encoder.finish().expect("deflate finish cannot fail").len()
}

// ── Catalog validation ────────────────────────────────────────────────

fn load_catalog(path: &Path) -> Result<Vec<CatalogEntry>> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read catalog at {}", path.display()))?;
    let entries: Vec<CatalogEntry> = serde_json::from_str(&content)
        .wrap_err_with(|| format!("invalid catalog JSON at {}", path.display()))?;

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in &entries {
        if !seen.insert(entry.id.clone()) {
            return Err(eyre!(
                "duplicate pack id {:?} in catalog — refusing to export",
                entry.id
            ));
        }
        if entry.sources.is_empty() {
            return Err(eyre!(
                "pack {:?} has no sources in the catalog — refusing to export",
                entry.id
            ));
        }
        for source in &entry.sources {
            if source.source.is_empty() {
                return Err(eyre!("pack {:?} has an empty source URL", entry.id));
            }
            parse_format(&source.format).wrap_err_with(|| format!("pack {:?}", entry.id))?;
        }
    }
    Ok(entries)
}

// ── Per-pack build ────────────────────────────────────────────────────

/// Builds one pack into a fresh database at `db_path` (as `pack.db`),
/// importing its sources (URLs unless overridden to local files), then pins
/// `pack_meta.built_at`/`name` for determinism.
async fn build_pack_db(
    entry: &CatalogEntry,
    sources: &[CatalogSource],
    overrides: &BTreeMap<String, String>,
    db_path: &Path,
    build_date: &str,
    temp_dir: &Path,
) -> Result<u64> {
    if db_path.exists() {
        std::fs::remove_file(db_path)
            .wrap_err_with(|| format!("failed to clean existing db at {}", db_path.display()))?;
    }

    let db = SitePackDB::from(db_path)
        .wrap_err_with(|| format!("failed to create pack database at {}", db_path.display()))?;

    let site_pack_id = SitePackId::try_new(entry.id.clone())
        .map_err(|error| eyre!("Invalid catalog site-pack ID: {error}"))?;
    let import_sources: Vec<SitePackSource> = sources
        .iter()
        .map(|source| {
            let format =
                parse_format(&source.format).expect("format validated during catalog load");
            let source_type = parse_source_type(&source.source_type)
                .expect("source_type validated during catalog load");
            let (resolved_type, source_value) = match source_type {
                SitePackSourceType::Url => match overrides.get(&source.source) {
                    Some(local_path) => {
                        if !Path::new(local_path).is_file() {
                            panic!(
                                "source override file for {} does not exist: {}",
                                entry.id, local_path
                            );
                        }
                        (SitePackSourceType::File, local_path.clone())
                    }
                    None => (SitePackSourceType::Url, source.source.clone()),
                },
                SitePackSourceType::File => (SitePackSourceType::File, source.source.clone()),
            };
            SitePackSource {
                id: SitePackSourceId::from(Uuid::new_v4()),
                site_pack_id: site_pack_id.clone(),
                source: source_value,
                source_type: resolved_type,
                format,
            }
        })
        .collect();

    let mut stream =
        Importer::import_sources(&db, import_sources, temp_dir.to_path_buf(), site_pack_id);

    let mut failed: Option<String> = None;
    while let Some(status) = stream.next().await {
        match status {
            SitePackCreationStatus::Failed { error, .. } => failed = Some(error),
            SitePackCreationStatus::Completed { .. } => {}
            _ => {}
        }
    }
    if let Some(error) = failed {
        return Err(eyre!("import of pack {} failed: {error}", entry.id));
    }

    // Pin deterministic metadata on the pack_meta row (the importer stamps
    // wall-clock `built_at`; reproducibility requires a fixed value).
    let built_at = format!("{build_date}T00:00:00Z");
    {
        let connection = &mut db
            .get_db_connection()
            .wrap_err("failed to open pack database connection")?;
        diesel::update(pack_meta::table)
            .set((
                pack_meta::built_at.eq(&built_at),
                pack_meta::name.eq(Some(entry.name.clone())),
            ))
            .execute(connection)
            .wrap_err_with(|| format!("failed to pin pack_meta for {}", entry.id))?;

        // Diesel's migration bookkeeping table stamps each applied migration
        // with SQLite `CURRENT_TIMESTAMP` (wall clock, second precision),
        // which would make the artifact bytes differ run to run. The version
        // column is all Diesel ever reads, so pin `run_on` to the build date.
        diesel::sql_query(
            "UPDATE __diesel_schema_migrations SET run_on = ? WHERE run_on IS NOT NULL",
        )
        .bind::<diesel::sql_types::Text, _>(format!("{build_date} 00:00:00"))
        .execute(connection)
        .wrap_err("failed to pin migration timestamps for determinism")?;
    }

    let entry_count = db
        .count_entries()
        .wrap_err("failed to count pack entries")?;
    Ok(entry_count as u64)
}

// ── Main ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> ExitCode {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    match run().await {
        Ok(summary) => {
            println!("\nExport complete.");
            println!("  manifest: {}", summary.manifest_path.display());
            println!(
                "  packs:    {} ({} skipped)",
                summary.exported, summary.skipped
            );
            println!("  raw bytes:      {}", summary.total_raw);
            println!("  deflate bytes:  {}", summary.total_deflate);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("export failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

struct RunSummary {
    manifest_path: PathBuf,
    exported: usize,
    skipped: usize,
    total_raw: u64,
    total_deflate: u64,
}

async fn run() -> Result<RunSummary> {
    let args = parse_args()?;
    let catalog_path = args.catalog.clone();
    let out_dir = args.out_dir.clone();

    let entries = load_catalog(&catalog_path)?;

    // Resolve selection + validate unknown ids before any work.
    let by_id: BTreeMap<&str, &CatalogEntry> = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let selected_ids: Vec<&CatalogEntry> = match &args.packs {
        Some(requested) => {
            let mut selected = Vec::new();
            for id in requested {
                let entry = by_id.get(id.as_str()).ok_or_else(|| {
                    eyre!("requested pack {id:?} is not in the catalog — refusing to export")
                })?;
                selected.push(*entry);
            }
            selected
        }
        None => entries.iter().collect(),
    };

    let overrides = match &args.source_overrides {
        Some(path) => load_overrides(path)?,
        None => BTreeMap::new(),
    };

    std::fs::create_dir_all(&out_dir)
        .wrap_err_with(|| format!("failed to create output dir {}", out_dir.display()))?;

    // Unique temp dir for source downloads (cleaned at the end).
    let unique = Uuid::new_v4();
    let temp_dir = std::env::temp_dir().join(format!("ace-site-pack-export-{unique}"));
    std::fs::create_dir_all(&temp_dir).wrap_err("failed to create export temp dir")?;

    let mut manifest_packs: Vec<ManifestPackEntry> = Vec::new();
    let mut total_raw: u64 = 0;
    let mut total_deflate: u64 = 0;
    let mut failed_ids: Vec<String> = Vec::new();

    for entry in &selected_ids {
        info!("Exporting pack {} ({})", entry.id, entry.name);

        // Validate all override files up front so "missing content" is a
        // loud, complete error.
        let mut missing_override: Option<String> = None;
        for source in &entry.sources {
            if let Some(local_path) = overrides.get(&source.source)
                && !Path::new(local_path).is_file()
            {
                missing_override = Some(format!(
                    "override for {} points at missing file {}",
                    entry.id, local_path
                ));
                break;
            }
        }
        if let Some(message) = missing_override {
            eprintln!("  ERROR: {message}");
            failed_ids.push(entry.id.clone());
            continue;
        }

        let pack_dir = out_dir.join(&entry.id);
        std::fs::create_dir_all(&pack_dir)
            .wrap_err_with(|| format!("failed to create pack dir {}", pack_dir.display()))?;
        let db_path = pack_dir.join("pack.db");

        let entry_count = match build_pack_db(
            entry,
            &entry.sources,
            &overrides,
            &db_path,
            &args.build_date,
            &temp_dir,
        )
        .await
        {
            Ok(count) => count,
            Err(error) => {
                eprintln!("  ERROR exporting {}: {error:#}", entry.id);
                failed_ids.push(entry.id.clone());
                let _ = std::fs::remove_dir_all(&pack_dir);
                continue;
            }
        };

        let artifact_bytes = std::fs::read(&db_path)
            .wrap_err_with(|| format!("failed to read built artifact {}", db_path.display()))?;
        let raw_size = artifact_bytes.len() as u64;
        let deflated = deflate_size(&artifact_bytes) as u64;
        total_raw += raw_size;
        total_deflate += deflated;

        let remote_id = RemoteSitePackId::try_new(entry.id.clone())
            .map_err(|error| eyre!("invalid catalog site-pack ID '{}': {error}", entry.id))?;
        let content_version = SitePackVersion::try_new(args.build_date.replace('-', "."))
            .map_err(|error| eyre!("invalid bundle content version: {error}"))?;
        let digest = Sha256Digest::from_bytes(&artifact_bytes);
        let file_rel = format!("{}/pack.db", entry.id);
        manifest_packs.push(ManifestPackEntry {
            id: remote_id,
            name: entry.name.clone(),
            description: entry.description.clone(),
            requires_pro: entry.requires_pro,
            content_version,
            sha256: digest.clone(),
            size_bytes: raw_size,
            entry_count,
            built_at: Some(format!("{}T00:00:00Z", args.build_date)),
            file: file_rel,
        });

        info!(
            "  {}: {} entries, raw {} bytes, deflate {} bytes, sha256 {}",
            entry.id,
            entry_count,
            raw_size,
            deflated,
            digest.as_ref().chars().take(12).collect::<String>()
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);

    if !failed_ids.is_empty() {
        return Err(eyre!(
            "export rejected missing/unobtainable content for packs: {}",
            failed_ids.join(", ")
        ));
    }

    let manifest = BundleManifest {
        schema_version: BundleSchemaVersion::try_new(SUPPORTED_MANIFEST_SCHEMA_VERSION)
            .expect("supported manifest schema version is non-zero"),
        build_date: args.build_date.clone(),
        generated_at: Some(chrono::Utc::now().to_rfc3339()),
        pack_db_format_version: PackDbFormatVersion::try_new(PACK_DB_FORMAT_VERSION.to_string())
            .expect("pack DB format version is non-empty"),
        packs: manifest_packs,
    };
    let manifest_path = out_dir.join("manifest.json");
    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest).wrap_err("failed to serialize manifest")?;
    std::fs::write(&manifest_path, &manifest_bytes)
        .wrap_err_with(|| format!("failed to write manifest at {}", manifest_path.display()))?;

    Ok(RunSummary {
        manifest_path,
        exported: manifest.packs.len(),
        skipped: selected_ids.len() - manifest.packs.len(),
        total_raw,
        total_deflate,
    })
}
