//! Shared outbound-file-download utilities.
//!
//! Every download in the engine funnels through this module so that the
//! timeout and size-limit policy lives in exactly one place:
//!
//! - **Connect timeout** (10 s): an unreachable host must fail fast instead
//!   of hanging a caller (error.g. startup content init) for minutes.
//! - **Overall request timeout** (60 s): the whole request, including body
//!   reads, is bounded so a stalled connection can never hang the engine.
//! - **Size caps**: `content_length` is checked up front and the streamed
//!   body is checked again as it is written, so a lying or chunked server
//!   cannot fill the disk.

use std::path::PathBuf;
use std::time::Duration;

use async_stream::try_stream;
use futures::Stream;
use sha2::{Digest, Sha256};
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{error, info, warn};

use crate::core::common::percentage::Percentage;

pub type DownloadError = Box<dyn std::error::Error + Send + Sync>;

/// How long to wait for the TCP connection to be established.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Total time budget for the entire request, including reading the body.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Default cap for general-purpose downloads (site-list sources, site-pack
/// backup archives). Generous but bounded: a hostile or broken server cannot
/// fill the disk.
pub const DEFAULT_MAX_DOWNLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB
/// Tighter cap for script bundles, which are small by construction.
pub const SCRIPT_BUNDLE_MAX_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB

/// Builds a [`reqwest::Client`] with the engine's outbound timeout policy.
///
/// Callers that need a client for anything other than downloading should
/// still use this helper (or replicate the same policy) so no outbound
/// request can hang the engine (see docs/review/architecture-review-2026-09.md
/// SVC-01).
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("failed to build HTTP client")
}

pub fn download_file(
    url: &str,
    path: PathBuf,
) -> impl Stream<Item = Result<Percentage, DownloadError>> {
    download_file_with_limit(url, path, DEFAULT_MAX_DOWNLOAD_BYTES)
}

/// Downloads `url` to `path`, enforcing the engine timeout policy and a
/// `max_bytes` cap on the total body size.
pub fn download_file_with_limit(
    url: &str,
    path: PathBuf,
    max_bytes: u64,
) -> impl Stream<Item = Result<Percentage, DownloadError>> {
    let url = url.to_owned();
    try_stream! {
        let client = http_client();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|error| -> DownloadError { Box::new(error) })?;
        let response = response
            .error_for_status()
            .map_err(|error| -> DownloadError { Box::new(error) })?;

        let total_size = response.content_length();

        if let Some(total) = total_size
            && total > max_bytes
        {
            let error: DownloadError = format!(
                "download of {url} refused: content length {total} exceeds the {max_bytes}-byte limit"
            )
            .into();
            Err(error)?;
        }

        let mut file = File::create(&path)
            .await
            .map_err(|error| -> DownloadError { Box::new(error) })?;

        let mut downloaded: u64 = 0;
        let byte_stream = response.bytes_stream();

        for await item in byte_stream {
            let chunk = item.map_err(|error| -> DownloadError { Box::new(error) })?;

            downloaded += chunk.len() as u64;
            if downloaded > max_bytes {
                let error: DownloadError = format!(
                    "download of {url} refused: body exceeds the {max_bytes}-byte limit"
                )
                .into();
                Err(error)?;
            }

            file.write_all(&chunk)
                .await
                .map_err(|error| -> DownloadError { Box::new(error) })?;

            if let Some(total) = total_size
                && total > 0
            {
                let percentage = (downloaded as f64 / total as f64) * 100.0;
                yield Percentage::new(percentage);
            }
        }

        file.flush()
            .await
            .map_err(|error| -> DownloadError { Box::new(error) })?;
    }
}

/// Downloads a ZIP from `url`, computes its SHA-256, and compares it against
/// `expected_sha256`. Retries up to `max_retries` times if verification
/// fails. Returns the downloaded bytes on success.
///
/// The body is read into memory, so it is capped at `max_bytes`.
pub async fn download_and_verify(
    url: &str,
    expected_sha256: &str,
    label: &str,
    max_retries: u8,
    max_bytes: u64,
) -> Result<bytes::Bytes, DownloadError> {
    let expected = expected_sha256.to_lowercase();

    for attempt in 1..=max_retries {
        if attempt > 1 {
            info!(
                "Retrying {} download — attempt {}/{}",
                label, attempt, max_retries
            );
        }

        let client = http_client();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|error| -> DownloadError { Box::new(error) })?;

        if let Some(total) = response.content_length()
            && total > max_bytes
        {
            return Err(format!(
                "download of {label} refused: content length {total} exceeds the {max_bytes}-byte limit"
            )
            .into());
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|error| -> DownloadError { Box::new(error) })?;

        if bytes.len() as u64 > max_bytes {
            return Err(format!(
                "download of {label} refused: body exceeds the {max_bytes}-byte limit"
            )
            .into());
        }

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode(hasher.finalize());

        if actual == expected {
            info!("{} SHA-256 verified successfully.", label);
            return Ok(bytes);
        }

        if attempt < max_retries {
            warn!(
                "{} SHA-256 mismatch on attempt {} — expected {}, got {}",
                label, attempt, expected, actual
            );
        } else {
            error!(
                "{} SHA-256 verification failed after {} attempts. Giving up. Expected: {}, Got: {}",
                label, attempt, expected, actual
            );
        }
    }

    Err(format!(
        "{} integrity check failed after {} retries",
        label, max_retries
    )
    .into())
}

/// Extracts a ZIP archive (in memory) into `dest_dir`, protecting against
/// path traversal and decompression bombs.
///
/// Every entry name is checked with `zip`'s `enclosed_name` (rejects absolute
/// paths, `..` segments, drive prefixes, and backslashes on Windows); the
/// cumulative extracted size is capped at `max_bytes`. `dest_dir` is created
/// if missing. Returns an error message on any violation — nothing is left
/// partially extracted on failure (the caller owns cleanup of `dest_dir`).
pub fn extract_zip_safely(
    archive_bytes: &[u8],
    dest_dir: &std::path::Path,
    max_bytes: u64,
) -> Result<(), String> {
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("failed to open zip archive: {error}"))?;

    std::fs::create_dir_all(dest_dir)
        .map_err(|error| format!("failed to create extraction directory: {error}"))?;

    let mut total_extracted: u64 = 0;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to read zip entry {index}: {error}"))?;

        // Strict validation on the RAW entry name. The zip crate's
        // `enclosed_name` *sanitizes* absolute paths and `..` segments
        // (normalizing them into the destination) rather than rejecting
        // them, so we check the raw name first: absolute paths, backslashes,
        // drive prefixes, and `..` segments are all refused outright.
        let raw_name = entry
            .name()
            .map_err(|error| format!("failed to read zip entry {index} name: {error}"))?
            .to_string();
        if raw_name.contains('\0') {
            return Err(format!("zip entry {index} has an unsafe path"));
        }
        if raw_name.starts_with('/') || raw_name.starts_with('\\') {
            return Err(format!("zip entry {index} has an unsafe (absolute) path"));
        }
        if raw_name.contains(':') || raw_name.contains('\\') {
            return Err(format!("zip entry {index} has an unsafe path"));
        }
        if raw_name.split('/').any(|segment| segment == "..") {
            return Err(format!("zip entry {index} has an unsafe (traversal) path"));
        }

        // Final layer: `enclosed_name` returns `None` for anything the
        // sanitizer still considers unsafe (error.g. NUL bytes).
        let relative_path = entry
            .enclosed_name()
            .ok_or_else(|| format!("zip entry {index} has an unsafe path"))?;

        if entry.is_dir() {
            let _ = std::fs::create_dir_all(dest_dir.join(relative_path));
            continue;
        }

        let entry_size = entry.size();
        total_extracted = total_extracted
            .checked_add(entry_size)
            .ok_or_else(|| "zip archive total size overflow".to_string())?;
        if total_extracted > max_bytes {
            return Err(format!(
                "zip archive exceeds the {max_bytes}-byte extraction limit"
            ));
        }

        let target = dest_dir.join(relative_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create directory: {error}"))?;
        }
        let mut out = std::io::BufWriter::new(
            std::fs::File::create(&target)
                .map_err(|error| format!("failed to create {}: {error}", target.display()))?,
        );
        let copied = std::io::copy(&mut entry, &mut out)
            .map_err(|error| format!("failed to extract {}: {error}", target.display()))?;
        if copied != entry_size {
            return Err(format!(
                "zip entry {index} size mismatch: expected {entry_size}, extracted {copied}"
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use futures::StreamExt;
    use sha2::{Digest, Sha256};

    /// Pins and collects a [`download_file_with_limit`] stream.
    async fn collect_download(
        url: &str,
        target: std::path::PathBuf,
        max_bytes: u64,
    ) -> Vec<Result<Percentage, DownloadError>> {
        let stream = download_file_with_limit(url, target, max_bytes);
        futures::pin_mut!(stream);
        stream.collect().await
    }

    #[tokio::test]
    async fn streamed_download_enforces_the_size_cap() {
        let router = axum::Router::new().route("/big.txt", get(|| async { "x".repeat(4096) }));
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/big.txt").unwrap();

        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("big.txt");
        let statuses = collect_download(url.as_str(), target.clone(), 1024).await;
        assert!(
            statuses.iter().any(|s| s.is_err()),
            "the download must fail once the cap is exceeded"
        );
        // At most `max_bytes` may ever be written before the error.
        let size_on_disk = tokio::fs::metadata(&target)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        assert!(
            size_on_disk <= 1024,
            "no file larger than the cap may be left behind (was {size_on_disk})"
        );
    }

    #[tokio::test]
    async fn content_length_is_checked_up_front() {
        let router = axum::Router::new().route(
            "/big.txt",
            get(|| async { ([("content-length", "5000")], "x".repeat(4096)) }),
        );
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/big.txt").unwrap();

        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("big.txt");
        let statuses = collect_download(url.as_str(), target.clone(), 1024).await;
        assert!(
            statuses.iter().any(|s| s.is_err()),
            "an oversized content-length must be refused before the body is written"
        );
        assert!(!target.exists());
    }

    #[tokio::test]
    async fn download_and_verify_accepts_matching_sha256() {
        let body = b"example.com\nother.com\n".to_vec();
        let expected = hex::encode(Sha256::digest(&body));
        let body_for_handler = body.clone();
        let router =
            axum::Router::new().route("/list.zip", get(move || async { body_for_handler }));
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/list.zip").unwrap();

        let result =
            download_and_verify(url.as_str(), &expected, "test bundle", 2, 1024 * 1024).await;
        assert_eq!(result.unwrap().as_ref(), &body[..]);
    }

    #[tokio::test]
    async fn download_and_verify_rejects_a_mismatched_sha256() {
        let body = b"example.com\n".to_vec();
        let router = axum::Router::new().route("/list.zip", get(|| async { body }));
        let server = axum_test::TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(router);
        let url = server.server_url("/list.zip").unwrap();

        let result = download_and_verify(
            url.as_str(),
            "0000000000000000000000000000000000000000000000000000000000000000",
            "test bundle",
            2,
            1024 * 1024,
        )
        .await;
        assert!(result.is_err());
    }

    /// Builds an in-memory zip archive from `(name, contents)` pairs.
    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let cursor = std::io::Cursor::new(Vec::new());
        let mut zip_writer = zip::ZipWriter::new(cursor);
        let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, contents) in entries {
            zip_writer
                .start_file(*name, options)
                .expect("start zip entry");
            zip_writer.write_all(contents).expect("write zip entry");
        }
        zip_writer.finish().expect("finish zip").into_inner()
    }

    #[test]
    fn extract_zip_safely_extracts_plain_archives() {
        let temp = tempfile::tempdir().unwrap();
        let archive = make_zip(&[("scripts-data.json", b"{\"ok\":true}")]);

        extract_zip_safely(&archive, temp.path(), 1024 * 1024).unwrap();

        let extracted = std::fs::read_to_string(temp.path().join("scripts-data.json")).unwrap();
        assert_eq!(extracted, "{\"ok\":true}");
    }

    #[test]
    fn extract_zip_safely_rejects_traversal_entries() {
        let temp = tempfile::tempdir().unwrap();

        for evil_name in [
            "../escape.txt",
            "../../escape.txt",
            "sub/../../escape.txt",
            "C:/windows/evil.txt",
            "\\evil.txt",
        ] {
            let archive = make_zip(&[(evil_name, b"evil")]);
            let result = extract_zip_safely(&archive, temp.path(), 1024 * 1024);
            assert!(result.is_err(), "entry {evil_name:?} must be rejected");
            assert!(
                !temp.path().parent().unwrap().join("escape.txt").exists(),
                "no file may escape the extraction root"
            );
        }
    }

    #[test]
    fn extract_zip_safely_enforces_the_size_cap() {
        let temp = tempfile::tempdir().unwrap();
        let archive = make_zip(&[("big.bin", &vec![0u8; 4096])]);

        let result = extract_zip_safely(&archive, temp.path(), 1024);
        assert!(result.is_err(), "an oversized archive must be rejected");
    }
}
