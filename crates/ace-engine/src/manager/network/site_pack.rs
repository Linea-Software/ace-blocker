use async_stream::stream;
use std::{
    collections::HashSet,
    env::temp_dir,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
};

use super::{NetworkManager, error::NetworkManagerOpErr};
use crate::core::{BlockingResult, network_block::uri_hash::UriHash};
use crate::core::{
    RemoteSitePackId, SitePack, SitePackId, SitePackImportError, SitePackVersion,
    common::percentage::Percentage,
    importer::{
        SitePackFormat, SitePackImportStream, SitePackSource, SitePackSourceId, SitePackSourceType,
    },
    network_block::site_pack::{
        SitePackCreationStatus, importer::stream::SitePackImportPhase, site_pack_db::SitePackDB,
    },
    utils::file_download,
};
use crate::{service::clients::AceServerClient, settings::Settings};
use chrono::Local;
use moka::sync::Cache;

type BlockDecisionCache = Cache<UriHash, BlockingResult>;

fn invalidate_caches(caches: &[BlockDecisionCache]) {
    for cache in caches {
        cache.invalidate_all();
    }
}

use tracing::{error, info, warn};
use uuid::Uuid;

/// RAII token that marks a site pack as "sync in progress" in the manager's
/// shared set. Dropping the token (normal stream end, stream failure, or the
/// remoc import stream being dropped on client disconnect) releases the slot, so
/// a cancelled or replayed sync can never leave the pack permanently busy.
struct SitePackSyncGuard {
    in_flight: Arc<Mutex<HashSet<SitePackId>>>,
    site_pack_id: SitePackId,
}

impl Drop for SitePackSyncGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = self.in_flight.lock() {
            set.remove(&self.site_pack_id);
        }
    }
}

/// After a successful **manual** remote resync, reconciles the persisted
/// `version` with the server's current release version for `remote_id`.
///
/// Manual resyncs restore the latest backup without learning its version from
/// the backup endpoint, so without this step the content updater would see an
/// "older" local version and re-download the identical archive on its next
/// background pass (duplicate download). Best effort: when the releases list
/// cannot be fetched the previous version is kept — the next background pass
/// reconciles it anyway.
async fn sync_remote_pack_version(
    db: &crate::db::EngineDB,
    client: &crate::ace_server_api::HttpClient,
    remote_id: &RemoteSitePackId,
    site_pack_arc: &Arc<RwLock<SitePack>>,
) -> Result<(), String> {
    let releases = client
        .get_latest_releases_version_data()
        .await
        .map_err(|error| error.to_string())?;

    let Some(remote_entry) = releases
        .site_packs
        .iter()
        .find(|entry| entry.id == remote_id.as_ref())
    else {
        return Ok(());
    };

    let local_row = db
        .get_site_pack_by_remote_id(remote_id)
        .map_err(|error| error.to_string())?;
    let current_version = local_row
        .version
        .map(SitePackVersion::try_new)
        .transpose()
        .map_err(|error| format!("invalid local site-pack version: {error}"))?;
    let remote_version = SitePackVersion::try_new(remote_entry.version.clone())
        .map_err(|error| format!("invalid remote site-pack version: {error}"))?;

    if current_version
        .as_ref()
        .is_some_and(|current| !remote_version.is_newer_than(current))
    {
        return Ok(());
    }

    db.update_site_pack_version(&local_row.id, &remote_version)
        .map_err(|error| error.to_string())?;

    if let Ok(mut site_pack) = site_pack_arc.write() {
        site_pack.set_version(remote_version.clone());
    }

    info!(
        "Manual resync of site list {} bumped version {} -> {}",
        remote_id,
        current_version
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<none>".to_string()),
        remote_version
    );

    Ok(())
}

/// Delete unfinished staging output on import failure, cancellation or disconnect.
struct StagedPackCleanup(PathBuf);

impl Drop for StagedPackCleanup {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "failed to clean staged site pack {}: {error}",
                self.0.display()
            );
        }
    }
}

/// The sole local-import coordinator. Keep the live pack in service while its
/// replacement is parsed, then emit COMPLETED only after the database swap.
fn run_local_import(
    site_pack: Arc<RwLock<SitePack>>,
    sources: Vec<SitePackSource>,
    site_pack_id: SitePackId,
    main_db: Arc<crate::db::EngineDB>,
    sync_guard: SitePackSyncGuard,
    related_caches: Vec<BlockDecisionCache>,
) -> SitePackImportStream<'static> {
    Box::pin(stream! {
        let _sync_guard = sync_guard;
        let live_dir = site_pack.read().unwrap().get_db_path().clone();
        let stage_dir = live_dir.with_file_name(format!(
            ".{}.staging-{}", site_pack_id, Uuid::new_v4()
        ));
        let _stage_cleanup = StagedPackCleanup(stage_dir.clone());

        let staged_db = match SitePackDB::from_staging(&stage_dir.join("pack.db")) {
            Ok(db) => db,
            Err(error) => {
                yield SitePackCreationStatus::Failed {
                    error: error.to_string(),
                    phase: Some(SitePackImportPhase::Initializing),
                };
                return;
            }
        };

        let mut completed = false;
        for await status in crate::core::network_block::site_pack::Importer::import_sources(
            &staged_db,
            sources,
            stage_dir.clone(),
            site_pack_id.clone(),
        ) {
            match status {
                SitePackCreationStatus::Completed { .. } => completed = true,
                SitePackCreationStatus::Failed { .. } => {
                    yield status;
                    return;
                }
                other => yield other,
            }
        }

        if !completed {
            yield SitePackCreationStatus::Failed {
                error: "Import ended without a terminal event".to_string(),
                phase: Some(SitePackImportPhase::Finalizing),
            };
            return;
        }

        // The importer writes pack_meta last. Validate it and close every
        // staging connection before renaming the file on Windows.
        let validation = staged_db.get_entry_count().and_then(|_| {
            staged_db.finish_staging(&stage_dir.join("pack.db"))
        });
        drop(staged_db);
        if let Err(error) = validation {
            yield SitePackCreationStatus::Failed {
                error: error.to_string(),
                phase: Some(SitePackImportPhase::Finalizing),
            };
            return;
        }

        let activation = match site_pack.write() {
            Ok(mut pack) => pack.install_staged_db(&stage_dir).map(|()| {
                let now = Local::now();
                pack.set_last_updated_at(now);
                if let Err(error) = main_db.update_site_pack_last_updated(&site_pack_id, &now) {
                    warn!("failed to persist site pack update timestamp: {error}");
                }
            }),
            Err(_) => Err(SitePackImportError::DatabaseFailure {
                message: "failed to lock the site pack during activation".to_string(),
            }),
        };

        match activation {
            Ok(()) => {
                invalidate_caches(&related_caches);
                yield SitePackCreationStatus::Completed {
                site_pack_id,
                progress: Percentage::new(100.0),
                phase: SitePackImportPhase::Completed,
                };
            }
            Err(error) => yield SitePackCreationStatus::Failed {
                error: format!("Could not activate imported site list: {error}"),
                phase: Some(SitePackImportPhase::Finalizing),
            },
        }
    })
}

impl NetworkManager {
    fn caches_for_pack(&self, id: &SitePackId) -> Vec<BlockDecisionCache> {
        self.network_blocks
            .values()
            .filter(|block| block.has_site_pack(id))
            .map(|block| block.results_cache_handle())
            .collect()
    }

    fn invalidate_pack_decisions(&self, id: &SitePackId) {
        for block in self
            .network_blocks
            .values()
            .filter(|block| block.has_site_pack(id))
        {
            block.invalidate_results_cache();
        }
    }

    /// Marks a site pack as syncing. Returns the RAII guard that releases the
    /// slot on drop, or [`NetworkManagerOpErr::SitePackBusy`] when a sync for
    /// the same pack is already in flight (concurrent requests, stream reconnects,
    /// or replayed retries must never start a duplicate import/resync).
    fn begin_site_pack_sync(
        &self,
        site_pack_id: &SitePackId,
    ) -> Result<SitePackSyncGuard, NetworkManagerOpErr> {
        let mut set = self
            .in_flight_site_pack_syncs
            .lock()
            .map_err(|_| NetworkManagerOpErr::RwLockPoison)?;
        if !set.insert(site_pack_id.clone()) {
            return Err(NetworkManagerOpErr::SitePackBusy {
                id: site_pack_id.clone(),
            });
        }
        Ok(SitePackSyncGuard {
            in_flight: self.in_flight_site_pack_syncs.clone(),
            site_pack_id: site_pack_id.clone(),
        })
    }

    pub fn is_site_pack_syncing(&self, site_pack_id: &SitePackId) -> bool {
        self.in_flight_site_pack_syncs
            .lock()
            .map(|set| set.contains(site_pack_id))
            .unwrap_or(false)
    }

    pub fn is_site_pack_being_used(&self, site_pack_id: &SitePackId) -> bool {
        self.network_blocks
            .iter()
            .any(|(_, block)| block.has_site_pack(site_pack_id))
    }

    /// Lock-state rule for site packs (canonical invariant — see
    /// `docs/site-pack-lock-invariant.md`):
    ///
    /// A site pack is **locked** at a moment in time iff
    /// 1. it exists locally,
    /// 2. it is *synchronized* (its data is loaded — an unsynced metadata row
    ///    enforces nothing, so locking it would make no sense), and
    /// 3. at least one *currently locked* network block references it.
    ///
    /// Everything else (missing, unsynced, or only referenced by unlocked
    /// blocks) is treated as unlocked. This is the single source of truth for
    /// deletion/resync/source-deletion guards; callers must not inline their
    /// own copy of the predicate.
    pub fn is_site_pack_locked(&self, site_pack_id: &SitePackId) -> bool {
        let Ok(site_pack) = self.get_site_pack(site_pack_id) else {
            return false;
        };

        // An unsynced pack has no data to protect; it must stay deletable and
        // resyncable so a locked block can still acquire its data.
        if !site_pack.is_synced() {
            return false;
        }

        let now = Local::now();

        self.network_blocks
            .iter()
            .any(|(_, block)| block.has_site_pack(site_pack_id) && block.is_locked(&now))
    }

    pub fn get_site_pack_path(site_pack_id: &SitePackId) -> PathBuf {
        let db_path = Settings::get_site_packs_temp_dir();
        db_path.join((*site_pack_id).to_string())
    }

    pub fn get_site_pack(
        &self,
        site_pack_id: &SitePackId,
    ) -> Result<RwLockReadGuard<'_, SitePack>, NetworkManagerOpErr> {
        let result = self.site_packs.get(site_pack_id);

        if let Some(site_pack) = result {
            if site_pack.is_poisoned() {
                return Err(NetworkManagerOpErr::RwLockPoison);
            }

            return Ok(site_pack.read().unwrap());
        }

        Err(NetworkManagerOpErr::SitePackNotFound {
            id: site_pack_id.clone(),
        })
    }

    pub fn get_all_site_packs(
        &self,
    ) -> Result<Vec<RwLockReadGuard<'_, SitePack>>, NetworkManagerOpErr> {
        self.site_packs
            .values()
            .map(|arc| {
                if arc.is_poisoned() {
                    return Err(NetworkManagerOpErr::RwLockPoison);
                }
                Ok(arc.read().unwrap())
            })
            .collect()
    }

    pub async fn add_local_site_pack(
        &mut self,
        pack_name: &str,
        description: Option<String>,
        icon: Option<String>,
    ) -> Result<Arc<RwLock<SitePack>>, NetworkManagerOpErr> {
        if !self.is_pro_user().await {
            return Err(NetworkManagerOpErr::ProPlanRequired);
        }

        let site_pack_id = SitePackId::try_new(Uuid::now_v7().to_string())
            .expect("generated UUID must be a safe site pack ID");

        let db_path = Settings::get_site_pack_path(&site_pack_id);

        self.db
            .add_site_pack(
                &site_pack_id,
                pack_name,
                description.clone(),
                true, // Default requires_pro to true
                icon.clone(),
            )
            .map_err(|error| {
                error!("Failed to add site list: {}", error);
                NetworkManagerOpErr::InternalError {
                    message: "Failed to add site list".to_string(),
                }
            })?;

        let site_pack = SitePack::from(
            site_pack_id.clone(),
            None,
            pack_name.to_string(),
            description,
            true,
            None,
            icon,
            &db_path,
            Vec::new(),
            None,
        )?;
        let site_pack_arc = Arc::new(RwLock::new(site_pack));
        self.site_packs.insert(site_pack_id, site_pack_arc.clone());
        Ok(site_pack_arc)
    }

    pub async fn import_site_pack(
        &mut self,
        pack_name: &str,
        description: Option<String>,
        icon: Option<String>,
        sources: Vec<SitePackSource>,
    ) -> Result<SitePackImportStream<'static>, NetworkManagerOpErr> {
        if !self.is_pro_user().await {
            return Err(NetworkManagerOpErr::InternalError {
                message: "Failed to add site list".to_string(),
            });
        }

        self.import_site_pack_for_verified_pro(pack_name, description, icon, sources)
    }

    /// Entry point for RPC after checking Pro status without holding the
    /// network-policy lock during a possible remote entitlement verification.
    /// Callers must verify the effective Pro policy before taking the write lock.
    pub(crate) fn import_site_pack_for_verified_pro(
        &mut self,
        pack_name: &str,
        description: Option<String>,
        icon: Option<String>,
        sources: Vec<SitePackSource>,
    ) -> Result<SitePackImportStream<'static>, NetworkManagerOpErr> {
        let site_pack_id = SitePackId::try_new(Uuid::now_v7().to_string())
            .expect("generated UUID must be a safe site pack ID");
        // Persist generated source IDs (as in add_site_pack_source), but retain
        // the incoming source values for the import progress stream.
        let stored_sources: Vec<SitePackSource> = sources
            .iter()
            .map(|source| SitePackSource {
                id: SitePackSourceId::from(Uuid::now_v7()),
                site_pack_id: site_pack_id.clone(),
                source: source.source.clone(),
                source_type: source.source_type,
                format: source.format,
            })
            .collect();

        self.db
            .add_local_site_pack_with_sources(
                &site_pack_id,
                pack_name,
                description.clone(),
                icon.clone(),
                &stored_sources,
            )
            .map_err(|error| {
                error!("Failed to persist site list and sources: {}", error);
                NetworkManagerOpErr::DatabaseError {
                    message: format!("Failed to add site list sources: {error}"),
                }
            })?;

        let db_path = Settings::get_site_pack_path(&site_pack_id);
        let site_pack_arc = Arc::new(RwLock::new(SitePack::from(
            site_pack_id.clone(),
            None,
            pack_name.to_string(),
            description,
            true,
            None,
            icon,
            &db_path,
            stored_sources,
            None,
        )?));
        self.site_packs
            .insert(site_pack_id.clone(), site_pack_arc.clone());

        // The long-running import owns its sync slot, but never closes the live DB
        // until a validated replacement is ready.
        let guard = self.begin_site_pack_sync(&site_pack_id)?;
        let related_caches = self.caches_for_pack(&site_pack_id);
        Ok(run_local_import(
            site_pack_arc,
            sources,
            site_pack_id,
            self.db.clone(),
            guard,
            related_caches,
        ))
    }

    pub fn delete_site_pack(
        &mut self,
        site_pack_id: &SitePackId,
    ) -> Result<(), NetworkManagerOpErr> {
        if self.is_site_pack_locked(site_pack_id) {
            return Err(NetworkManagerOpErr::SitePackIsLocked {
                id: site_pack_id.clone(),
            });
        }
        if self.is_site_pack_syncing(site_pack_id) {
            return Err(NetworkManagerOpErr::SitePackBusy {
                id: site_pack_id.clone(),
            });
        }

        self.db.delete_site_pack(site_pack_id).map_err(|error| {
            NetworkManagerOpErr::DatabaseError {
                message: format!("failed to delete site list {}: {}", site_pack_id, error),
            }
        })?;

        let db_path = self
            .site_packs
            .get(site_pack_id)
            .ok_or_else(|| NetworkManagerOpErr::SitePackNotFound {
                id: site_pack_id.clone(),
            })?
            .read()
            .unwrap()
            .get_db_path()
            .clone();

        self.site_packs.remove(site_pack_id);

        self.network_blocks
            .iter_mut()
            .for_each(|(_, content_list)| {
                content_list.remove_site_pack(site_pack_id);
            });

        if let Err(error) = fs::remove_dir_all(db_path.as_path()) {
            error!(
                "Failed to delete sites list database at {}: {}",
                db_path.as_path().to_str().unwrap_or_default(),
                error
            );
            return Err(NetworkManagerOpErr::InternalError {
                message: format!(
                    "failed to delete sites list database directory {}: {}",
                    db_path.as_path().to_str().unwrap_or_default(),
                    error
                ),
            });
        }

        Ok(())
    }

    pub async fn resync_site_pack(
        &mut self,
        site_pack_id: &SitePackId,
    ) -> Result<SitePackImportStream<'static>, NetworkManagerOpErr> {
        if self.is_site_pack_locked(site_pack_id) {
            return Err(NetworkManagerOpErr::SitePackIsLocked {
                id: site_pack_id.clone(),
            });
        }

        // Reject concurrent/replayed syncs of the same pack. The returned
        // stream runs outside the manager write lock (the import stream lives on
        // after the route returns), so the slot is only freed when the stream
        // ends — completion, failure, or client disconnect dropping the body.
        let guard = self.begin_site_pack_sync(site_pack_id)?;

        let site_pack_arc = self
            .site_packs
            .get(site_pack_id)
            .ok_or_else(|| NetworkManagerOpErr::SitePackNotFound {
                id: site_pack_id.clone(),
            })?
            .clone();

        if site_pack_arc.is_poisoned() {
            return Err(NetworkManagerOpErr::RwLockPoison);
        }

        let site_pack_id = site_pack_id.clone();
        let remote_id = {
            let guard = site_pack_arc.read().unwrap();
            guard.get_remote_id().cloned()
        };

        if let Some(remote_id) = remote_id {
            let api_key = self
                .account_manager
                .get_api_key()
                .await
                .map_err(|message| NetworkManagerOpErr::InternalError { message })?;

            // Unique temp file per attempt: two sequential resyncs of the same
            // pack (or a retry while a previous attempt still cleans up) must
            // never clobber each other's archive.
            let zip_path =
                temp_dir().join(format!("{}_backup_{}.zip", site_pack_id, Uuid::new_v4()));

            let site_pack_arc_clone = site_pack_arc.clone();
            let db_clone = self.db.clone();
            let related_caches = self.caches_for_pack(&site_pack_id);

            let import_stream = Box::pin(stream! {
                let _guard = guard;

                // Phase 0: resolve the backup URL. This is network I/O that
                // must not run under the manager write lock, and its failure
                // (e.g. the Ace server answering 404 "Site pack not found")
                // must reach the client as a terminal FAILED event — never as
                // a terminal FAILED event emitted over the RPC stream.
                let client = AceServerClient::new_http_client(api_key.as_ref());
                let url = match client.get_site_pack_backup(&remote_id).await {
                    Ok(response) => response.url,
                    Err(error) => {
                        yield SitePackCreationStatus::Failed {
                            error: format!("failed to get site list backup: {}", error),
                            phase: Some(SitePackImportPhase::Downloading),
                        };
                        return;
                    }
                };

                // Phase 1: Download (0-40%)
                let dl = file_download::download_file(url.as_str(), zip_path.clone());
                for await status in dl {
                    match status {
                        Ok(percentage) => {
                            yield SitePackCreationStatus::InProgress {
                                site_pack_id: site_pack_id.clone(),
                                progress: percentage * Percentage::new(40.0),
                                phase: SitePackImportPhase::Downloading,
                            };
                        }
                        Err(error) => {
                            yield SitePackCreationStatus::Failed {
                                error: format!("Failed to download site list backup: {}", error),
                                phase: Some(SitePackImportPhase::Downloading),
                            };
                            return;
                        }
                    }
                }

                // Phase 2: Restore (40-100%) — the archive contains the
                // SQLite database file directly, no extraction needed.
                yield SitePackCreationStatus::InProgress {
                    site_pack_id: site_pack_id.clone(),
                    progress: Percentage::new(40.0),
                    phase: SitePackImportPhase::Restoring,
                };

                // Perform restore without holding lock across yield
                let restore_result = {
                    if let Ok(mut list) = site_pack_arc_clone.write() {
                        let result = list.restore_from_backup(zip_path.clone());
                        if result.is_ok() {
                            let now = Local::now();
                            let _ =
                                db_clone.update_site_pack_last_updated(&site_pack_id, &now);
                            list.set_last_updated_at(now);
                        }
                        result
                    } else {
                        Err(SitePackImportError::IoFailure {
                            message: "failed to lock the site pack".to_string(),
                        })
                    }
                };

                if restore_result.is_ok() {
                    invalidate_caches(&related_caches);
                }
                if let Err(error) = restore_result {
                    yield SitePackCreationStatus::Failed {
                        error: format!("Failed to restore site list from backup: {}", error),
                        phase: Some(SitePackImportPhase::Restoring),
                    };
                    return;
                }

                // A manual remote resync restored the *latest* server content,
                // so the local version bookkeeping must follow — otherwise the
                // background updater sees an "older" version and re-downloads
                // the identical archive on its next pass (duplicate download).
                // Best effort: if the releases list cannot be read, the row
                // keeps its previous version and the next background pass
                // reconciles it.
                if let Err(error) = sync_remote_pack_version(
                    &db_clone,
                    &client,
                    &remote_id,
                    &site_pack_arc_clone,
                )
                .await
                {
                    warn!(
                        "Failed to sync version of site list {} after manual resync: {}",
                        remote_id, error
                    );
                }

                yield SitePackCreationStatus::Completed {
                    site_pack_id: site_pack_id.clone(),
                    progress: Percentage::new(100.0),
                    phase: SitePackImportPhase::Completed,
                };
            });

            return Ok(import_stream);
        }

        let sources = site_pack_arc.read().unwrap().get_sources().clone();
        let related_caches = self.caches_for_pack(&site_pack_id);
        Ok(run_local_import(
            site_pack_arc,
            sources,
            site_pack_id,
            self.db.clone(),
            guard,
            related_caches,
        ))
    }

    // --- Site List Sources ---

    pub fn get_site_pack_sources(
        &self,
        site_pack_id: &SitePackId,
    ) -> Result<Vec<SitePackSource>, NetworkManagerOpErr> {
        Ok(self
            .get_site_pack(site_pack_id)
            .map_err(|_| NetworkManagerOpErr::SitePackNotFound {
                id: site_pack_id.clone(),
            })?
            .get_sources()
            .clone())
    }

    pub fn add_site_pack_source(
        &mut self,
        site_pack_id: &SitePackId,
        source: &str,
        source_type: SitePackSourceType,
        source_format: SitePackFormat,
    ) -> Result<SitePackSource, NetworkManagerOpErr> {
        let source_id = SitePackSourceId::from(Uuid::now_v7());

        self.db
            .add_site_pack_source(
                &source_id,
                site_pack_id,
                source,
                &source_type.to_string(),
                &source_format.to_string(),
            )
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to add site list source to {}: {}",
                    site_pack_id, error
                ),
            })?;

        let source = SitePackSource {
            id: source_id,
            site_pack_id: site_pack_id.clone(),
            source: source.to_string(),
            source_type,
            format: source_format,
        };

        // Update in-memory state
        if let Some(site_pack_arc) = self.site_packs.get(site_pack_id) {
            let mut site_pack = site_pack_arc.write().unwrap();
            site_pack.add_source(source.clone());
        }

        Ok(source)
    }

    pub fn delete_site_pack_source(
        &mut self,
        source_id: uuid::Uuid,
    ) -> Result<(), NetworkManagerOpErr> {
        // 1. Get the source to find the site_pack_id
        let source_model = self
            .db
            .get_site_pack_source_by_id(&source_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to get site list source {}: {}", source_id, error),
            })?;

        let site_pack_id =
            SitePackId::try_new(source_model.site_pack_id.clone()).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("Invalid persisted site-pack ID: {error}"),
                }
            })?;

        // Refuse when the pack is locked (single source of truth: the same
        // predicate as delete_site_pack / resync_site_pack). Deleting a source
        // weakens the pack's blocking, which a locked referencing block must
        // never allow.
        if self.is_site_pack_locked(&site_pack_id) {
            return Err(NetworkManagerOpErr::SitePackIsLocked { id: site_pack_id });
        }

        // 3. Delete from DB
        self.db
            .delete_site_pack_source(&source_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to delete site list source {}: {}", source_id, error),
            })?;

        // 4. Update in-memory state
        if let Some(site_pack_arc) = self.site_packs.get(&site_pack_id) {
            let mut site_pack = site_pack_arc.write().unwrap();
            site_pack.remove_source(&SitePackSourceId::from(source_id));
        }

        Ok(())
    }

    /// Adds a new remote site list with version tracking.
    /// Remote site lists are managed by the Ace server and can be updated automatically.
    pub async fn add_remote_site_pack(
        &mut self,
        remote_id: &RemoteSitePackId,
        name: &str,
        description: Option<String>,
        requires_pro: bool,
        version: &SitePackVersion,
        sources: Vec<SitePackSource>,
    ) -> Result<Arc<RwLock<SitePack>>, NetworkManagerOpErr> {
        if !self.is_pro_user().await {
            return Err(NetworkManagerOpErr::ProPlanRequired);
        }

        // Check if a site list with this remote_id already exists
        if let Ok(existing_model) = self.db.get_site_pack_by_remote_id(remote_id) {
            return Err(NetworkManagerOpErr::InternalError {
                message: format!(
                    "Remote site list {} already exists with id {}",
                    remote_id, existing_model.id
                ),
            });
        }

        let site_pack_id = SitePackId::try_new(Uuid::now_v7().to_string())
            .expect("generated UUID must be a safe site pack ID");
        let db_path = Settings::get_site_pack_path(&site_pack_id);

        // Derive the canonical icon slug once so the in-memory pack and the
        // DB row agree immediately (the API serves pack icons from memory).
        let icon = crate::db::site_pack_repo::derive_icon_from_name(name);

        let site_pack = SitePack::from(
            site_pack_id.clone(),
            Some(remote_id.clone()),
            name.to_string(),
            description.clone(),
            requires_pro,
            Some(version.clone()),
            icon,
            &db_path,
            sources.clone(),
            None,
        )?;

        self.site_packs
            .insert(site_pack_id.clone(), Arc::new(RwLock::new(site_pack)));

        let site_pack_arc =
            self.site_packs
                .get(&site_pack_id)
                .ok_or(NetworkManagerOpErr::InternalError {
                    message: "Couldn't fetch just inserted site list somehow".to_string(),
                })?;

        // Add to database
        self.db
            .add_remote_site_pack(
                &site_pack_id,
                remote_id,
                name,
                description,
                requires_pro,
                version,
            )
            .map_err(|error| {
                error!("Failed to add remote site list: {}", error);
                NetworkManagerOpErr::InternalError {
                    message: "Failed to add remote site list".to_string(),
                }
            })?;

        // Add sources to database
        for source in &sources {
            let source_id = SitePackSourceId::from(Uuid::now_v7());
            self.db
                .add_site_pack_source(
                    &source_id,
                    &site_pack_id,
                    &source.source,
                    &source.source_type.to_string(),
                    &source.format.to_string(),
                )
                .map_err(|error| {
                    error!("Failed to add site list source: {}", error);
                    NetworkManagerOpErr::InternalError {
                        message: "Failed to add site list source".to_string(),
                    }
                })?;
        }

        Ok(site_pack_arc.clone())
    }

    /// Commits a staged remote site-pack update **in place** (same site pack
    /// UUID), under the manager write lock.
    ///
    /// The caller ([`ContentUpdateService::stage_pending_updates`]) has
    /// already downloaded, extracted, and validated the new database into
    /// `staged.staging_dir` outside the lock. This method only:
    ///
    /// 1. re-checks that the update is still needed (concurrent callers have
    ///    defined behavior: the second one sees the version already applied
    ///    and skips);
    /// 2. validates the staged database once more;
    /// 3. swaps the pack directory atomically (live → old, staged → live,
    ///    with rollback on failure), so the previous valid pack survives any
    ///    failure;
    /// 4. persists the new version on the **same** DB row and updates
    ///    memory.
    ///
    /// Keeping the pack UUID means `network_block_to_site_pack` join rows and
    /// every `NetworkBlock` reference stay valid — no re-pointing, no orphaned
    /// rows, and no Safe-Mode-on-restart (MGR-01).
    pub fn commit_site_pack_update(
        &mut self,
        staged: crate::service::content_updater::StagedSitePack,
    ) -> Result<(), NetworkManagerOpErr> {
        // 1. Re-check under the lock.
        let existing_model = match self.db.get_site_pack_by_remote_id(&staged.remote_id) {
            Ok(model) => model,
            Err(_) => {
                // The pack no longer exists locally; nothing to update.
                return Ok(());
            }
        };
        let current_version = existing_model
            .version
            .map(SitePackVersion::try_new)
            .transpose()
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: format!("invalid persisted site-pack version: {error}"),
            })?;
        let site_pack_id = SitePackId::try_new(existing_model.id.clone()).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Invalid persisted site-pack ID: {error}"),
            }
        })?;
        let site_pack_arc = match self.site_packs.get(&site_pack_id) {
            Some(arc) => arc.clone(),
            None => {
                warn!(
                    "Site list {} not loaded in memory; skipping update",
                    site_pack_id
                );
                return Ok(());
            }
        };

        // A persisted version can match the latest release even if the pack
        // was never downloaded (e.g. following backup restore). Only a pack
        // with actual local data can skip a same-version update.
        let has_local_data = {
            let pack = site_pack_arc
                .read()
                .map_err(|_| NetworkManagerOpErr::RwLockPoison)?;
            pack.is_synced() && pack.get_db_path().join("pack.db").is_file()
        };
        if has_local_data
            && current_version
                .as_ref()
                .is_some_and(|current| !staged.new_version.is_newer_than(current))
        {
            tracing::info!(
                "Site list {} is up to date (current: {:?}, latest: {})",
                staged.remote_id,
                current_version,
                staged.new_version
            );
            return Ok(());
        }

        // 2. Validate the staged database once more (cheap; it was already
        // validated at staging time).
        {
            let staged_db_path = staged.staging_dir.join("pack.db");
            let staged_db = SitePackDB::from(&staged_db_path).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("staged site list database failed to open: {}", error),
                }
            })?;
            let count =
                staged_db
                    .count_entries()
                    .map_err(|error| NetworkManagerOpErr::InternalError {
                        message: format!("staged site list database is unreadable: {}", error),
                    })?;
            if count == 0 {
                return Err(NetworkManagerOpErr::InternalError {
                    message: "staged site list database is empty".to_string(),
                });
            }
        }

        // 3. Atomic directory swap with rollback.
        let mut site_pack = site_pack_arc.write().unwrap();
        let live_dir = site_pack.get_db_path().clone();
        let parent = live_dir
            .parent()
            .ok_or_else(|| NetworkManagerOpErr::InternalError {
                message: "site list has no parent directory".to_string(),
            })?
            .to_path_buf();
        let unique = Uuid::new_v4();
        let old_dir = parent.join(format!(".{}.old-{}", site_pack_id, unique));

        site_pack.close_db();
        let live_exists = live_dir.exists();
        if live_exists && let Err(error) = std::fs::rename(&live_dir, &old_dir) {
            let _ = site_pack.reload_db();
            return Err(NetworkManagerOpErr::InternalError {
                message: format!("failed to move the current site list database aside: {error}"),
            });
        }
        if let Err(error) = std::fs::rename(&staged.staging_dir, &live_dir) {
            if live_exists {
                let _ = std::fs::rename(&old_dir, &live_dir);
            }
            let _ = site_pack.reload_db();
            return Err(NetworkManagerOpErr::InternalError {
                message: format!("failed to activate the updated site list: {error}"),
            });
        }
        if let Err(error) = site_pack.reload_db() {
            // Roll the swap back: the new database did not open.
            let _ = std::fs::remove_dir_all(&live_dir);
            if live_exists {
                let _ = std::fs::rename(&old_dir, &live_dir);
            }
            let _ = site_pack.reload_db();
            return Err(NetworkManagerOpErr::InternalError {
                message: format!("failed to reload the updated site list: {error}"),
            });
        }
        drop(site_pack);
        self.invalidate_pack_decisions(&site_pack_id);

        // 4. Persist version + timestamp on the SAME row, then update memory.
        let now = Local::now();
        self.db
            .update_site_pack_version(&site_pack_id, &staged.new_version)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to persist site list {} version {}: {}",
                    site_pack_id, staged.new_version, error
                ),
            })?;
        self.db
            .update_site_pack_last_updated(&site_pack_id, &now)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to persist site list {} update time: {}",
                    site_pack_id, error
                ),
            })?;

        let mut site_pack = site_pack_arc.write().unwrap();
        site_pack.set_version(staged.new_version.clone());
        site_pack.set_last_updated_at(now);
        drop(site_pack);

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

        tracing::info!(
            "Successfully updated site list {} to version {} (in place)",
            staged.remote_id,
            staged.new_version
        );

        Ok(())
    }

    /// Commits a fully-staged content update under the manager write lock.
    ///
    /// All network I/O (metadata fetches, downloads, hash verification,
    /// extraction) happened in
    /// [`ContentUpdateService::stage_pending_updates`] **outside** this lock
    /// (SVC-02): a hung server can never stall the proxy or the API. This
    /// method only validates, swaps directories atomically, and transitions
    /// DB/memory state.
    ///
    /// Returns an error when anything failed — including per-item staging
    /// failures carried in `staged.staging_errors` — so callers (error.g. the
    /// `POST /system/update/content` route) can surface a partial failure
    /// instead of claiming success (API-02). Every item is still attempted.
    pub async fn apply_staged_content_updates(
        &mut self,
        staged: crate::service::content_updater::StagedContentUpdate,
    ) -> Result<(), crate::service::content_updater::SitePackUpdateError> {
        use crate::service::content_updater::SitePackUpdateError;

        let mut first_error: Option<String> = None;

        // ── Resistance scripts (Pro only; already gated at staging) ──
        if let Some(bundle) = staged.resistance_scripts
            && let Err(error) = self.commit_resistance_scripts(bundle)
        {
            warn!("Failed to commit resistance scripts: {}", error);
            if first_error.is_none() {
                first_error = Some(format!("resistance scripts: {error}"));
            }
        }

        // ── Site packs (in-place atomic swap per pack) ────────────────
        for site_pack in staged.site_packs {
            if let Err(error) = self.commit_site_pack_update(site_pack) {
                warn!("Failed to commit site list update: {}", error);
                if first_error.is_none() {
                    first_error = Some(format!("site list update: {error}"));
                }
            }
        }

        // ── Staging-phase failures are reported to the caller too ─────
        if let Some(error) = staged.staging_errors.first()
            && first_error.is_none()
        {
            first_error = Some(error.clone());
        }

        match first_error {
            Some(error) => Err(SitePackUpdateError::UpdateError(format!(
                "content update completed with errors: {error}"
            ))),
            None => {
                info!("All staged content updates committed successfully.");
                Ok(())
            }
        }
    }

    /// Adds a remote site list without sources (called from Updater on first startup).
    /// The actual site list data will be downloaded later through the API.
    pub async fn add_remote_site_pack_from_updater(
        &mut self,
        remote_id: &RemoteSitePackId,
        name: &str,
        description: Option<String>,
        requires_pro: bool,
        version: &SitePackVersion,
    ) -> Result<(), NetworkManagerOpErr> {
        // Check if already exists
        if self.db.get_site_pack_by_remote_id(remote_id).is_ok() {
            return Ok(());
        }

        let site_pack_id = SitePackId::try_new(Uuid::now_v7().to_string())
            .expect("generated UUID must be a safe site pack ID");
        let db_path = Settings::get_site_pack_path(&site_pack_id);

        // Derive the canonical icon slug once so the in-memory pack and the
        // DB row agree immediately (the API serves pack icons from memory).
        let icon = crate::db::site_pack_repo::derive_icon_from_name(name);

        let site_pack = SitePack::from(
            site_pack_id.clone(),
            Some(remote_id.clone()),
            name.to_string(),
            description.clone(),
            requires_pro,
            Some(version.clone()),
            icon,
            &db_path,
            Vec::new(), // No sources yet
            None,
        )?;

        self.site_packs
            .insert(site_pack_id.clone(), Arc::new(RwLock::new(site_pack)));

        // Add to database
        self.db
            .add_remote_site_pack(
                &site_pack_id,
                remote_id,
                name,
                description,
                requires_pro,
                version,
            )
            .map_err(|error| {
                error!("Failed to add remote site list: {}", error);
                NetworkManagerOpErr::InternalError {
                    message: "Failed to add remote site list".to_string(),
                }
            })?;

        info!("Registered remote site list: {} ({})", name, remote_id);
        Ok(())
    }

    /// Marks a remote site list as ignored, so it won't be re-registered on next fetch.
    ///
    /// This is the "delete" operation for remote packs (rows are kept so the
    /// pack can be re-enabled later). It is refused while the pack is locked —
    /// hiding an ignored pack makes the engine drop it from memory, which would
    /// silently strip a locked referencing block of its rules.
    pub fn mark_remote_site_pack_as_ignored(
        &mut self,
        site_pack_id: &SitePackId,
    ) -> Result<(), NetworkManagerOpErr> {
        if self.is_site_pack_locked(site_pack_id) {
            return Err(NetworkManagerOpErr::SitePackIsLocked {
                id: site_pack_id.clone(),
            });
        }
        if self.is_site_pack_syncing(site_pack_id) {
            return Err(NetworkManagerOpErr::SitePackBusy {
                id: site_pack_id.clone(),
            });
        }

        // Set is_ignored in DB
        self.db
            .set_site_pack_ignored(site_pack_id, true)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("Failed to mark site list as ignored: {}", error),
            })?;

        // Remove from memory
        self.site_packs.remove(site_pack_id);

        info!("Marked remote site list {} as ignored", site_pack_id);
        Ok(())
    }

    /// Re-enables all ignored remote site lists and re-fetches their metadata.
    pub async fn re_enable_all_remote_site_packs(
        &mut self,
        allow_pro: bool,
    ) -> Result<(), NetworkManagerOpErr> {
        info!("Re-enabling all ignored remote site lists...");

        // Get all ignored remote site lists
        let ignored_lists = self.db.get_ignored_remote_site_packs().map_err(|error| {
            NetworkManagerOpErr::DatabaseError {
                message: format!("Failed to get ignored site lists: {}", error),
            }
        })?;

        // Re-enable each one in DB and load into memory
        for model in &ignored_lists {
            if model.requires_pro && !allow_pro {
                continue;
            }
            let site_pack_id = SitePackId::try_new(model.id.clone()).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("Invalid persisted site-pack ID: {error}"),
                }
            })?;

            // Set is_ignored to false in DB
            self.db
                .set_site_pack_ignored(&site_pack_id, false)
                .map_err(|error| NetworkManagerOpErr::DatabaseError {
                    message: format!("Failed to re-enable site list: {}", error),
                })?;

            // Load into memory if not already there
            if !self.site_packs.contains_key(&site_pack_id) {
                let source_models =
                    self.db
                        .get_site_pack_sources(&site_pack_id)
                        .map_err(|error| NetworkManagerOpErr::DatabaseError {
                            message: format!("Failed to load site-list sources: {error}"),
                        })?;

                let sources: Vec<crate::core::importer::SitePackSource> = source_models
                    .into_iter()
                    .map(|source_model| {
                        let format = crate::core::importer::SitePackFormat::try_from(
                            source_model.format.as_str(),
                        )
                        .unwrap_or(crate::core::importer::SitePackFormat::Standard);
                        let source_type = crate::core::importer::SitePackSourceType::try_from(
                            source_model.source_type.as_str(),
                        )
                        .unwrap_or(crate::core::importer::SitePackSourceType::Url);
                        let source_id =
                            uuid::Uuid::parse_str(&source_model.id).map_err(|error| {
                                NetworkManagerOpErr::InternalError {
                                    message: format!(
                                        "Invalid source UUID '{}' in database: {error}",
                                        source_model.id
                                    ),
                                }
                            })?;
                        Ok(crate::core::importer::SitePackSource {
                            id: crate::core::importer::SitePackSourceId::from(source_id),
                            site_pack_id: SitePackId::try_new(source_model.site_pack_id).map_err(
                                |error| NetworkManagerOpErr::InternalError {
                                    message: format!("Invalid persisted site-pack ID: {error}"),
                                },
                            )?,
                            source: source_model.source,
                            source_type,
                            format,
                        })
                    })
                    .collect::<Result<Vec<_>, NetworkManagerOpErr>>()?;

                let remote_id = model
                    .remote_id
                    .clone()
                    .map(RemoteSitePackId::try_new)
                    .transpose()
                    .map_err(|error| NetworkManagerOpErr::InternalError {
                        message: format!("Invalid remote site-pack ID in database: {error}"),
                    })?;
                let version = model
                    .version
                    .clone()
                    .map(SitePackVersion::try_new)
                    .transpose()
                    .map_err(|error| NetworkManagerOpErr::InternalError {
                        message: format!("Invalid site-pack version in database: {error}"),
                    })?;

                let last_updated_at = model.last_updated_at.as_ref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Local))
                });

                let site_pack = crate::core::SitePack::from(
                    site_pack_id.clone(),
                    remote_id,
                    model.name.clone(),
                    model.description.clone(),
                    model.requires_pro,
                    version,
                    model.icon.clone(),
                    &crate::settings::Settings::get_site_pack_path(&site_pack_id),
                    sources,
                    last_updated_at,
                )?;

                self.site_packs
                    .insert(site_pack_id.clone(), Arc::new(RwLock::new(site_pack)));

                info!(
                    "Loaded re-enabled site list into memory: {} ({})",
                    model.name, site_pack_id
                );
            } else {
                info!("Re-enabled site list: {} ({})", model.name, site_pack_id);
            }
        }

        // Fetch and register any new remote site lists from server
        let mut content_service =
            crate::service::content_updater::ContentUpdateService::from(self.db.clone());

        content_service
            .fetch_and_register_remote_site_packs(self)
            .await
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: format!("Failed to re-fetch remote site lists: {}", error),
            })?;

        info!("Finished re-enabling remote site lists");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network_block::uri_hash::UriHash;
    use crate::core::{Lock, LockState, SitePackId};
    use crate::db::EngineDB;
    use crate::settings::Settings;
    use chrono::Local;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct TestContext {
        manager: NetworkManager,
        _temp_dir: tempfile::TempDir,
    }

    fn typed_remote_id(value: &str) -> RemoteSitePackId {
        RemoteSitePackId::try_new(value.to_string()).unwrap()
    }

    fn typed_version(value: &str) -> SitePackVersion {
        SitePackVersion::try_new(value.to_string()).unwrap()
    }

    fn setup_test_manager() -> TestContext {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");

        // One shared process-wide sites dir for all unit tests (TESTINFRA-01).
        Settings::set_test_sites_dir(Settings::shared_test_sites_dir());

        let db = EngineDB::from(&db_path).unwrap();
        let db_lock = Arc::new(db);
        let account_manager = crate::manager::AccountManager::from(db_lock.clone()).unwrap();
        let mut manager = NetworkManager::from(db_lock, account_manager).unwrap();
        manager.set_debug_pro_policy(crate::manager::network::DebugProPolicy::Pro);

        TestContext {
            manager,
            _temp_dir: temp_dir,
        }
    }

    fn local_source(id: &SitePackId, path: &std::path::Path) -> SitePackSource {
        SitePackSource {
            id: SitePackSourceId::from(Uuid::now_v7()),
            site_pack_id: id.clone(),
            source: path.to_string_lossy().into_owned(),
            source_type: SitePackSourceType::File,
            format: SitePackFormat::WildCardDomains,
        }
    }

    fn seed_pack(pack: &Arc<RwLock<SitePack>>, hostname: &str) {
        use crate::core::network_block::site_pack::site_pack_db::{NewPackMetaModel, hash_host};
        let path = pack.read().unwrap().get_db_path().join("pack.db");
        let db = SitePackDB::from(&path).unwrap();
        db.insert_entries(&[hash_host(hostname)]).unwrap();
        db.insert_pack_meta(NewPackMetaModel {
            id: pack.read().unwrap().get_id().to_string(),
            format_version: "1.0".into(),
            name: None,
            entry_count: 1,
            built_at: chrono::Utc::now().to_rfc3339(),
        })
        .unwrap();
        drop(db);
        pack.write().unwrap().reload_db().unwrap();
    }

    #[tokio::test]
    async fn local_resync_preserves_live_data_on_failure_then_swaps_on_success() {
        use futures::StreamExt;

        let mut ctx = setup_test_manager();
        let pack = ctx
            .manager
            .add_local_site_pack("Staged Resync", None, None)
            .await
            .unwrap();
        seed_pack(&pack, "old.example");
        let id = pack.read().unwrap().get_id().clone();

        let missing = ctx._temp_dir.path().join("missing.txt");
        let guard = ctx.manager.begin_site_pack_sync(&id).unwrap();
        let failed: Vec<_> = run_local_import(
            pack.clone(),
            vec![local_source(&id, &missing)],
            id.clone(),
            ctx.manager.db.clone(),
            guard,
            Vec::new(),
        )
        .collect()
        .await;
        assert!(matches!(
            failed.last(),
            Some(SitePackCreationStatus::Failed { .. })
        ));
        assert!(
            pack.read()
                .unwrap()
                .blocking_result("old.example")
                .is_blocked()
        );
        assert!(!ctx.manager.is_site_pack_syncing(&id));

        let new_source = ctx._temp_dir.path().join("new.txt");
        std::fs::write(&new_source, "new.example\n").unwrap();
        let guard = ctx.manager.begin_site_pack_sync(&id).unwrap();
        let mut stream = run_local_import(
            pack.clone(),
            vec![local_source(&id, &new_source)],
            id.clone(),
            ctx.manager.db.clone(),
            guard,
            Vec::new(),
        );
        assert!(matches!(
            stream.next().await,
            Some(SitePackCreationStatus::Started { .. })
        ));
        // Parsing happens only when the stream advances; enforcement stays live.
        assert!(
            pack.read()
                .unwrap()
                .blocking_result("old.example")
                .is_blocked()
        );
        let statuses: Vec<_> = stream.collect().await;
        assert!(matches!(
            statuses.last(),
            Some(SitePackCreationStatus::Completed { .. })
        ));
        assert!(
            pack.read()
                .unwrap()
                .blocking_result("new.example")
                .is_blocked()
        );
        assert!(
            !pack
                .read()
                .unwrap()
                .blocking_result("old.example")
                .is_blocked()
        );
        assert!(!ctx.manager.is_site_pack_syncing(&id));
    }

    #[tokio::test]
    async fn cancelling_local_resync_preserves_pack_and_releases_sync_slot() {
        use futures::StreamExt;

        let mut ctx = setup_test_manager();
        let pack = ctx
            .manager
            .add_local_site_pack("Cancelled Resync", None, None)
            .await
            .unwrap();
        seed_pack(&pack, "old.example");
        let id = pack.read().unwrap().get_id().clone();
        let source = ctx._temp_dir.path().join("new.txt");
        std::fs::write(&source, "new.example\n").unwrap();
        let guard = ctx.manager.begin_site_pack_sync(&id).unwrap();
        let mut stream = run_local_import(
            pack.clone(),
            vec![local_source(&id, &source)],
            id.clone(),
            ctx.manager.db.clone(),
            guard,
            Vec::new(),
        );
        assert!(matches!(
            stream.next().await,
            Some(SitePackCreationStatus::Started { .. })
        ));
        drop(stream);
        assert!(
            pack.read()
                .unwrap()
                .blocking_result("old.example")
                .is_blocked()
        );
        assert!(!ctx.manager.is_site_pack_syncing(&id));
        let live_dir = pack.read().unwrap().get_db_path().clone();
        let staged_prefix = format!(".{id}.staging-");
        assert!(
            !std::fs::read_dir(live_dir.parent().unwrap())
                .unwrap()
                .any(|entry| {
                    entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&staged_prefix)
                })
        );
    }

    #[tokio::test]
    async fn test_add_and_get_site_pack() {
        let mut ctx = setup_test_manager();
        let name = "Test Site List";

        let site_pack_arc = ctx
            .manager
            .add_local_site_pack(name, None, None)
            .await
            .unwrap();
        site_pack_arc.write().unwrap().reload_db().unwrap();

        let site_pack_uuid = site_pack_arc.read().unwrap().get_id().to_string();

        let site_pack = ctx
            .manager
            .get_site_pack(&SitePackId::try_new(site_pack_uuid.clone()).unwrap())
            .unwrap();
        assert_eq!(site_pack.get_id().to_string(), site_pack_uuid);
        assert_eq!(site_pack.get_name(), name);
    }

    #[tokio::test]
    async fn local_pack_insert_failure_does_not_publish_a_ghost_pack() {
        use diesel::connection::SimpleConnection;
        let mut ctx = setup_test_manager();
        ctx.manager
            .get_db()
            .get_db_connection()
            .unwrap()
            .batch_execute("DROP TABLE site_packs")
            .unwrap();
        let before = ctx.manager.site_packs.len();
        assert!(
            ctx.manager
                .add_local_site_pack("Failure", None, None)
                .await
                .is_err()
        );
        assert_eq!(ctx.manager.site_packs.len(), before);
    }

    #[tokio::test]
    async fn import_source_insert_failure_rolls_back_the_new_pack() {
        use diesel::connection::SimpleConnection;
        let mut ctx = setup_test_manager();
        let before = ctx.manager.get_db().get_all_site_packs().unwrap().len();
        let before_live = ctx.manager.site_packs.len();
        ctx.manager
            .get_db()
            .get_db_connection()
            .unwrap()
            .batch_execute("DROP TABLE site_pack_sources")
            .unwrap();
        let source = local_source(
            &SitePackId::try_new("provisional".to_string()).unwrap(),
            std::path::Path::new("invalid-list.txt"),
        );
        assert!(
            ctx.manager
                .import_site_pack("Failure", None, None, vec![source])
                .await
                .is_err()
        );
        assert_eq!(
            ctx.manager.get_db().get_all_site_packs().unwrap().len(),
            before
        );
        assert_eq!(ctx.manager.site_packs.len(), before_live);
    }

    #[tokio::test]
    async fn test_get_all_site_packs() {
        let mut ctx = setup_test_manager();

        let l1 = ctx
            .manager
            .add_local_site_pack("List 1", None, None)
            .await
            .unwrap();
        l1.write().unwrap().reload_db().unwrap();
        let id1 = l1.read().unwrap().get_id().to_string();

        let l2 = ctx
            .manager
            .add_local_site_pack("List 2", None, None)
            .await
            .unwrap();
        l2.write().unwrap().reload_db().unwrap();
        let id2 = l2.read().unwrap().get_id().to_string();

        let all_lists = ctx.manager.get_all_site_packs().unwrap();

        let found1 = all_lists.iter().any(|l| **l.get_id() == id1);
        let found2 = all_lists.iter().any(|l| **l.get_id() == id2);

        assert!(found1, "List 1 should be found");
        assert!(found2, "List 2 should be found");
    }

    #[tokio::test]
    async fn test_delete_site_pack() {
        let mut ctx = setup_test_manager();

        let list = ctx
            .manager
            .add_local_site_pack("Delete Me", None, None)
            .await
            .unwrap();
        list.write().unwrap().reload_db().unwrap();
        let site_pack_uuid = list.read().unwrap().get_id().to_string();
        let db_path = list.read().unwrap().get_db_path().clone();

        // Verify it's there
        assert!(
            ctx.manager
                .site_packs
                .contains_key(&SitePackId::try_new(site_pack_uuid.clone()).unwrap())
        );

        // DROP the list reference so RocksDB can close the DB before we delete the directory
        drop(list);

        ctx.manager
            .delete_site_pack(&SitePackId::try_new(site_pack_uuid.clone()).unwrap())
            .unwrap();

        // Verify it's gone from memory
        assert!(
            !ctx.manager
                .site_packs
                .contains_key(&SitePackId::try_new(site_pack_uuid).unwrap())
        );
        // Verify DB directory is gone
        assert!(!db_path.exists());
    }

    #[tokio::test]
    async fn test_delete_locked_site_pack_fails() {
        let mut ctx = setup_test_manager();

        let list = ctx
            .manager
            .add_local_site_pack("Blocked List", None, None)
            .await
            .unwrap();
        list.write().unwrap().reload_db().unwrap();
        let site_pack_uuid = list.read().unwrap().get_id().to_string();

        let block_id = ctx
            .manager
            .create_network_block("Test Block".to_string())
            .unwrap();
        ctx.manager
            .add_network_block_to_site_pack(
                &block_id,
                &SitePackId::try_new(site_pack_uuid.clone()).unwrap(),
            )
            .unwrap();

        // Lock the block
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&block_id, lock).unwrap();

        let result = ctx
            .manager
            .delete_site_pack(&SitePackId::try_new(site_pack_uuid).unwrap());
        assert!(matches!(
            result,
            Err(NetworkManagerOpErr::SitePackIsLocked { .. })
        ));
    }

    // ── Lock-state rule (Issue 3 canonical invariant) ──────────────────

    /// Builds a synced local pack attached to a fresh block, returning the
    /// pack id and the block id.
    async fn attach_pack_to_block(
        ctx: &mut TestContext,
    ) -> (SitePackId, crate::core::NetworkBlockId) {
        let list = ctx
            .manager
            .add_local_site_pack("Lock Rule Pack", None, None)
            .await
            .unwrap();
        list.write().unwrap().reload_db().unwrap();
        let pack_id = list.read().unwrap().get_id().clone();

        let block_id = ctx
            .manager
            .create_network_block("Lock Rule Block".to_string())
            .unwrap();
        ctx.manager
            .add_network_block_to_site_pack(&block_id, &pack_id)
            .unwrap();

        (pack_id, block_id)
    }

    #[tokio::test]
    async fn site_pack_assignment_sql_failures_do_not_change_live_links() {
        use diesel::connection::SimpleConnection;

        let mut ctx = setup_test_manager();
        let (pack_id, attached_block_id) = attach_pack_to_block(&mut ctx).await;
        let detached_block_id = ctx
            .manager
            .create_network_block("Unattached block".to_string())
            .unwrap();

        // Remove only the join table in this isolated test database to cause
        // deterministic failures for both link mutation directions.
        ctx.manager
            .get_db()
            .get_db_connection()
            .unwrap()
            .batch_execute("DROP TABLE network_block_to_site_pack")
            .unwrap();

        assert!(matches!(
            ctx.manager
                .add_network_block_to_site_pack(&detached_block_id, &pack_id),
            Err(NetworkManagerOpErr::DatabaseError { .. })
        ));
        assert!(
            !ctx.manager
                .get_network_block_from(&detached_block_id)
                .unwrap()
                .has_site_pack(&pack_id)
        );

        assert!(matches!(
            ctx.manager
                .remove_network_block_to_site_pack(&attached_block_id, &pack_id),
            Err(NetworkManagerOpErr::DatabaseError { .. })
        ));
        assert!(
            ctx.manager
                .get_network_block_from(&attached_block_id)
                .unwrap()
                .has_site_pack(&pack_id)
        );
    }

    fn lock_block_until_end(manager: &mut NetworkManager, block_id: &crate::core::NetworkBlockId) {
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        manager
            .set_lock(
                block_id,
                Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time }),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn test_site_pack_unlocked_when_only_unlocked_block_references_it() {
        let mut ctx = setup_test_manager();
        let (pack_id, _block_id) = attach_pack_to_block(&mut ctx).await;

        // Synced and referenced, but the referencing block is NOT locked.
        assert!(!ctx.manager.is_site_pack_locked(&pack_id));
    }

    #[tokio::test]
    async fn test_site_pack_locked_when_one_locked_block_references_it() {
        let mut ctx = setup_test_manager();
        let (pack_id, block_id) = attach_pack_to_block(&mut ctx).await;
        lock_block_until_end(&mut ctx.manager, &block_id);

        assert!(ctx.manager.is_site_pack_locked(&pack_id));
    }

    #[tokio::test]
    async fn test_site_pack_locked_when_any_of_multiple_blocks_is_locked() {
        let mut ctx = setup_test_manager();
        let (pack_id, first_block) = attach_pack_to_block(&mut ctx).await;

        // Second block also references the pack but stays unlocked.
        let second_block = ctx
            .manager
            .create_network_block("Unlocked Ref".to_string())
            .unwrap();
        ctx.manager
            .add_network_block_to_site_pack(&second_block, &pack_id)
            .unwrap();

        assert!(!ctx.manager.is_site_pack_locked(&pack_id));

        lock_block_until_end(&mut ctx.manager, &first_block);
        assert!(
            ctx.manager.is_site_pack_locked(&pack_id),
            "one locked referencing block must lock the pack even with unlocked refs"
        );
    }

    #[tokio::test]
    async fn test_unsynced_site_pack_is_never_locked() {
        let mut ctx = setup_test_manager();
        let (pack_id, block_id) = attach_pack_to_block(&mut ctx).await;
        lock_block_until_end(&mut ctx.manager, &block_id);
        assert!(ctx.manager.is_site_pack_locked(&pack_id));

        // The pack loses its data (e.g. cache cleared): it enforces nothing,
        // so the lock rule no longer applies — it must stay resyncable so the
        // locked block can re-acquire data.
        ctx.manager
            .site_packs
            .get(&pack_id)
            .unwrap()
            .write()
            .unwrap()
            .close_db();
        assert!(!ctx.manager.is_site_pack_locked(&pack_id));
    }

    #[tokio::test]
    async fn test_delete_site_pack_source_refused_when_locked() {
        let mut ctx = setup_test_manager();
        let (pack_id, block_id) = attach_pack_to_block(&mut ctx).await;
        ctx.manager
            .add_site_pack_source(
                &pack_id,
                "https://example.com/list.txt",
                SitePackSourceType::Url,
                SitePackFormat::Standard,
            )
            .unwrap();
        let source_id = *ctx.manager.get_site_pack_sources(&pack_id).unwrap()[0].id;
        lock_block_until_end(&mut ctx.manager, &block_id);

        let result = ctx.manager.delete_site_pack_source(source_id);
        assert!(matches!(
            result,
            Err(NetworkManagerOpErr::SitePackIsLocked { .. })
        ));
    }

    #[tokio::test]
    async fn test_site_pack_busy_guard_rejects_concurrent_syncs() {
        let mut ctx = setup_test_manager();
        let (pack_id, _block_id) = attach_pack_to_block(&mut ctx).await;

        let guard = ctx.manager.begin_site_pack_sync(&pack_id).unwrap();
        assert!(ctx.manager.is_site_pack_syncing(&pack_id));

        // A second begin for the same pack is refused.
        let second = ctx.manager.begin_site_pack_sync(&pack_id);
        assert!(matches!(
            second,
            Err(NetworkManagerOpErr::SitePackBusy { .. })
        ));

        // Delete while syncing is refused too.
        let delete = ctx.manager.delete_site_pack(&pack_id);
        assert!(matches!(
            delete,
            Err(NetworkManagerOpErr::SitePackBusy { .. })
        ));

        // Dropping the guard (stream end/cancel) releases the slot.
        drop(guard);
        assert!(!ctx.manager.is_site_pack_syncing(&pack_id));
        assert!(ctx.manager.begin_site_pack_sync(&pack_id).is_ok());
    }

    #[tokio::test]
    async fn test_site_pack_sources() {
        let mut ctx = setup_test_manager();
        let list = ctx
            .manager
            .add_local_site_pack("Sources List", None, None)
            .await
            .unwrap();
        list.write().unwrap().reload_db().unwrap();
        let site_pack_uuid = list.read().unwrap().get_id().to_string();

        ctx.manager
            .add_site_pack_source(
                &SitePackId::try_new(site_pack_uuid.clone()).unwrap(),
                "https://example.com/list.txt",
                SitePackSourceType::Url,
                SitePackFormat::Standard,
            )
            .unwrap();

        let sources = ctx
            .manager
            .get_site_pack_sources(&SitePackId::try_new(site_pack_uuid.clone()).unwrap())
            .unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source, "https://example.com/list.txt");

        let source_uuid = *sources[0].id;

        ctx.manager.delete_site_pack_source(source_uuid).unwrap();

        let sources_after = ctx
            .manager
            .get_site_pack_sources(&SitePackId::try_new(site_pack_uuid).unwrap())
            .unwrap();
        assert_eq!(sources_after.len(), 0);
    }

    // ── commit_site_pack_update ───────────────────────────────────────

    use crate::core::network_block::site_pack::site_pack_db::{NewPackMetaModel, hash_host};
    use crate::service::content_updater::{StagedContentUpdate, StagedSitePack};

    /// Builds a staged site-pack update whose staging directory contains a
    /// valid pack database with `hosts`.
    fn make_staged_site_pack(
        parent: &std::path::Path,
        remote_id: &str,
        version: &str,
        hosts: &[&str],
    ) -> StagedSitePack {
        let unique = Uuid::new_v4();
        let staging_dir = parent.join(format!(".staging-{unique}"));
        std::fs::create_dir_all(&staging_dir).unwrap();

        let db = SitePackDB::from(&staging_dir.join("pack.db")).unwrap();
        let hashes: Vec<UriHash> = hosts.iter().map(|host| hash_host(host)).collect();
        db.insert_entries(&hashes).unwrap();
        db.insert_pack_meta(NewPackMetaModel {
            id: remote_id.to_string(),
            format_version: "1.0".to_string(),
            name: None,
            entry_count: hosts.len() as i32,
            built_at: chrono::Utc::now().to_rfc3339(),
        })
        .unwrap();

        StagedSitePack {
            remote_id: typed_remote_id(remote_id),
            new_version: typed_version(version),
            archive_path: parent.join("backup.zip"),
            staging_dir,
        }
    }

    #[tokio::test]
    async fn commit_site_pack_update_applies_in_place_and_keeps_block_references() {
        let mut ctx = setup_test_manager();
        let temp = ctx._temp_dir.path().to_path_buf();

        // A remote pack, first synced with v1.0 content, then connected to a
        // network block. (Registered at 0.9 so the staged 1.0 commit applies.)
        ctx.manager
            .add_remote_site_pack_from_updater(
                &typed_remote_id("remote-1"),
                "Remote Pack",
                None,
                false,
                &typed_version("0.9.0"),
            )
            .await
            .unwrap();
        ctx.manager
            .commit_site_pack_update(make_staged_site_pack(
                &temp,
                "remote-1",
                "1.0.0",
                &["old.example.com"],
            ))
            .unwrap();
        let pack_id = ctx
            .manager
            .db
            .get_site_pack_by_remote_id("remote-1")
            .unwrap()
            .id;
        let pack_id = SitePackId::try_new(pack_id).unwrap();

        let block_id = ctx
            .manager
            .create_network_block("Test Block".to_string())
            .unwrap();
        ctx.manager
            .add_network_block_to_site_pack(&block_id, &pack_id)
            .unwrap();

        // Commit a staged update (as the content updater would produce it).
        let staged = make_staged_site_pack(&temp, "remote-1", "1.1.0", &["new.example.com"]);
        ctx.manager.commit_site_pack_update(staged).unwrap();

        // The SAME pack id is still referenced by the block (MGR-01: no
        // UUID churn, no re-pointing, no orphaned join rows).
        let block = ctx.manager.get_network_block_from(&block_id).unwrap();
        assert!(
            block.has_site_pack(&pack_id),
            "the block must still reference the same pack id"
        );
        let pack = ctx.manager.get_site_pack(&pack_id).unwrap();
        assert!(pack.blocking_result("new.example.com").is_blocked());
        assert!(!pack.blocking_result("old.example.com").is_blocked());
        assert_eq!(pack.get_version().map(AsRef::as_ref), Some("1.1.0"));

        // The DB row is the same row, now with the new version.
        let row = ctx
            .manager
            .db
            .get_site_pack_by_remote_id("remote-1")
            .unwrap();
        assert_eq!(row.id, pack_id.to_string());
        assert_eq!(row.version.as_deref(), Some("1.1.0"));

        // No staging/old litter remains in the packs dir.
        let leftovers: Vec<String> = std::fs::read_dir(&temp)
            .unwrap()
            .filter_map(|error| error.ok())
            .map(|error| error.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains("staging") || name.contains(".old-"))
            .collect();
        assert!(leftovers.is_empty(), "staging/old dirs must be cleaned up");
    }

    #[tokio::test]
    async fn commit_site_pack_update_recovers_missing_data_at_same_version() {
        let mut ctx = setup_test_manager();
        let temp = ctx._temp_dir.path().to_path_buf();

        ctx.manager
            .add_remote_site_pack_from_updater(
                &typed_remote_id("remote-missing"),
                "Remote Pack",
                None,
                false,
                &typed_version("1.0.0"),
            )
            .await
            .unwrap();
        let pack_id = SitePackId::try_new(
            ctx.manager
                .db
                .get_site_pack_by_remote_id("remote-missing")
                .unwrap()
                .id,
        )
        .unwrap();
        assert!(!ctx.manager.get_site_pack(&pack_id).unwrap().is_synced());

        ctx.manager
            .commit_site_pack_update(make_staged_site_pack(
                &temp,
                "remote-missing",
                "1.0.0",
                &["restored.example.com"],
            ))
            .unwrap();

        let pack = ctx.manager.get_site_pack(&pack_id).unwrap();
        assert!(pack.is_synced());
        assert!(pack.blocking_result("restored.example.com").is_blocked());
        assert_eq!(pack.get_version().map(AsRef::as_ref), Some("1.0.0"));
    }

    #[tokio::test]
    async fn commit_site_pack_update_skips_when_not_newer() {
        let mut ctx = setup_test_manager();
        let temp = ctx._temp_dir.path().to_path_buf();

        ctx.manager
            .add_remote_site_pack_from_updater(
                &typed_remote_id("remote-1"),
                "Remote Pack",
                None,
                false,
                &typed_version("1.0.0"),
            )
            .await
            .unwrap();

        ctx.manager
            .commit_site_pack_update(make_staged_site_pack(
                &temp,
                "remote-1",
                "1.1.0",
                &["v1.example.com"],
            ))
            .unwrap();

        // A concurrent/duplicate caller staging the SAME version must be a
        // defined no-op: Ok, and nothing changes (SVC-02 requirement 3).
        let duplicate = make_staged_site_pack(&temp, "remote-1", "1.1.0", &["v2.example.com"]);
        ctx.manager.commit_site_pack_update(duplicate).unwrap();

        let pack_id = ctx
            .manager
            .db
            .get_site_pack_by_remote_id("remote-1")
            .unwrap()
            .id;
        {
            let pack = ctx
                .manager
                .get_site_pack(&SitePackId::try_new(pack_id.clone()).unwrap())
                .unwrap();
            assert!(
                pack.blocking_result("v1.example.com").is_blocked(),
                "the first committed content must be unchanged"
            );
            assert!(!pack.blocking_result("v2.example.com").is_blocked());
        }

        // An OLDER version is also a no-op.
        ctx.manager
            .commit_site_pack_update(make_staged_site_pack(
                &temp,
                "remote-1",
                "1.0.0",
                &["old.example.com"],
            ))
            .unwrap();
        let pack = ctx
            .manager
            .get_site_pack(&SitePackId::try_new(pack_id).unwrap())
            .unwrap();
        assert!(pack.blocking_result("v1.example.com").is_blocked());
        assert_eq!(pack.get_version().map(AsRef::as_ref), Some("1.1.0"));
    }

    #[tokio::test]
    async fn commit_site_pack_update_failure_preserves_the_previous_pack() {
        let mut ctx = setup_test_manager();
        let temp = ctx._temp_dir.path().to_path_buf();

        ctx.manager
            .add_remote_site_pack_from_updater(
                &typed_remote_id("remote-1"),
                "Remote Pack",
                None,
                false,
                &typed_version("1.0.0"),
            )
            .await
            .unwrap();
        ctx.manager
            .commit_site_pack_update(make_staged_site_pack(
                &temp,
                "remote-1",
                "1.1.0",
                &["v1.example.com"],
            ))
            .unwrap();

        // A staged "update" whose database is garbage must fail the commit
        // and leave the previous valid pack fully usable.
        let unique = Uuid::new_v4();
        let bad_staging = temp.join(format!(".staging-{unique}"));
        std::fs::create_dir_all(&bad_staging).unwrap();
        std::fs::write(bad_staging.join("pack.db"), "not a sqlite file").unwrap();
        let bad = StagedSitePack {
            remote_id: typed_remote_id("remote-1"),
            new_version: typed_version("1.2.0"),
            archive_path: temp.join("backup.zip"),
            staging_dir: bad_staging,
        };

        let result = ctx.manager.commit_site_pack_update(bad);
        assert!(result.is_err(), "a corrupt staged database must fail");

        let pack_id = ctx
            .manager
            .db
            .get_site_pack_by_remote_id("remote-1")
            .unwrap()
            .id;
        let pack = ctx
            .manager
            .get_site_pack(&SitePackId::try_new(pack_id).unwrap())
            .unwrap();
        assert!(
            pack.blocking_result("v1.example.com").is_blocked(),
            "the previous pack must survive a failed update"
        );
        assert_eq!(pack.get_version().map(AsRef::as_ref), Some("1.1.0"));
    }

    #[tokio::test]
    async fn concurrent_commits_have_defined_behavior() {
        let ctx = setup_test_manager();
        let temp = ctx._temp_dir.path().to_path_buf();

        // Move the manager behind the same Arc<RwLock> the engine uses.
        let manager_arc: Arc<tokio::sync::RwLock<NetworkManager>> =
            Arc::new(tokio::sync::RwLock::new(ctx.manager));
        {
            let mut manager = manager_arc.write().await;
            manager
                .add_remote_site_pack_from_updater(
                    &typed_remote_id("remote-1"),
                    "Remote Pack",
                    None,
                    false,
                    &typed_version("1.0.0"),
                )
                .await
                .unwrap();
        }

        // Two callers stage and commit the same pack update concurrently
        // (error.g. background loop racing a user-triggered update). The commit
        // re-checks the version under the lock, so exactly one applies the
        // data and the other is a no-op — no corruption, no error.
        let first = {
            let staged = StagedContentUpdate {
                site_packs: vec![make_staged_site_pack(
                    &temp,
                    "remote-1",
                    "1.1.0",
                    &["a.example.com"],
                )],
                resistance_scripts: None,
                staging_errors: Vec::new(),
            };
            let manager = manager_arc.clone();
            tokio::spawn(async move {
                manager
                    .write()
                    .await
                    .apply_staged_content_updates(staged)
                    .await
            })
        };
        let second = {
            let staged = StagedContentUpdate {
                site_packs: vec![make_staged_site_pack(
                    &temp,
                    "remote-1",
                    "1.1.0",
                    &["b.example.com"],
                )],
                resistance_scripts: None,
                staging_errors: Vec::new(),
            };
            let manager = manager_arc.clone();
            tokio::spawn(async move {
                manager
                    .write()
                    .await
                    .apply_staged_content_updates(staged)
                    .await
            })
        };

        first.await.unwrap().unwrap();
        second.await.unwrap().unwrap();

        // Final state is consistent: one version, one row, one pack.
        let contains_a;
        let contains_b;
        {
            let manager = manager_arc.read().await;
            let pack_id = manager
                .db
                .get_site_pack_by_remote_id("remote-1")
                .unwrap()
                .id;
            let pack = manager
                .get_site_pack(&SitePackId::try_new(pack_id).unwrap())
                .unwrap();
            assert_eq!(pack.get_version().map(AsRef::as_ref), Some("1.1.0"));
            contains_a = pack.blocking_result("a.example.com").is_blocked();
            contains_b = pack.blocking_result("b.example.com").is_blocked();
        }
        assert!(
            contains_a ^ contains_b,
            "exactly one of the concurrent commits must have applied (a={contains_a}, b={contains_b})"
        );

        // One row only.
        let count = ctx._temp_dir.path().join("test_db.sqlite");
        let db = EngineDB::from(&count).unwrap();
        let rows = db.get_site_pack_by_remote_id("remote-1").unwrap();
        assert_eq!(rows.remote_id.as_deref(), Some("remote-1"));
    }
}
