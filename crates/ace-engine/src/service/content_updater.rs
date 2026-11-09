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
        ApiKey, RemoteSitePackId, Sha256Digest, SitePackId, SitePackVersion,
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
use std::collections::HashSet;
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
    candidate.is_newer_than(current)
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

#[derive(Clone)]
pub struct ContentUpdateService {
    db: Arc<EngineDB>,
    latest_releases_info: Option<LatestReleasesInfo>,
    latest_check: Option<DateTime<Utc>>,
    /// Serializes full content transitions independently of the metadata mutex.
    update_gate: Arc<Mutex<()>>,
}

impl ContentUpdateService {
    /// Snapshot state without retaining the shared metadata mutex during I/O.
    pub(crate) fn detached(&self) -> (Self, Arc<Mutex<()>>) {
        (self.clone(), self.update_gate.clone())
    }

    /// Publish metadata from a completed pass while the single-flight gate is held.
    pub(crate) fn publish_metadata_from(&mut self, completed: &Self) {
        self.latest_releases_info = completed.latest_releases_info.clone();
        self.latest_check = completed.latest_check;
    }

    /// Constructs the service. Cannot fail (LIF-03): the persisted
    /// "last check" timestamp is loaded on the first check, not at startup,
    /// so failure propagates to the update caller without aborting engine startup.
    pub fn from(db: Arc<EngineDB>) -> Self {
        ContentUpdateService {
            db,
            latest_releases_info: None,
            latest_check: None,
            update_gate: Arc::new(Mutex::new(())),
        }
    }

    fn load_latest_check(&mut self) -> Result<()> {
        let settings = self.db.get_settings()?;
        self.latest_check = settings
            .latest_update_check_at
            .map(|at| DateTime::parse_from_rfc3339(&at).map(|date| date.with_timezone(&Utc)))
            .transpose()?;
        Ok(())
    }

    pub async fn requires_update_check(&mut self) -> Result<bool> {
        self.load_latest_check()?;
        Ok(match self.latest_check {
            None => true,
            Some(last_check) => Utc::now() - last_check > UPDATE_FETCH_THRESHOLD,
        })
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
        if self.latest_releases_info.is_none() || self.requires_update_check().await? {
            self.check_for_updates().await?;
        }
        Ok(self
            .latest_releases_info
            .clone()
            .expect("Should be some after check_for_updates"))
    }

    pub async fn resistance_scripts_require_update(&mut self) -> Result<bool, SitePackUpdateError> {
        let latest = self.get_latest_info().await.map_err(|error| {
            SitePackUpdateError::FetchError(format!("Cannot fetch resistance metadata: {error}"))
        })?;
        let Some(remote) = latest.resistance else {
            return Ok(false);
        };
        let current = self.db.get_resistance_mode_settings().map_err(|error| {
            SitePackUpdateError::UpdateError(format!("Cannot read resistance settings: {error}"))
        })?;
        let scripts_dir = crate::settings::Settings::get_resistance_scripts_dir();
        let version_mismatch = current.scripts_version.as_ref() != Some(&remote.version);
        let files_missing = !scripts_dir.exists();
        Ok(version_mismatch || files_missing)
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
        let scripts_need_update = if is_pro {
            self.resistance_scripts_require_update().await?
        } else {
            false
        };
        if scripts_need_update {
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
        let latest = self.get_latest_info().await.map_err(|error| {
            SitePackUpdateError::FetchError(format!("Failed to fetch site-pack metadata: {error}"))
        })?;

        // A restored or seeded block can reference a remote pack with version
        // metadata but no local data. Consider every network block, even
        // disabled ones, and de-duplicate packs shared by multiple blocks.
        let attached_pack_ids: HashSet<String> = self
            .db
            .get_network_blocks_to_site_packs()
            .map_err(|error| {
                SitePackUpdateError::FetchError(format!(
                    "Failed to load network-block site-pack assignments: {error}"
                ))
            })?
            .into_values()
            .flatten()
            .collect();

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

            let local_pack = self
                .db
                .find_site_pack_by_remote_id(&remote_id)
                .map_err(|error| {
                    SitePackUpdateError::FetchError(format!(
                        "Failed to look up site pack {remote_id}: {error}"
                    ))
                })?;
            match local_pack {
                Some(local_model) => {
                    let local_id =
                        SitePackId::try_new(local_model.id.clone()).map_err(|error| {
                            SitePackUpdateError::UpdateError(format!(
                                "invalid persisted site-pack ID for {remote_id}: {error}"
                            ))
                        })?;
                    let attached_but_missing = attached_pack_ids.contains(&local_model.id)
                        && !Settings::get_site_pack_path(&local_id)
                            .join("pack.db")
                            .is_file();
                    let current_version = local_model
                        .version
                        .map(SitePackVersion::try_new)
                        .transpose()
                        .map_err(|error| {
                            SitePackUpdateError::UpdateError(format!(
                                "invalid persisted site-pack version for {remote_id}: {error}"
                            ))
                        })?;
                    let needs_update = attached_but_missing
                        || current_version
                            .as_ref()
                            .map(|current| version_is_newer(&candidate_version, current))
                            .unwrap_or(true);
                    if needs_update {
                        info!(
                            "Site list {} needs update (attached but missing: {}): {} -> {}",
                            remote_id,
                            attached_but_missing,
                            current_version
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "<none>".to_string()),
                            candidate_version
                        );
                        updates_needed.push((remote_id, candidate_version));
                    }
                }
                None => {
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
        let latest = self.get_latest_info().await.map_err(|error| {
            SitePackUpdateError::FetchError(format!("Failed to fetch releases metadata: {error}"))
        })?;

        let site_packs_info = latest.site_packs;

        // Get list of ignored remote IDs
        let ignored_ids: std::collections::HashSet<RemoteSitePackId> = self
            .db
            .get_ignored_remote_site_packs()
            .map_err(|error| {
                SitePackUpdateError::UpdateError(format!("Cannot load ignored site packs: {error}"))
            })?
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
    pub fn db_get_site_pack_version(
        &self,
        remote_id: &str,
    ) -> Result<Option<String>, SitePackUpdateError> {
        self.db
            .find_site_pack_by_remote_id(remote_id)
            .map(|model| model.and_then(|pack| pack.version))
            .map_err(|error| {
                SitePackUpdateError::FetchError(format!(
                    "Failed to read local site-pack version: {error}"
                ))
            })
    }

    pub async fn get_latest_installer_version(
        &mut self,
    ) -> Result<Option<String>, SitePackUpdateError> {
        let latest = self.get_latest_info().await.map_err(|error| {
            SitePackUpdateError::FetchError(format!("Cannot fetch installer metadata: {error}"))
        })?;
        Ok(latest.installer.map(|installer| installer.version))
    }
}

/// Site list update errors
#[derive(Debug, thiserror::Error)]
pub enum SitePackUpdateError {
    #[error("Failed to fetch site list info: {0}")]
    FetchError(String),
    /// Release discovery is unavailable, but installed content can still be used.
    #[error("Release metadata temporarily unavailable: {0}")]
    ReleaseUnavailable(String),
    #[error("Failed to update site list: {0}")]
    UpdateError(String),
}

/// Distinguish temporary server/transport outages from local DB failures and
/// malformed responses, which must still be reported as internal errors.
fn classify_release_check_error(error: eyre::Report) -> SitePackUpdateError {
    use crate::ace_server_api::ApiOpError;

    let retryable = error
        .downcast_ref::<ApiOpError<serde_json::Value>>()
        .is_some_and(|error| match error {
            ApiOpError::Transport(transport) => transport.is_retryable(),
            ApiOpError::Api(response) => response.is_retryable(),
        });

    if retryable {
        SitePackUpdateError::ReleaseUnavailable(error.to_string())
    } else {
        SitePackUpdateError::FetchError(format!("update check failed: {error}"))
    }
}

/// Convenience type for a shareable [`ContentUpdateService`] behind a `tokio` mutex.
pub type SharedContentUpdateService = Arc<Mutex<ContentUpdateService>>;

/// Convenience type for a shareable update-check interval.
pub type SharedUpdateInterval = Arc<RwLock<Duration>>;

/// Runs one full content-update pass: fetch metadata (network), stage
/// downloads/extractions (network, no manager lock), then commit under the
/// manager write lock (local disk swaps + DB/memory transitions only).
pub(crate) async fn run_update_pass(
    content_service: &SharedContentUpdateService,
    network_manager: &Arc<RwLock<NetworkManager>>,
    account_manager: &crate::manager::AccountManager,
) -> Result<(), SitePackUpdateError> {
    let (mut service, gate) = content_service.lock().await.detached();
    let _single_flight = gate.lock().await;

    // 1. Fetch fresh metadata — no manager lock.
    service
        .check_for_updates()
        .await
        .map_err(classify_release_check_error)?;

    // Publish fetched metadata immediately: a later staging failure must not
    // strand the shared cache on an obsolete release after the DB timestamp advanced.
    content_service.lock().await.publish_metadata_from(&service);

    // 2. Stage all pending downloads/extractions — no manager lock.
    // Free simulation must also strip the real Pro API key from requests to
    // Ace Server. The persisted account and credential remain unchanged.
    let send_credentials = network_manager.read().await.sends_registered_credentials();
    let api_key = if send_credentials {
        account_manager
            .get_api_key()
            .await
            .map_err(SitePackUpdateError::FetchError)?
    } else {
        None
    };
    let is_pro = send_credentials && account_manager.is_pro().await;
    let staged = service
        .stage_pending_updates(api_key.as_ref(), is_pro)
        .await?;

    // 3. Commit under the manager write lock.
    let mut manager = network_manager.write().await;
    let commit_result = manager.apply_staged_content_updates(staged).await;

    // Register any new remote site lists that appeared since last check
    // (metadata + DB only; the releases info is fresh from step 1, so this
    // performs no network I/O).
    let register_result = service
        .fetch_and_register_remote_site_packs(&mut manager)
        .await;
    drop(manager);

    // Metadata is the only shared mutable state: publish it quickly after I/O.
    content_service.lock().await.publish_metadata_from(&service);
    commit_result.map_err(|error| SitePackUpdateError::UpdateError(error.to_string()))?;
    register_result.map_err(|error| {
        SitePackUpdateError::UpdateError(format!("Failed to register remote site packs: {error}"))
    })?;
    Ok(())
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
    fn release_check_distinguishes_offline_from_internal_errors() {
        use crate::ace_server_api::{ApiOpError, HttpError};

        let offline = eyre::Report::new(ApiOpError::<serde_json::Value>::Transport(
            HttpError::Timeout,
        ));
        assert!(matches!(
            classify_release_check_error(offline),
            SitePackUpdateError::ReleaseUnavailable(_)
        ));

        let unavailable = eyre::Report::new(ApiOpError::<serde_json::Value>::Api(
            crate::ace_server_api::ApiError {
                status: 503,
                headers: reqwest::header::HeaderMap::new(),
                body: String::new(),
                raw_body: vec![],
                typed: None,
                parse_error: None,
            },
        ));
        assert!(matches!(
            classify_release_check_error(unavailable),
            SitePackUpdateError::ReleaseUnavailable(_)
        ));

        let internal = eyre::eyre!("database failed to save last update timestamp");
        assert!(matches!(
            classify_release_check_error(internal),
            SitePackUpdateError::FetchError(_)
        ));
    }

    #[tokio::test]
    async fn connected_missing_pack_is_selected_once_even_when_version_matches() {
        Settings::set_test_sites_dir(Settings::shared_test_sites_dir());
        let temp = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&temp.path().join("updates.sqlite")).unwrap());
        let connected_id = format!("connected-{}", uuid::Uuid::new_v4());
        let unconnected_id = format!("unconnected-{}", uuid::Uuid::new_v4());
        for remote_id in [&connected_id, &unconnected_id] {
            db.add_default_site_pack(remote_id, remote_id, "Test", None, false, "1.0.0")
                .unwrap();
        }
        let block = db.add_network_block("First".to_string()).unwrap();
        let block_id = uuid::Uuid::parse_str(&block.id).unwrap();
        db.add_network_block_to_site_pack(&block_id, &connected_id)
            .unwrap();
        let other_block = db.add_network_block("Disabled".to_string()).unwrap();
        let other_block_id = uuid::Uuid::parse_str(&other_block.id).unwrap();
        db.set_network_block_enabled(&other_block_id, false)
            .unwrap();
        db.add_network_block_to_site_pack(&other_block_id, &connected_id)
            .unwrap();

        // Use local release metadata; the regression must not contact the server.
        let remote_packs: Vec<_> = [&connected_id, &unconnected_id]
            .iter()
            .map(|id| {
                serde_json::json!({
                    "id": id, "name": "Test", "description": "",
                    "requiresPro": false, "size": 1, "updatedAt": null,
                    "version": "1.0.0"
                })
            })
            .collect();
        let release = serde_json::from_value(serde_json::json!({
            "installer": null,
            "resistance": null,
            "sitePacks": remote_packs
        }))
        .unwrap();
        db.update_latest_update_check_at(Utc::now().to_rfc3339())
            .unwrap();
        let mut service = ContentUpdateService::from(db);
        service.latest_releases_info = Some(release);

        let connected_pack_id = SitePackId::try_new(connected_id.clone()).unwrap();
        let path = Settings::get_site_pack_path(&connected_pack_id);
        std::fs::create_dir_all(&path).unwrap();
        // The directory alone must not count as a completed download.
        let pending = service.get_site_packs_to_update().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0.as_ref(), connected_id);

        // Once the database exists, a same-version pass should not re-download.
        SitePackDB::from(&path.join("pack.db")).unwrap();
        assert!(service.get_site_packs_to_update().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn content_update_gate_serializes_passes_without_locking_shared_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&temp.path().join("singleflight.sqlite")).unwrap());
        let shared = Arc::new(Mutex::new(ContentUpdateService::from(db)));
        let (mut detached, gate) = shared.lock().await.detached();
        let (_, other_gate) = shared.lock().await.detached();
        assert!(Arc::ptr_eq(&gate, &other_gate));

        let guard = gate.lock().await;
        assert!(
            other_gate.try_lock().is_err(),
            "another update may not run concurrently"
        );
        assert!(
            shared.try_lock().is_ok(),
            "metadata access must stay responsive"
        );

        detached.latest_check = Some(Utc::now());
        shared.lock().await.publish_metadata_from(&detached);
        assert_eq!(shared.lock().await.latest_check, detached.latest_check);
        drop(guard);
        assert!(other_gate.try_lock().is_ok());
    }

    #[tokio::test]
    async fn update_history_load_reports_invalid_timestamp_and_database_failures() {
        use diesel::connection::SimpleConnection;

        let temp = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&temp.path().join("history.sqlite")).unwrap());
        let mut service = ContentUpdateService::from(db.clone());
        assert!(service.requires_update_check().await.unwrap());

        let mut connection = db.get_db_connection().unwrap();
        connection
            .batch_execute("UPDATE settings SET latest_update_check_at = 'invalid-timestamp';")
            .unwrap();
        drop(connection);
        assert!(service.requires_update_check().await.is_err());

        let mut connection = db.get_db_connection().unwrap();
        connection.batch_execute("DROP TABLE settings;").unwrap();
        drop(connection);
        assert!(service.requires_update_check().await.is_err());
    }

    #[test]
    fn version_is_newer_compares_dotted_numbers() {
        let is_newer = |candidate: &str, current: &str| {
            let candidate = SitePackVersion::try_new(candidate.to_string()).unwrap();
            let current = SitePackVersion::try_new(current.to_string()).unwrap();
            version_is_newer(&candidate, &current)
        };

        assert!(is_newer("1.10.0", "1.9.0"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("1.9.0", "1.10.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "2.0.0"));
        assert!(is_newer("2026.09.01", "2.1.0"));
        assert!(is_newer("2026.10.01", "2026.09.30"));
    }

    #[test]
    fn local_pack_version_lookup_distinguishes_absence_from_database_error() {
        use diesel::connection::SimpleConnection;

        let temp = tempfile::tempdir().unwrap();
        let db = Arc::new(EngineDB::from(&temp.path().join("content.sqlite")).unwrap());
        let service = ContentUpdateService::from(db.clone());
        assert_eq!(service.db_get_site_pack_version("absent").unwrap(), None);

        let mut connection = db.get_db_connection().unwrap();
        connection.batch_execute("DROP TABLE site_packs").unwrap();
        drop(connection);
        assert!(service.db_get_site_pack_version("absent").is_err());
    }
}
