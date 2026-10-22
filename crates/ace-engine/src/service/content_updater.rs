//! Content update service.
//!
//! The [`ContentUpdateService`] is responsible for fetching the latest
//! release metadata from the Ace Blocker server (`GET /api/v1/releases/latest`),
//! comparing local versions against it, and determining which content assets
//! (site lists, resistance scripts) need to be updated. It delegates the
//! actual application of updates to [`crate::manager::NetworkManager`].
//! (Engine self-update machinery was removed as unwired dead code — SIMP-02.)
//!
//! # Lock discipline (see architecture-review-2026-09.md SVC-02)
//!
//! Network I/O — metadata fetches, downloads, hash verification, and archive
//! extraction — happens in the **staging** phase, which never touches the
//! [`crate::manager::NetworkManager`]. The staged artifacts are then
//! committed under the manager write lock by
//! [`NetworkManager::apply_staged_content_updates`], which only does local
//! disk swaps, validation, and DB/memory state transitions. A hung server
//! can therefore never stall the proxy or the API.

use crate::{
    ace_server_api::GetLatestReleasesVersionDataResponse,
    core::{
        ApiKey, RemoteSitePackId, Sha256Digest, SitePackVersion,
        network_block::site_pack::{site_pack_db::SitePackDB, stage_backup_archive},
        utils::file_download,
    },
    db::EngineDB,
    manager::{NetworkManager, NetworkManagerOpErr},
    service::clients::AceServerClient,
    settings::Settings,
};
use chrono::{DateTime, Utc};
use eyre::Result;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

type LatestReleasesInfo = GetLatestReleasesVersionDataResponse;

const UPDATE_FETCH_THRESHOLD: chrono::Duration = chrono::Duration::days(1);

/// Default interval between automatic background update checks.
pub const DEFAULT_UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Maximum number of download retries when hash verification fails.
const MAX_DOWNLOAD_RETRIES: u8 = 3;

/// Compares two validated site-pack versions using the domain comparison policy.
pub fn version_is_newer(candidate: &SitePackVersion, current: &SitePackVersion) -> bool {
    (candidate.is_bundle_style() && !current.is_bundle_style()) || candidate.is_newer_than(current)
}

/// A staged site-pack update: the new pack database has been downloaded and
/// extracted into `staging_dir`; the downloaded archive is kept only so it is
/// removed when the staging object is dropped. Committing moves `staging_dir`
/// into place with an atomic swap.
pub struct StagedSitePack {
    pub remote_id: RemoteSitePackId,
    pub new_version: SitePackVersion,
    /// Path of the downloaded ZIP archive (removed on drop).
    pub archive_path: PathBuf,
    /// Directory containing the validated `pack.db` (removed on drop).
    pub staging_dir: PathBuf,
}

impl Drop for StagedSitePack {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.archive_path);
        let _ = std::fs::remove_dir_all(&self.staging_dir);
    }
}

/// A staged resistance-scripts bundle: downloaded, SHA-256 verified, and
/// extracted into `staging_dir` (which is removed when this is dropped).
pub struct StagedScriptBundle {
    pub version: String,
    pub staging_dir: PathBuf,
}

impl Drop for StagedScriptBundle {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.staging_dir);
    }
}

/// Everything staged and ready to be committed under the manager write lock.
pub struct StagedContentUpdate {
    pub site_packs: Vec<StagedSitePack>,
    pub resistance_scripts: Option<StagedScriptBundle>,
    /// Non-fatal errors collected while staging (error.g. one pack failed to
    /// download). The pass still commits what succeeded, but the failures
    /// are reported to the caller so it can surface a partial failure.
    pub staging_errors: Vec<String>,
}

pub struct ContentUpdateService {
    db: Arc<EngineDB>,
    latest_releases_info: Option<LatestReleasesInfo>,
    latest_check: Option<DateTime<Utc>>,
}

impl ContentUpdateService {
    /// Constructs the service. Cannot fail (LIF-03): the persisted
    /// "last check" timestamp is loaded best-effort so a DB hiccup can never
    /// prevent the engine from starting.
    pub fn from(db: Arc<EngineDB>) -> Self {
        let mut service = ContentUpdateService {
            db,
            latest_releases_info: None,
            latest_check: None,
        };

        service.load_latest_check();

        service
    }

    fn load_latest_check(&mut self) {
        if let Ok(settings) = self.db.get_settings()
            && let Some(check_at) = settings.latest_update_check_at
            && let Ok(dt) = DateTime::parse_from_rfc3339(&check_at)
        {
            self.latest_check = Some(dt.with_timezone(&Utc));
        }
    }

    pub async fn requires_update_check(&mut self) -> bool {
        self.load_latest_check();

        match self.latest_check {
            None => true,
            Some(last_check) => Utc::now() - last_check > UPDATE_FETCH_THRESHOLD,
        }
    }

    pub async fn check_for_updates(&mut self) -> Result<()> {
        info!("Checking for updates...");
        let client = AceServerClient::new_http_client(None);
        let info = client.get_latest_releases_version_data().await?;

        self.latest_releases_info = Some(info);
        let now = Utc::now();
        self.latest_check = Some(now);

        self.db.update_latest_update_check_at(now.to_rfc3339())?;

        Ok(())
    }

    async fn get_latest_info(&mut self) -> Result<LatestReleasesInfo> {
        if self.latest_releases_info.is_none() || self.requires_update_check().await {
            self.check_for_updates().await?;
        }
        Ok(self
            .latest_releases_info
            .clone()
            .expect("Should be some after check_for_updates"))
    }

    pub async fn resistance_scripts_require_update(&mut self) -> bool {
        let latest = match self.get_latest_info().await {
            Ok(info) => info,
            Err(error) => {
                warn!("Failed to get latest info: {}", error);
                return false;
            }
        };

        let Some(remote) = latest.resistance else {
            return false;
        };

        let current = match self.db.get_resistance_mode_settings() {
            Ok(s) => s,
            Err(error) => {
                warn!("Failed to get current resistance settings: {}", error);
                return false;
            }
        };

        // Check if version is different OR if the directory doesn't exist
        let scripts_dir = crate::settings::Settings::get_resistance_scripts_dir();
        let version_mismatch = current.scripts_version.as_ref() != Some(&remote.version);
        let files_missing = !scripts_dir.exists();

        version_mismatch || files_missing
    }

    /// Stages every pending content update (site packs + resistance scripts)
    /// **without** holding the NetworkManager lock: metadata fetches,
    /// downloads, hash verification, and archive extraction all happen here
    /// against temp locations. Returns an error only for catastrophic
    /// failures; per-item failures are recorded in
    /// [`StagedContentUpdate::staging_errors`] and committed what succeeded.
    pub async fn stage_pending_updates(
        &mut self,
        api_key: Option<&ApiKey>,
        is_pro: bool,
    ) -> Result<StagedContentUpdate, SitePackUpdateError> {
        let mut staged = StagedContentUpdate {
            site_packs: Vec::new(),
            resistance_scripts: None,
            staging_errors: Vec::new(),
        };

        // ── Site packs ────────────────────────────────────────────────
        match self.get_site_packs_to_update().await {
            Ok(updates) => {
                for (remote_id, new_version) in updates {
                    match self
                        .stage_site_pack_backup(&remote_id, &new_version, api_key)
                        .await
                    {
                        Ok(site_pack) => staged.site_packs.push(site_pack),
                        Err(error) => {
                            warn!("Failed to stage site list {} update: {}", remote_id, error);
                            staged
                                .staging_errors
                                .push(format!("site list {remote_id}: {error}"));
                        }
                    }
                }
            }
            Err(error) => {
                warn!("Failed to check site lists for updates: {}", error);
                staged.staging_errors.push(format!("update check: {error}"));
            }
        }

        // ── Resistance scripts (Pro only) ─────────────────────────────
        if is_pro && self.resistance_scripts_require_update().await {
            match self.stage_resistance_scripts(api_key).await {
                Ok(bundle) => staged.resistance_scripts = Some(bundle),
                Err(error) => {
                    warn!("Failed to stage resistance scripts: {}", error);
                    staged
                        .staging_errors
                        .push(format!("resistance scripts: {error}"));
                }
            }
        }

        Ok(staged)
    }

    /// Downloads, validates, and stages one remote site pack backup.
    async fn stage_site_pack_backup(
        &self,
        remote_id: &RemoteSitePackId,
        new_version: &SitePackVersion,
        api_key: Option<&ApiKey>,
    ) -> Result<StagedSitePack, SitePackUpdateError> {
        let client = AceServerClient::new_http_client(api_key);
        let response = client
            .get_site_pack_backup(remote_id)
            .await
            .map_err(|error| {
                SitePackUpdateError::FetchError(format!("failed to get site list backup: {error}"))
            })?;
        let url = response.url;

        let unique = uuid::Uuid::new_v4();
        let archive_path =
            std::env::temp_dir().join(format!("ace-site-pack-{remote_id}-{unique}.zip"));
        let staging_dir = Settings::get_site_packs_dir().join(format!(".staging-{unique}"));

        let download = file_download::download_file_with_limit(
            &url,
            archive_path.clone(),
            file_download::DEFAULT_MAX_DOWNLOAD_BYTES,
        );
        futures::pin_mut!(download);
        while let Some(result) = download.next().await {
            if let Err(error) = result {
                let _ = std::fs::remove_file(&archive_path);
                return Err(SitePackUpdateError::UpdateError(format!(
                    "failed to download site list backup: {error}"
                )));
            }
        }

        let staged_db = (|| -> Result<PathBuf, SitePackUpdateError> {
            let path = stage_backup_archive(&archive_path, &staging_dir)
                .map_err(|error| SitePackUpdateError::UpdateError(error.to_string()))?;
            let db = SitePackDB::from(&path)
                .map_err(|error| SitePackUpdateError::UpdateError(error.to_string()))?;
            let count = db
                .count_entries()
                .map_err(|error| SitePackUpdateError::UpdateError(error.to_string()))?;
            if count == 0 {
                return Err(SitePackUpdateError::UpdateError(
                    "site list backup contains an empty database".to_string(),
                ));
            }
            Ok(path)
        })();
        if let Err(error) = staged_db {
            let _ = std::fs::remove_file(&archive_path);
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(error);
        }

        info!(
            "Staged site list {} update to version {}",
            remote_id, new_version
        );
        Ok(StagedSitePack {
            remote_id: remote_id.clone(),
            new_version: new_version.clone(),
            archive_path,
            staging_dir,
        })
    }

    /// Downloads, verifies, and stages the resistance-scripts bundle.
    ///
    /// The bundle is downloaded into memory (SHA-256 verified), extracted
    /// with traversal protection into a staging directory on the same volume
    /// as the live scripts directory, and checked for its `scripts-data.json`
    /// catalog so the commit's reload cannot fail on a truncated bundle.
    pub async fn stage_resistance_scripts(
        &self,
        api_key: Option<&ApiKey>,
    ) -> Result<StagedScriptBundle, NetworkManagerOpErr> {
        let api_key = api_key.ok_or_else(|| NetworkManagerOpErr::ProStatusCheckFailed {
            message: "No API key found".to_string(),
        })?;

        let client = AceServerClient::new_http_client(Some(api_key));

        let scripts_response = client
            .get_resistance_mode_scripts()
            .await
            .map_err(|error| {
                warn!(
                    "Failed to download resistance scripts: {}",
                    error.to_string()
                );
                // 401/403 from the server mean the API key is invalid or the
                // account is not Pro — an entitlement failure, not a
                // transport or CDN problem. Keep those two classes
                // distinguishable.
                let is_entitlement_failure = matches!(
                    &error,
                    crate::ace_server_api::ApiOpError::Api(api_err)
                        if api_err.status == 401 || api_err.status == 403
                );
                if is_entitlement_failure {
                    NetworkManagerOpErr::ProStatusCheckFailed {
                        message: error.to_string(),
                    }
                } else {
                    NetworkManagerOpErr::ScriptDownloadFailed {
                        message: error.to_string(),
                    }
                }
            })?;

        let scripts_url = scripts_response.url.clone();
        let expected_sha256 = Sha256Digest::try_new(scripts_response.sha256).map_err(|error| {
            NetworkManagerOpErr::ScriptDownloadFailed {
                message: format!("server returned an invalid SHA-256 digest: {error}"),
            }
        })?;

        info!(
            "Downloading resistance scripts from: {} (expected SHA-256: {})",
            scripts_url, expected_sha256
        );

        let bytes = file_download::download_and_verify(
            &scripts_url,
            &expected_sha256,
            "resistance scripts",
            MAX_DOWNLOAD_RETRIES,
            file_download::SCRIPT_BUNDLE_MAX_BYTES,
        )
        .await
        .map_err(|error| NetworkManagerOpErr::ScriptDownloadFailed {
            message: error.to_string(),
        })?;

        let unique = uuid::Uuid::new_v4();
        let scripts_parent = Settings::get_resistance_scripts_dir()
            .parent()
            .ok_or_else(|| NetworkManagerOpErr::InternalError {
                message: "resistance scripts directory has no parent".to_string(),
            })?
            .to_path_buf();
        let staging_dir = scripts_parent.join(format!(".resistance_scripts.staging-{unique}"));

        file_download::extract_zip_safely(
            &bytes,
            &staging_dir,
            file_download::SCRIPT_BUNDLE_MAX_BYTES,
        )
        .map_err(|error| NetworkManagerOpErr::ScriptDownloadFailed {
            message: format!("Failed to extract scripts: {error}"),
        })?;

        // The catalog must be present, or the commit-time reload would
        // replace a working tree with nothing.
        if !staging_dir.join("scripts-data.json").exists() {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(NetworkManagerOpErr::ScriptDownloadFailed {
                message: "staged resistance scripts have no scripts-data.json catalog".to_string(),
            });
        }

        info!("Resistance scripts staged to: {:?}", staging_dir);

        Ok(StagedScriptBundle {
            version: scripts_response.version,
            staging_dir,
        })
    }

    /// Checks if any remote site lists need updates based on server version info.
    /// Returns typed `(remote_id, new_version)` pairs that need updating.
    pub async fn get_site_packs_to_update(
        &mut self,
    ) -> Result<Vec<(RemoteSitePackId, SitePackVersion)>, SitePackUpdateError> {
        let latest = match self.get_latest_info().await {
            Ok(info) => info,
            Err(error) => {
                warn!("Failed to get latest site lists info: {}", error);
                return Ok(Vec::new());
            }
        };

        let mut updates_needed = Vec::new();
        for remote_list in latest.site_packs {
            let remote_id = RemoteSitePackId::try_new(remote_list.id).map_err(|error| {
                SitePackUpdateError::UpdateError(format!("invalid remote site-pack ID: {error}"))
            })?;
            let candidate_version =
                SitePackVersion::try_new(remote_list.version).map_err(|error| {
                    SitePackUpdateError::UpdateError(format!(
                        "invalid remote site-pack version: {error}"
                    ))
                })?;

            match self.db.get_site_pack_by_remote_id(&remote_id) {
                Ok(local_model) => {
                    let current_version = local_model
                        .version
                        .map(SitePackVersion::try_new)
                        .transpose()
                        .map_err(|error| {
                            SitePackUpdateError::UpdateError(format!(
                                "invalid persisted site-pack version for {remote_id}: {error}"
                            ))
                        })?;
                    let needs_update = current_version
                        .as_ref()
                        .map(|current| version_is_newer(&candidate_version, current))
                        .unwrap_or(true);
                    if needs_update {
                        info!(
                            "Site list {} needs update: {} -> {}",
                            remote_id,
                            current_version
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "<none>".to_string()),
                            candidate_version
                        );
                        updates_needed.push((remote_id, candidate_version));
                    }
                }
                Err(_) => {
                    info!("Site list {} not found locally, will be added", remote_id);
                    updates_needed.push((remote_id, candidate_version));
                }
            }
        }

        Ok(updates_needed)
    }

    /// Fetches all available remote site lists from the server and adds them locally
    /// if they don't exist. Does not download the actual site list data.
    /// Skips site lists that have been marked as ignored by the user.
    /// Uses get_latest_releases_version_data for metadata.
    pub async fn fetch_and_register_remote_site_packs(
        &mut self,
        network_manager: &mut NetworkManager,
    ) -> Result<(), SitePackUpdateError> {
        info!("Fetching remote site lists metadata from server...");

        // Get the latest releases info which contains site lists metadata
        let latest = match self.get_latest_info().await {
            Ok(info) => info,
            Err(error) => {
                warn!("Failed to get latest releases info: {}", error);
                return Ok(());
            }
        };

        let site_packs_info = latest.site_packs;

        // Get list of ignored remote IDs
        let ignored_ids: std::collections::HashSet<RemoteSitePackId> = self
            .db
            .get_ignored_remote_site_packs()
            .unwrap_or_else(|error| {
                warn!(%error, "Ignored remote site-pack query failed; preserving existing fallback");
                Vec::new()
            })
            .into_iter()
            .filter_map(|model| model.remote_id)
            .map(RemoteSitePackId::try_new)
            .collect::<std::result::Result<_, _>>()
            .map_err(|error| {
                SitePackUpdateError::UpdateError(format!(
                    "invalid persisted remote site-pack ID: {error}"
                ))
            })?;

        for remote_list in site_packs_info {
            let remote_id = RemoteSitePackId::try_new(remote_list.id).map_err(|error| {
                SitePackUpdateError::UpdateError(format!("invalid remote site-pack ID: {error}"))
            })?;
            let version = SitePackVersion::try_new(remote_list.version).map_err(|error| {
                SitePackUpdateError::UpdateError(format!(
                    "invalid remote site-pack version: {error}"
                ))
            })?;

            if ignored_ids.contains(&remote_id) {
                info!("Site list {} is ignored, skipping", remote_id);
                continue;
            }

            match self.db.get_site_pack_by_remote_id(&remote_id) {
                Ok(model) => {
                    if model.is_ignored {
                        info!("Site list {} exists but is ignored, skipping", remote_id);
                        continue;
                    }
                    info!(
                        "Site list {} already exists locally (non-ignored), skipping",
                        remote_id
                    );
                    continue;
                }
                Err(_) => info!("Site list {} not found locally, adding...", remote_id),
            }

            info!(
                "Adding new remote site list: {} (version {})",
                remote_list.name, version
            );

            if let Err(error) = network_manager
                .add_remote_site_pack_from_updater(
                    &remote_id,
                    &remote_list.name,
                    Some(remote_list.description.clone()),
                    remote_list.requires_pro,
                    &version,
                )
                .await
            {
                warn!("Failed to add remote site list {}: {}", remote_id, error);
            }
        }

        info!("Finished fetching and registering remote site lists");
        Ok(())
    }

    /// Re-enables all ignored remote site lists and re-fetches their metadata.
    pub async fn re_enable_remote_site_packs(
        &mut self,
        network_manager: &mut NetworkManager,
    ) -> Result<(), SitePackUpdateError> {
        info!("Re-enabling all ignored remote site lists...");

        // Get all ignored remote site lists
        let ignored_lists = self.db.get_ignored_remote_site_packs().map_err(|error| {
            SitePackUpdateError::FetchError(format!("Failed to get ignored site lists: {}", error))
        })?;

        // Re-enable each one
        for model in &ignored_lists {
            // Set is_ignored to false in DB
            self.db
                .set_site_pack_ignored(&model.id, false)
                .map_err(|error| {
                    SitePackUpdateError::UpdateError(format!(
                        "Failed to re-enable site list: {}",
                        error
                    ))
                })?;

            info!("Re-enabled site list: {}", model.name);
        }

        // Re-fetch all remote site lists
        self.fetch_and_register_remote_site_packs(network_manager)
            .await?;

        info!("Finished re-enabling remote site lists");
        Ok(())
    }

    // NOTE: there is intentionally no `get_latest_resistance_version`
    // accessor. The resistance-scripts version is committed by
    // `NetworkManager::commit_resistance_scripts` (memory + SQLite) from
    // the staged bundle itself, only after verification/extraction/reload
    // succeed — the updater must not persist a version it never installed.

    /// Local version of a remote site pack, for honest status reporting
    /// (API-01): the update-check response must not fabricate empty versions.
    pub fn db_get_site_pack_version(&self, remote_id: &str) -> Option<String> {
        self.db
            .get_site_pack_by_remote_id(remote_id)
            .ok()
            .and_then(|model| model.version)
    }

    pub async fn get_latest_installer_version(&mut self) -> Option<String> {
        let latest = self.get_latest_info().await.ok()?;
        latest.installer.map(|i| i.version)
    }
}

/// Site list update errors
#[derive(Debug, thiserror::Error)]
pub enum SitePackUpdateError {
    #[error("Failed to fetch site list info: {0}")]
    FetchError(String),
    #[error("Failed to update site list: {0}")]
    UpdateError(String),
}

/// Convenience type for a shareable [`ContentUpdateService`] behind a `tokio` mutex.
pub type SharedContentUpdateService = Arc<Mutex<ContentUpdateService>>;

/// Convenience type for a shareable update-check interval.
pub type SharedUpdateInterval = Arc<RwLock<Duration>>;

/// Runs one full content-update pass: fetch metadata (network), stage
/// downloads/extractions (network, no manager lock), then commit under the
/// manager write lock (local disk swaps + DB/memory transitions only).
async fn run_update_pass(
    content_service: &SharedContentUpdateService,
    network_manager: &Arc<RwLock<NetworkManager>>,
    account_manager: &crate::manager::AccountManager,
) -> Result<(), SitePackUpdateError> {
    let mut service = content_service.lock().await;

    // 1. Fetch fresh metadata — no manager lock.
    service.check_for_updates().await.map_err(|error| {
        SitePackUpdateError::FetchError(format!("update check failed: {error}"))
    })?;

    // 2. Stage all pending downloads/extractions — no manager lock.
    let api_key = account_manager
        .get_api_key()
        .await
        .map_err(SitePackUpdateError::FetchError)?;
    let is_pro = account_manager.is_pro().await;
    let staged = service
        .stage_pending_updates(api_key.as_ref(), is_pro)
        .await?;

    // 3. Commit under the manager write lock.
    let mut manager = network_manager.write().await;
    let commit_result = manager.apply_staged_content_updates(staged).await;

    // Register any new remote site lists that appeared since last check
    // (metadata + DB only; the releases info is fresh from step 1, so this
    // performs no network I/O).
    if let Err(error) = service
        .fetch_and_register_remote_site_packs(&mut manager)
        .await
    {
        warn!("Failed to register new site lists: {}", error);
    }
    drop(manager);
    drop(service);

    commit_result
}

/// Runs a periodic background content-update check.
///
/// Does **not** block a thread — it uses [`tokio::time::sleep`] to register a
/// timer in the runtime's timer wheel, yielding the task back to the pool
/// between ticks.
///
/// The first pass runs **immediately** (SVC-15): content freshness must not
/// wait a full interval after a long offline period, and startup content
/// init is handled by this loop instead of blocking engine boot.
///
/// In **release** builds the interval is fixed at 24 hours.  In **debug**
/// builds the interval can be mutated at runtime via
/// the debug-only `setUpdateCheckInterval` remoc operation.
pub async fn run_background_update_loop(
    content_service: SharedContentUpdateService,
    network_manager: Arc<RwLock<NetworkManager>>,
    interval: SharedUpdateInterval,
    account_manager: crate::manager::AccountManager,
) {
    loop {
        info!("Running scheduled background content update check…");
        if let Err(error) =
            run_update_pass(&content_service, &network_manager, &account_manager).await
        {
            warn!("Background content update pass failed: {}", error);
        }

        let sleep_duration = *interval.read().await;
        info!(
            "Next background update check scheduled in {} seconds",
            sleep_duration.as_secs()
        );
        tokio::time::sleep(sleep_duration).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_newer_compares_dotted_numbers() {
        let is_newer = |candidate: &str, current: &str| {
            let candidate = SitePackVersion::try_new(candidate.to_string()).unwrap();
            let current = SitePackVersion::try_new(current.to_string()).unwrap();
            version_is_newer(&candidate, &current)
        };

        assert!(is_newer("1.10", "1.9"));
        assert!(is_newer("2.0", "1.9.9"));
        assert!(is_newer("1.0.1", "1.0"));
        assert!(!is_newer("1.9", "1.10"));
        assert!(!is_newer("1.0", "1.0"));
        assert!(!is_newer("1.0", "1.0.0"));
        assert!(is_newer("1.0.0", "1.0"));
        assert!(!is_newer("1.0", "2.0"));
        assert!(is_newer("2026.09.01", "pack-adult-free"));
        // Non-numeric segments fall back to a string comparison.
        assert!(is_newer("1.1-beta", "1.1-alpha"));
        assert!(!is_newer("1.1-alpha", "1.1-beta"));
    }
}
