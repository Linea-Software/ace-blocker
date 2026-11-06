//! Whole-product release discovery, deliberately independent of content packs
//! and TrustedClock. An unsigned server version string is never install authority.
//! An unconfigured build safely reports no *verified* release.

#[cfg(target_os = "windows")]
use ace_update_security::VerifiedRelease;
use ace_update_security::{TrustPolicy, UpdateError, verify_release};
use semver::Version;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};
#[cfg(target_os = "windows")]
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::warn;

const MIN_CHECK_GAP: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_MANIFEST_BYTES: usize = 16 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);

// No endpoint is hard-coded until the Ace website implements and publishes
// the signed-manifest contract. A production build provides this HTTPS URL.
const MANIFEST_URL: Option<&str> = option_env!("ACE_RELEASE_MANIFEST_URL");

#[derive(Clone, Debug)]
pub enum ProductUpdateCheck {
    Checking,
    Available(String),
    Downloading(String),
    Verifying(String),
    Downloaded(String),
    UpToDate,
    Disabled,
    Error,
}

impl Default for ProductUpdateCheck {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Default)]
struct CheckCache {
    checked_at: Option<Instant>,
    result: ProductUpdateCheck,
}
static CHECK: OnceLock<Mutex<CheckCache>> = OnceLock::new();

/// Single-flight signed release discovery. It NEVER awaits a multi-minute
/// download and does not acquire a content, database, or NetworkManager lock.
/// Download and Authenticode verification are completed independently.
pub async fn check_available() -> ProductUpdateCheck {
    let gate = CHECK.get_or_init(|| Mutex::new(CheckCache::default()));
    {
        let mut cache = gate.lock().await;
        if cache
            .checked_at
            .is_some_and(|at| at.elapsed() < MIN_CHECK_GAP)
        {
            return cache.result.clone();
        }
        // Mark before I/O: concurrent RPC calls see a busy state, rather
        // than launching another network fetch.
        cache.checked_at = Some(Instant::now());
        cache.result = ProductUpdateCheck::Checking;
    }

    let result = fetch_verified_release().await;
    let mut cache = gate.lock().await;
    match result {
        Ok(Some(release)) => {
            let version = release.version().to_string();
            #[cfg(target_os = "windows")]
            if crate::platform::in_service_mode() {
                cache.result = ProductUpdateCheck::Downloading(version.clone());
                tokio::spawn(async move { download_and_verify(release).await });
                return cache.result.clone();
            }
            cache.result = ProductUpdateCheck::Available(version);
        }
        Ok(None) => cache.result = ProductUpdateCheck::UpToDate,
        Err(error) => {
            if error
                .downcast_ref::<UpdateError>()
                .is_some_and(|e| matches!(e, UpdateError::NotConfigured))
            {
                cache.result = ProductUpdateCheck::Disabled;
            } else {
                warn!("Product update discovery unavailable: {error}");
                cache.result = ProductUpdateCheck::Error;
            }
        }
    }
    cache.result.clone()
}

#[cfg(target_os = "windows")]
async fn set_download_phase(version: &str, next: ProductUpdateCheck) {
    let gate = CHECK.get_or_init(|| Mutex::new(CheckCache::default()));
    let mut cache = gate.lock().await;
    let current = matches!(
        &cache.result,
        ProductUpdateCheck::Downloading(active) | ProductUpdateCheck::Verifying(active)
            if active == version
    );
    if current {
        cache.result = next;
    }
}

#[cfg(target_os = "windows")]
async fn download_and_verify(release: VerifiedRelease) {
    let version = release.version().to_string();
    let result = async {
        let staged = stage_verified_msi(&release).await?;
        set_download_phase(&version, ProductUpdateCheck::Verifying(version.clone())).await;
        verify_authenticode(&staged, &release).await
    }
    .await;

    match result {
        Ok(()) => {
            set_download_phase(&version, ProductUpdateCheck::Downloaded(version.clone())).await
        }
        Err(error) => {
            warn!("Signed MSI staging/verification failed: {error}");
            set_download_phase(&version, ProductUpdateCheck::Error).await;
        }
    }
}

// The cache below is NOT an executable/update authorization. It may be
// discarded at any time, and is always revalidated. The eventual privileged
// MSI runner must open the staged file securely and repeat all checks.
#[cfg(target_os = "windows")]
const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[cfg(target_os = "windows")]
fn reject_reparse_points(path: &Path) -> std::io::Result<()> {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    for component in path.ancestors() {
        let metadata = std::fs::symlink_metadata(component)?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(std::io::Error::other(
                "update staging path contains a reparse point",
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn verify_staged_hash(
    path: PathBuf,
    release: VerifiedRelease,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::task::spawn_blocking(move || {
        reject_reparse_points(&path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_PACKAGE_BYTES {
            return Err(UpdateError::PackageTooLarge);
        }
        let file = std::fs::File::open(&path)?;
        release.verify_package(file)
    })
    .await??;
    Ok(())
}

#[cfg(target_os = "windows")]
async fn stage_verified_msi(
    release: &VerifiedRelease,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let staging = crate::settings::Settings::get_app_data_dir().join("updates");
    tokio::fs::create_dir_all(&staging).await?;
    reject_reparse_points(&staging)?;

    // The MSI creates the ProgramData parent with SYSTEM/Admin-only writes.
    // This is a cache, not a trusted executable location. Never use a path
    // from the manifest as a local filename.
    let base = format!(
        "AceBlocker-{}-{}",
        release.version(),
        &release.metadata().sha256[..16]
    );
    let final_path = staging.join(format!("{base}.msi"));
    let partial_path = staging.join(format!("{base}.partial"));
    if final_path.exists() {
        verify_staged_hash(final_path.clone(), release.clone()).await?;
        return Ok(final_path);
    }
    // A previous crash might leave a .partial from this exact manifest.
    if partial_path.exists() {
        reject_reparse_points(&partial_path)?;
        tokio::fs::remove_file(&partial_path).await?;
    }

    let download_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
        let mut output = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial_path)
            .await?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20 * 60))
            .build()?;
        let mut response = client
            .get(&release.metadata().package_url)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_PACKAGE_BYTES)
        {
            return Err(Box::new(UpdateError::PackageTooLarge));
        }
        let mut total_bytes = 0u64;
        while let Some(chunk) = response.chunk().await? {
            total_bytes = total_bytes
                .checked_add(chunk.len() as u64)
                .filter(|size| *size <= MAX_PACKAGE_BYTES)
                .ok_or(UpdateError::PackageTooLarge)?;
            output.write_all(&chunk).await?;
        }
        output.flush().await?;
        output.sync_all().await?;
        drop(output);
        verify_staged_hash(partial_path.clone(), release.clone()).await?;
        // Creation is exclusive and the cache is rehashed on every use.
        tokio::fs::rename(&partial_path, &final_path).await?;
        Ok(())
    }
    .await;
    if download_result.is_err() {
        let _ = tokio::fs::remove_file(&partial_path).await;
    }
    download_result?;
    reject_reparse_points(&final_path)?;
    Ok(final_path)
}

/// A distinct Authenticode check on the MSI before advertising it as staged.
/// Uses the MSI-installed, administrator-protected script and pinned signer
/// certificate from the signed manifest (already checked against build pin).
#[cfg(target_os = "windows")]
async fn verify_authenticode(
    path: &Path,
    release: &VerifiedRelease,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    reject_reparse_points(path)?;
    let script = crate::settings::Settings::get_install_dir().join("verify-product-signature.ps1");
    reject_reparse_points(&script)?;
    let root = std::env::var_os("SystemRoot").ok_or(UpdateError::NotConfigured)?;
    let powershell = PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let mut command = tokio::process::Command::new(powershell);
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .arg("-MsiPath")
        .arg(path)
        .arg("-ExpectedPublisherCertSha256")
        .arg(&release.metadata().publisher_certificate_sha256)
        .kill_on_drop(true);
    let status = tokio::time::timeout(Duration::from_secs(60), command.status()).await??;
    if !status.success() {
        return Err(Box::new(UpdateError::InvalidSignature));
    }
    // Hash the SAME cached package again after Authenticode verified it.
    // Reverification at execution time remains mandatory.
    verify_staged_hash(path.to_path_buf(), release.clone()).await?;
    Ok(())
}

/// A separate low-frequency discovery task; no TrustedClock or content
/// updater state is touched. Failed fetches exponentially back off.
#[cfg(target_os = "windows")]
pub async fn run_periodic_checks() {
    if MANIFEST_URL.is_none() || TrustPolicy::embedded().is_err() {
        return;
    }
    let mut consecutive_errors = 0u32;
    loop {
        let status = check_available().await;
        if matches!(status, ProductUpdateCheck::Error) {
            consecutive_errors = (consecutive_errors + 1).min(2);
        } else {
            consecutive_errors = 0;
        }
        // Cadence: 6h / 12h / 24h plus up to 30 minutes of jitter.
        // System time here is ONLY scheduling noise, never trusted lock time.
        let noise = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64
            % 1800;
        let hours = 6u64 * (1u64 << consecutive_errors);
        tokio::time::sleep(Duration::from_secs(hours * 3600 + noise)).await;
    }
}

async fn fetch_verified_release()
-> Result<Option<ace_update_security::VerifiedRelease>, Box<dyn std::error::Error + Send + Sync>> {
    let Some(url) = MANIFEST_URL else {
        return Err(Box::new(UpdateError::NotConfigured));
    };
    let policy = TrustPolicy::embedded()?;
    let parsed = reqwest::Url::parse(url)?;
    if parsed.scheme() != "https"
        || !matches!(
            parsed.host_str(),
            Some("aceblocker.com") | Some("www.aceblocker.com")
        )
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err(Box::new(UpdateError::UnsafeUrl));
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(6))
        .timeout(TIMEOUT)
        .build()?;
    let mut response = client.get(parsed).send().await?.error_for_status()?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > MAX_MANIFEST_BYTES.saturating_sub(body.len()) {
            return Err(Box::new(UpdateError::Malformed));
        }
        body.extend_from_slice(&chunk);
    }
    let installed = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let release = match verify_release(&body, &policy, &installed, &installed) {
        Ok(release) => release,
        Err(UpdateError::NotNewer) => return Ok(None),
        Err(error) => return Err(Box::new(error)),
    };
    // This is only a verified announcement, NOT a verified MSI.
    // Download and OS Authenticode verification are handled asynchronously
    // in the Windows service, with no interruption to filtering.
    Ok(Some(release))
}
