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
use futures::{StreamExt, stream::BoxStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::core::utils::file_download;

/// Number of hashes accumulated before flushing to SQLite (which uses smaller SQL statements).
const INSERT_BATCH_SIZE: usize = 50_000;

/// A bounded producer/consumer pipeline: parsing never holds a SQLite writer
/// connection, and the worker cannot outlive its staged database on cancellation.
struct BatchWriter {
    sender: Option<mpsc::Sender<Vec<UriHash>>>,
    worker: Option<std::thread::JoinHandle<Result<WriteStats, crate::db::error::DBError>>>,
}

#[derive(Default)]
struct WriteStats {
    submitted: usize,
    elapsed: Duration,
}

impl BatchWriter {
    fn start(db: SitePackDB) -> Result<Self, SitePackImportError> {
        const QUEUE_CAPACITY: usize = 2;
        let (sender, mut receiver) = mpsc::channel::<Vec<UriHash>>(QUEUE_CAPACITY);
        let worker = std::thread::Builder::new()
            .name("ace-site-pack-writer".into())
            .spawn(move || {
                let mut stats = WriteStats::default();
                while let Some(mut batch) = receiver.blocking_recv() {
                    let started = Instant::now();
                    stats.submitted += flush_hash_batch(&db, &mut batch)?;
                    stats.elapsed += started.elapsed();
                }
                Ok(stats)
            })
            .map_err(|error| SitePackImportError::DatabaseFailure {
                message: format!("failed to start site pack writer: {error}"),
            })?;
        Ok(Self {
            sender: Some(sender),
            worker: Some(worker),
        })
    }

    async fn enqueue(&self, batch: Vec<UriHash>) -> Result<(), SitePackImportError> {
        if let Some(sender) = &self.sender {
            sender
                .send(batch)
                .await
                .map_err(|_| SitePackImportError::DatabaseFailure {
                    message: "site pack writer stopped before accepting a batch".to_string(),
                })
        } else {
            Err(SitePackImportError::DatabaseFailure {
                message: "site pack writer is closed".to_string(),
            })
        }
    }

    fn finish(mut self) -> Result<WriteStats, SitePackImportError> {
        self.sender.take();
        self.join_worker()
    }

    fn join_worker(&mut self) -> Result<WriteStats, SitePackImportError> {
        let Some(worker) = self.worker.take() else {
            return Err(SitePackImportError::DatabaseFailure {
                message: "site pack writer already joined".to_string(),
            });
        };
        worker
            .join()
            .map_err(|_| SitePackImportError::DatabaseFailure {
                message: "site pack writer panicked".to_string(),
            })?
            .map_err(Into::into)
    }
}

impl Drop for BatchWriter {
    fn drop(&mut self) {
        // Stream cancellation drops the producer. Joining here is deliberate:
        // the staging cleanup must never remove a database with an active writer.
        self.sender.take();
        if self.worker.is_some()
            && let Err(error) = self.join_worker()
        {
            warn!(%error, "site pack writer failed during cancellation");
        }
    }
}

const MAX_CONCURRENT_DOWNLOADS: usize = 3;
const DOWNLOAD_PROGRESS_WEIGHT: f64 = 20.0;

enum DownloadEvent {
    Progress { index: usize, percent: u32 },
    Completed { index: usize },
    Failed(String),
}

/// Lazy streams run concurrently only while polled by the importer. Dropping
/// the import stream cancels every request, without detached download tasks.
fn download_source_stream(
    index: usize,
    source: SitePackSource,
    path: PathBuf,
) -> BoxStream<'static, DownloadEvent> {
    Box::pin(stream! {
        let started = Instant::now();
        let download = file_download::download_large_list_with_limit(
            &source.source,
            path,
            file_download::DEFAULT_MAX_DOWNLOAD_BYTES,
        );
        futures::pin_mut!(download);
        let mut last_percent = 0;
        while let Some(progress) = download.next().await {
            match progress {
                Ok(progress) => {
                    let percent = (progress.into_inner().floor() as u32).min(100);
                    if percent != last_percent {
                        last_percent = percent;
                        yield DownloadEvent::Progress { index, percent };
                    }
                }
                Err(error) => {
                    yield DownloadEvent::Failed(error.to_string());
                    return;
                }
            }
        }
        info!(source_id = %source.id, elapsed_ms = started.elapsed().as_millis(),
            "site pack source download timing");
        yield DownloadEvent::Completed { index };
    })
}

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
            // Downloads are bounded to the next small group. The following
            // group is not fetched until the current group's files are consumed.
            for (source_index, source) in sources.iter().enumerate() {
                let group_start = source_index / MAX_CONCURRENT_DOWNLOADS * MAX_CONCURRENT_DOWNLOADS;
                let group_end = (group_start + MAX_CONCURRENT_DOWNLOADS).min(sources.len());
                let source_group = &sources[group_start..group_end];
                let group_base = group_start as f64 / sources_count * 100.0;
                let group_span = source_group.len() as f64 / sources_count * 100.0;
                let url_count = source_group.iter().filter(|source| source.source_type == SitePackSourceType::Url).count();
                let download_weight = if url_count == 0 { 0.0 } else { DOWNLOAD_PROGRESS_WEIGHT };
                if source_index == group_start {
                    let mut active_downloads = futures::stream::SelectAll::new();
                    for (index, source) in source_group.iter().enumerate() {
                        if source.source_type == SitePackSourceType::Url {
                            active_downloads.push(download_source_stream(index, source.clone(),
                                temp_dir.join(format!("{}.txt", source.id))));
                        }
                    }
                    let mut download_percents = vec![0_u32; source_group.len()];
                    let mut last_download_progress = 0_u32;
                    while let Some(event) = active_downloads.next().await {
                        match event {
                            DownloadEvent::Progress { index, percent } => {
                                download_percents[index] = percent;
                            }
                            DownloadEvent::Completed { index } => {
                                download_percents[index] = 100;
                            }
                            DownloadEvent::Failed(message) => {
                                yield Self::failed(SitePackImportError::DownloadFailure { message },
                                    SitePackImportPhase::Downloading);
                                return;
                            }
                        }
                        let total_percent: u64 = download_percents.iter().map(|percent| u64::from(*percent)).sum();
                        let weighted = (group_base + total_percent as f64 / (url_count.max(1) as f64 * 100.0)
                            * download_weight / 100.0 * group_span).floor() as u32;
                        if weighted > last_download_progress {
                            last_download_progress = weighted;
                            yield Self::in_progress(&site_pack_id, Percentage::new(weighted as f64),
                                SitePackImportPhase::Downloading);
                        }
                    }

                }
                let source_offset = source_index - group_start;
                let source = source.clone();
                let source_base = group_base + group_span * (download_weight / 100.0
                    + source_offset as f64 / source_group.len() as f64 * (100.0 - download_weight) / 100.0);

                // URL sources have already been downloaded concurrently.
                let (file_path, is_temp_file) = match source.source_type {
                    SitePackSourceType::Url => (temp_dir.join(format!("{}.txt", source.id)), true),
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
                let mut enqueue_time = Duration::ZERO;
                let import_started = Instant::now();
                let writer = match BatchWriter::start(db.clone()) {
                    Ok(writer) => writer,
                    Err(error) => {
                        yield Self::failed(error, SitePackImportPhase::Importing);
                        return;
                    }
                };

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
                                    let enqueue_started = Instant::now();
                                    let full_batch = std::mem::replace(
                                        &mut hash_batch, Vec::with_capacity(INSERT_BATCH_SIZE)
                                    );
                                    if let Err(error) = writer.enqueue(full_batch).await {
                                        yield Self::failed(error, SitePackImportPhase::Importing);
                                        return;
                                    }
                                    enqueue_time += enqueue_started.elapsed();
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
                            Percentage::new(source_base + percent as f64 / 100.0 * group_span
                                / source_group.len() as f64 * (100.0 - download_weight) / 100.0),
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
                    let final_batch = std::mem::take(&mut hash_batch);
                    if let Err(error) = writer.enqueue(final_batch).await {
                        yield Self::failed(error, SitePackImportPhase::Importing);
                        return;
                    }
                }

                // The stage cannot be activated until all database writes complete.
                let write_stats = match writer.finish() {
                    Ok(stats) => stats,
                    Err(error) => {
                        yield Self::failed(error, SitePackImportPhase::Importing);
                        return;
                    }
                };
                info!(
                    site_pack_id = %site_pack_id,
                    source_id = %source.id,
                    bytes = total_bytes,
                    candidate_hashes,
                    submitted_hashes = write_stats.submitted,
                    elapsed_ms = import_started.elapsed().as_millis(),
                    enqueue_ms = enqueue_time.as_millis(),
                    db_write_ms = write_stats.elapsed.as_millis(),
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
    async fn writer_pipeline_handles_multiple_full_batches_and_duplicates() {
        let temp = tempdir().unwrap();
        let list_path = temp.path().join("large-list.txt");
        let mut content = String::with_capacity(3_000_000);
        for index in 0..115_000 {
            content.push_str(&format!("domain{index}.com\n"));
        }
        for index in 0..5_000 {
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
        assert_eq!(db.count_entries().unwrap(), 115_000);
        assert!(
            db.blocking_result(&hash_host("domain114999.com"))
                .unwrap()
                .is_blocked()
        );
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
    async fn downloads_multiple_sources_concurrently_before_importing() {
        let rendezvous = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let first = rendezvous.clone();
        let second = rendezvous.clone();
        let router = axum::Router::new()
            .route(
                "/first",
                get(move || {
                    let rendezvous = first.clone();
                    async move {
                        rendezvous.wait().await;
                        "example.com\n"
                    }
                }),
            )
            .route(
                "/second",
                get(move || {
                    let rendezvous = second.clone();
                    async move {
                        rendezvous.wait().await;
                        "second.example\n"
                    }
                }),
            );
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let first_url = server.server_url("/first").unwrap();
        let second_url = server.server_url("/second").unwrap();
        let temp = tempdir().unwrap();
        let db = test_db(&temp.path().join("pack.db"));
        let statuses = tokio::time::timeout(
            Duration::from_secs(4),
            run_import(
                &db,
                vec![
                    url_source(first_url.to_string(), SitePackFormat::WildCardDomains),
                    url_source(second_url.to_string(), SitePackFormat::WildCardDomains),
                ],
            ),
        )
        .await
        .expect("downloads should overlap rather than wait for each other");
        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 2);
        let mut previous = 0.0;
        for status in statuses {
            if let SitePackCreationStatus::InProgress { progress, .. } = status {
                assert!(progress.into_inner() >= previous);
                previous = progress.into_inner();
            }
        }
    }

    #[tokio::test]
    async fn downloading_many_sources_uses_bounded_groups() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let active = std::sync::Arc::new(AtomicUsize::new(0));
        let peak = std::sync::Arc::new(AtomicUsize::new(0));
        let requests = active.clone();
        let maximum = peak.clone();
        let router = axum::Router::new().route(
            "/list",
            get(move || {
                let active = requests.clone();
                let peak = maximum.clone();
                async move {
                    let in_flight = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(in_flight, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(80)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    "example.com\n"
                }
            }),
        );
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/list").unwrap().to_string();
        let temp = tempdir().unwrap();
        let db = test_db(&temp.path().join("pack.db"));
        let sources = (0..5)
            .map(|_| url_source(url.clone(), SitePackFormat::WildCardDomains))
            .collect();
        let statuses = tokio::time::timeout(Duration::from_secs(5), run_import(&db, sources))
            .await
            .unwrap();
        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert_eq!(db.count_entries().unwrap(), 1);
        assert!(peak.load(Ordering::SeqCst) <= MAX_CONCURRENT_DOWNLOADS);
        let mut previous = 0.0;
        for status in statuses {
            if let SitePackCreationStatus::InProgress { progress, .. } = status {
                assert!(progress.into_inner() >= previous);
                previous = progress.into_inner();
            }
        }
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
