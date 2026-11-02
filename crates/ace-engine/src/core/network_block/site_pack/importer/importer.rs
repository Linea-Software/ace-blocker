//! Site pack importer.
//!
//! Imports domain blocklists (hosts, wildcard, adblock) into a [`SitePackDB`]:
//! every domain is hashed with XXH3-64 and inserted into `entries.host_hash`
//! in batches of [`INSERT_BATCH_SIZE`].

use super::stream::{SitePackCreationStatus, SitePackImportPhase, SitePackImportStream};
use super::{SitePackFormat, SitePackId, SitePackImportError, SitePackSource, SitePackSourceType};
use crate::core::common::percentage::Percentage;
use crate::core::network_block::site_pack::site_pack_db::{
    NewPackMetaModel, SitePackDB, hash_host,
};
use crate::core::network_block::uri_hash::UriHash;

use ace_domain_blocklist::{DomainLineParsingResult, DomainListFormatKind, detect_format};
use async_stream::stream;
use chrono::Utc;
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn};

use crate::core::utils::file_download;

/// Number of hashes accumulated before flushing to SQLite (which uses smaller SQL statements).
const INSERT_BATCH_SIZE: usize = 50_000;

pub struct Importer;

impl Importer {
    /// Imports the given sources into `db`, reporting progress through the
    /// returned stream.
    ///
    /// The database is rebuilt from scratch: previous entries and metadata
    /// are dropped before the sources are imported. On success a fresh
    /// `pack_meta` row is written with the final entry count.
    pub fn import_sources<'a>(
        db: &'a SitePackDB,
        sources: Vec<SitePackSource>,
        temp_dir: PathBuf,
        site_pack_id: SitePackId,
    ) -> SitePackImportStream<'a> {
        Box::pin(stream! {
            yield SitePackCreationStatus::Started {
                site_pack_id: site_pack_id.clone(),
                progress: Percentage::new(0.0),
                phase: SitePackImportPhase::Initializing,
            };

            if let Err(error) = db.clear() {
                yield Self::failed(error, SitePackImportPhase::Initializing);
                return;
            }

            let sources_count = sources.len().max(1) as f64;
            let mut hash_batch: Vec<UriHash> = Vec::with_capacity(INSERT_BATCH_SIZE);

            for (source_index, source) in sources.iter().enumerate() {
                let source = source.clone();
                let source_base = (source_index as f64) / sources_count * 100.0;

                // ── 1. Obtain the list file ───────────────────────────
                let (file_path, is_temp_file) = match source.source_type {
                    SitePackSourceType::Url => {
                        let file_path = temp_dir.join(format!("{}.txt", source.id));

                        // Reuse the engine's bounded HTTP downloader rather
                        // than maintaining a second request/chunk/size policy.
                        // Quantize percentage for the existing event contract.
                        let download_started = Instant::now();
                        let download = file_download::download_file_with_limit(
                            &source.source,
                            file_path.clone(),
                            file_download::DEFAULT_MAX_DOWNLOAD_BYTES,
                        );
                        futures::pin_mut!(download);
                        let mut last_percent: u32 = 0;
                        while let Some(progress) = download.next().await {
                            match progress {
                                Ok(progress) => {
                                    let percent = progress.into_inner().floor() as u32;
                                    if percent != last_percent {
                                        last_percent = percent;
                                        yield Self::in_progress(
                                            &site_pack_id,
                                            Percentage::new(source_base + percent as f64 / sources_count),
                                            SitePackImportPhase::Downloading,
                                        );
                                    }
                                }
                                Err(error) => {
                                    let _ = tokio::fs::remove_file(&file_path).await;
                                    yield Self::failed(
                                        SitePackImportError::DownloadFailure {
                                            message: error.to_string(),
                                        },
                                        SitePackImportPhase::Downloading,
                                    );
                                    return;
                                }
                            }
                        }

                        info!(
                            site_pack_id = %site_pack_id,
                            source_id = %source.id,
                            elapsed_ms = download_started.elapsed().as_millis(),
                            "site pack source download timing"
                        );
                        (file_path, true)
                    }
                    SitePackSourceType::File => (PathBuf::from(&source.source), false),
                };

                // ── 2. Resolve the list format ────────────────────────
                let kind = match source.format {
                    SitePackFormat::Standard => DomainListFormatKind::Hosts,
                    SitePackFormat::WildCardDomains => DomainListFormatKind::Wildcard,
                    SitePackFormat::Auto => {
                        let sample = match read_sample(&file_path).await {
                            Ok(sample) => sample,
                            Err(error) => {
                                yield Self::failed(error, SitePackImportPhase::Importing);
                                return;
                            }
                        };
                        detect_format(&sample)
                    }
                };

                // ── 3. Stream the list into the database ──────────────
                let file = match tokio::fs::File::open(&file_path).await {
                    Ok(file) => file,
                    Err(error) => {
                        yield Self::failed(io_error(error), SitePackImportPhase::Importing);
                        return;
                    }
                };
                let total_bytes = match file.metadata().await {
                    Ok(metadata) => metadata.len(),
                    Err(error) => {
                        yield Self::failed(io_error(error), SitePackImportPhase::Importing);
                        return;
                    }
                };

                let mut reader = BufReader::with_capacity(64 * 1024, file);
                let mut line_buffer: Vec<u8> = Vec::new();
                let mut consumed_bytes: u64 = 0;
                let mut last_percent: u32 = 0;
                let mut bad_encoding_lines: u64 = 0;
                let mut malformed_lines: u64 = 0;
                let mut candidate_hashes: u64 = 0;
                let mut submitted_hashes: u64 = 0;
                let mut batch_flush_time = Duration::ZERO;
                let import_started = Instant::now();

                loop {
                    line_buffer.clear();
                    let bytes_read = match reader.read_until(b'\n', &mut line_buffer).await {
                        Ok(bytes_read) => bytes_read,
                        Err(error) => {
                            yield Self::failed(io_error(error), SitePackImportPhase::Importing);
                            return;
                        }
                    };
                    if bytes_read == 0 {
                        break;
                    }
                    consumed_bytes += bytes_read as u64;

                    let line = match std::str::from_utf8(&line_buffer) {
                        Ok(line) => line,
                        Err(_) => {
                            // Real-world lists contain the occasional
                            // non-UTF-8 line: skip it instead of aborting.
                            bad_encoding_lines += 1;
                            continue;
                        }
                    };

                    match kind.parse_line(line) {
                        Ok(DomainLineParsingResult::Skip) => {}
                        Ok(DomainLineParsingResult::Domains(domains)) => {
                            for domain in domains {
                                hash_batch.push(hash_host(&domain.into_inner()));
                                candidate_hashes += 1;
                                if hash_batch.len() >= INSERT_BATCH_SIZE {
                                    let write_started = Instant::now();
                                    let result = flush_hash_batch(db, &mut hash_batch);
                                    batch_flush_time += write_started.elapsed();
                                    match result {
                                        Ok(count) => submitted_hashes += count as u64,
                                        Err(error) => {
                                            yield Self::failed(error, SitePackImportPhase::Importing);
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Real-world lists contain the occasional
                            // malformed line (bare IPs, odd characters):
                            // skip it instead of aborting the whole import.
                            malformed_lines += 1;
                        }
                    }

                    let percent = progress_percent(consumed_bytes, total_bytes);
                    if percent != last_percent {
                        last_percent = percent;
                        yield Self::in_progress(
                            &site_pack_id,
                            Percentage::new(source_base + percent as f64 / sources_count),
                            SitePackImportPhase::Importing,
                        );
                    }
                }

                if bad_encoding_lines > 0 || malformed_lines > 0 {
                    warn!(
                        "skipped {bad_encoding_lines} non-UTF-8 and {malformed_lines} malformed lines in {}",
                        source.source
                    );
                }

                // Flush the remaining hashes of this source.
                if !hash_batch.is_empty() {
                    let write_started = Instant::now();
                    let result = flush_hash_batch(db, &mut hash_batch);
                    batch_flush_time += write_started.elapsed();
                    match result {
                        Ok(count) => submitted_hashes += count as u64,
                        Err(error) => {
                            yield Self::failed(error, SitePackImportPhase::Importing);
                            return;
                        }
                    }
                }

                info!(
                    site_pack_id = %site_pack_id,
                    source_id = %source.id,
                    bytes = total_bytes,
                    candidate_hashes,
                    submitted_hashes,
                    elapsed_ms = import_started.elapsed().as_millis(),
                    batch_flush_ms = batch_flush_time.as_millis(),
                    "site pack source import timing"
                );

                if is_temp_file {
                    let _ = tokio::fs::remove_file(&file_path).await;
                }
            }

            // ── 4. Finalize: write the pack metadata ──────────────────
            let entry_count = match db.count_entries() {
                Ok(entry_count) => entry_count,
                Err(error) => {
                    yield Self::failed(error, SitePackImportPhase::Finalizing);
                    return;
                }
            };
            let pack_meta = NewPackMetaModel {
                id: site_pack_id.to_string(),
                format_version: "1.0".to_string(),
                name: None,
                entry_count: entry_count as i32,
                built_at: Utc::now().to_rfc3339(),
            };
            if let Err(error) = db.insert_pack_meta(pack_meta) {
                yield Self::failed(error, SitePackImportPhase::Finalizing);
                return;
            }

            info!("site pack {site_pack_id} imported: {entry_count} entries");
            yield SitePackCreationStatus::Completed {
                site_pack_id,
                progress: Percentage::new(100.0),
                phase: SitePackImportPhase::Completed,
            };
        })
    }

    fn in_progress(
        site_pack_id: &SitePackId,
        progress: Percentage,
        phase: SitePackImportPhase,
    ) -> SitePackCreationStatus {
        SitePackCreationStatus::InProgress {
            site_pack_id: site_pack_id.clone(),
            progress,
            phase,
        }
    }

    fn failed(
        error: impl Into<SitePackImportError>,
        phase: SitePackImportPhase,
    ) -> SitePackCreationStatus {
        SitePackCreationStatus::Failed {
            error: error.into().to_string(),
            phase: Some(phase),
        }
    }
}

/// Sort by SQLite's signed INTEGER PRIMARY KEY order, then remove duplicates
/// before binding. This reduces random B-tree writes and repeated SQL work.
fn flush_hash_batch(
    db: &SitePackDB,
    batch: &mut Vec<UriHash>,
) -> Result<usize, crate::db::error::DBError> {
    batch.sort_unstable_by_key(|hash| *hash.as_ref() as i64);
    batch.dedup();
    let count = batch.len();
    db.insert_entries(batch)?;
    batch.clear();
    Ok(count)
}

fn io_error(error: std::io::Error) -> SitePackImportError {
    SitePackImportError::IoFailure {
        message: error.to_string(),
    }
}

/// Progress as a whole percentage of `consumed` out of `total` bytes.
fn progress_percent(consumed: u64, total: u64) -> u32 {
    if total == 0 {
        return 100;
    }
    (consumed.min(total) as f64 / total as f64 * 100.0) as u32
}

/// Reads up to `DETECTION_SAMPLE_LINES` leading lines of the file.
async fn read_sample(path: &Path) -> Result<String, SitePackImportError> {
    let file = tokio::fs::File::open(path).await.map_err(io_error)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut sample = String::new();
    for _ in 0..ace_domain_blocklist::DETECTION_SAMPLE_LINES {
        let mut line_buffer: Vec<u8> = Vec::new();
        let bytes_read = reader
            .read_until(b'\n', &mut line_buffer)
            .await
            .map_err(io_error)?;
        if bytes_read == 0 {
            break;
        }
        sample.push_str(&String::from_utf8_lossy(&line_buffer));
    }
    Ok(sample)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network_block::site_pack::importer::{
        SitePackFormat, SitePackId, SitePackSource, SitePackSourceId, SitePackSourceType,
    };
    use axum::routing::get;
    use std::path::Path;
    use tempfile::tempdir;
    use uuid::Uuid;

    fn test_db(db_path: &Path) -> SitePackDB {
        SitePackDB::from(db_path).expect("failed to create test site pack db")
    }

    fn file_source(path: &str, format: SitePackFormat) -> SitePackSource {
        SitePackSource {
            id: SitePackSourceId::new(Uuid::now_v7()),
            site_pack_id: SitePackId::try_new("test-pack").unwrap(),
            source: path.to_string(),
            source_type: SitePackSourceType::File,
            format,
        }
    }

    fn url_source(url: String, format: SitePackFormat) -> SitePackSource {
        SitePackSource {
            id: SitePackSourceId::new(Uuid::now_v7()),
            site_pack_id: SitePackId::try_new("test-pack").unwrap(),
            source: url,
            source_type: SitePackSourceType::Url,
            format,
        }
    }

    async fn run_import(
        db: &SitePackDB,
        sources: Vec<SitePackSource>,
    ) -> Vec<SitePackCreationStatus> {
        let temp = tempdir().unwrap();
        Importer::import_sources(
            db,
            sources,
            temp.path().to_path_buf(),
            SitePackId::try_new("test-pack").unwrap(),
        )
        .collect()
        .await
    }

    #[tokio::test]
    async fn imports_wildcard_file_with_hashes() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        std::fs::write(&list_path, "# comment\nexample.com\nads.example.com\n").unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::WildCardDomains,
            )],
        )
        .await;

        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 2);
        assert_eq!(db.get_entry_count().unwrap(), 2);
        assert!(
            db.blocking_result(&hash_host("example.com"))
                .unwrap()
                .is_blocked()
        );
        assert!(
            db.blocking_result(&hash_host("ads.example.com"))
                .unwrap()
                .is_blocked()
        );
        assert!(
            !db.blocking_result(&hash_host("not-in-list.com"))
                .unwrap()
                .is_blocked()
        );
    }

    #[tokio::test]
    async fn auto_detects_hosts_format() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        std::fs::write(
            &list_path,
            "# comment\n0.0.0.0 example.com\n127.0.0.1 www.example.com\n",
        )
        .unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::Auto,
            )],
        )
        .await;

        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 2);
        assert!(
            db.blocking_result(&hash_host("www.example.com"))
                .unwrap()
                .is_blocked()
        );
    }

    #[tokio::test]
    async fn auto_detects_adblock_format_and_ignores_exceptions() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        std::fs::write(
            &list_path,
            "[Adblock Plus]\n! Title: Light\n||example.com^\n@@||allowed.com^\n||ads.example.com^\n",
        )
        .unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::Auto,
            )],
        )
        .await;

        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 2);
        assert!(
            db.blocking_result(&hash_host("example.com"))
                .unwrap()
                .is_blocked()
        );
        assert!(
            !db.blocking_result(&hash_host("allowed.com"))
                .unwrap()
                .is_blocked()
        );
    }

    #[tokio::test]
    async fn inserts_in_batches_of_5000() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        let mut content = String::new();
        for index in 0..12_000 {
            content.push_str(&format!("domain{index}.com\n"));
        }
        std::fs::write(&list_path, content).unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::WildCardDomains,
            )],
        )
        .await;

        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 12_000);
        assert!(
            db.blocking_result(&hash_host("domain11999.com"))
                .unwrap()
                .is_blocked()
        );
    }

    #[test]
    fn flush_hash_batch_sorts_and_deduplicates_signed_hashes() {
        let temp = tempdir().unwrap();
        let db = test_db(&temp.path().join("pack.db"));
        let mut batch = vec![
            UriHash::new(u64::MAX),
            UriHash::new(1),
            UriHash::new(u64::MAX),
            UriHash::new(0),
            UriHash::new(1),
        ];

        assert_eq!(flush_hash_batch(&db, &mut batch).unwrap(), 3);
        assert!(batch.is_empty());
        assert_eq!(db.count_entries().unwrap(), 3);
        for hash in [0, 1, u64::MAX] {
            assert!(
                db.blocking_result(&UriHash::new(hash))
                    .unwrap()
                    .is_blocked()
            );
        }
    }

    #[tokio::test]
    async fn duplicate_domains_are_ignored() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        std::fs::write(&list_path, "example.com\nexample.com\nexample.com\n").unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::WildCardDomains,
            )],
        )
        .await;

        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 1);
    }

    #[tokio::test]
    async fn malformed_lines_are_skipped() {
        // Real-world lists contain bare IPs, underscore domains and odd
        // lines: valid entries must be imported and the rest skipped.
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        std::fs::write(
            &list_path,
            "example.com\n1.0.0.4\n_domainkey.youtube.com\nbad..com\n0.0.0.0 hosts-line.com\n",
        )
        .unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::WildCardDomains,
            )],
        )
        .await;

        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        // example.com, 1.0.0.4 (accepted as an entry) and the underscore
        // domain are imported; "bad..com" and the hosts-format line are
        // skipped.
        assert_eq!(db.count_entries().unwrap(), 3);
        assert!(
            db.blocking_result(&hash_host("example.com"))
                .unwrap()
                .is_blocked()
        );
        assert!(
            db.blocking_result(&hash_host("1.0.0.4"))
                .unwrap()
                .is_blocked()
        );
        assert!(
            db.blocking_result(&hash_host("_domainkey.youtube.com"))
                .unwrap()
                .is_blocked()
        );
        assert!(
            !db.blocking_result(&hash_host("hosts-line.com"))
                .unwrap()
                .is_blocked()
        );
    }

    #[tokio::test]
    async fn missing_file_fails_the_import() {
        let temp = tempdir().unwrap();
        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                temp.path().join("does-not-exist.txt").to_str().unwrap(),
                SitePackFormat::WildCardDomains,
            )],
        )
        .await;

        assert!(statuses.iter().any(|status| matches!(
            status,
            SitePackCreationStatus::Failed {
                phase: Some(SitePackImportPhase::Importing),
                ..
            }
        )));
    }

    #[tokio::test]
    async fn reports_progress_and_completion() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("list.txt");
        let mut content = String::new();
        for index in 0..200 {
            content.push_str(&format!("domain{index}.com\n"));
        }
        std::fs::write(&list_path, content).unwrap();

        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![file_source(
                list_path.to_str().unwrap(),
                SitePackFormat::WildCardDomains,
            )],
        )
        .await;

        assert!(matches!(
            statuses.first(),
            Some(SitePackCreationStatus::Started { .. })
        ));
        assert!(statuses.iter().any(|status| matches!(
            status,
            SitePackCreationStatus::InProgress {
                phase: SitePackImportPhase::Importing,
                ..
            }
        )));
        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
    }

    #[tokio::test]
    async fn imports_from_url() {
        let router =
            axum::Router::new().route("/list.txt", get(|| async { "example.com\nother.com\n" }));
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/list.txt").unwrap();

        let temp = tempdir().unwrap();
        let db = test_db(&temp.path().join("pack.db"));
        let statuses = run_import(
            &db,
            vec![url_source(url.to_string(), SitePackFormat::WildCardDomains)],
        )
        .await;

        assert!(statuses.iter().any(|status| matches!(
            status,
            SitePackCreationStatus::InProgress {
                phase: SitePackImportPhase::Downloading,
                ..
            }
        )));
        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 2);
        assert!(
            db.blocking_result(&hash_host("example.com"))
                .unwrap()
                .is_blocked()
        );
    }
    #[tokio::test]
    async fn http_error_during_import_emits_download_failure() {
        let router = axum::Router::new().route(
            "/error",
            get(|| async { (axum::http::StatusCode::BAD_GATEWAY, "temporary failure") }),
        );
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/error").unwrap();
        let temp = tempdir().unwrap();
        let db = test_db(&temp.path().join("pack.db"));

        let statuses = run_import(
            &db,
            vec![url_source(url.to_string(), SitePackFormat::WildCardDomains)],
        )
        .await;
        assert!(statuses.iter().any(|status| matches!(
            status,
            SitePackCreationStatus::Failed {
                phase: Some(SitePackImportPhase::Downloading),
                ..
            }
        )));
        assert_eq!(db.count_entries().unwrap(), 0);
    }
}
