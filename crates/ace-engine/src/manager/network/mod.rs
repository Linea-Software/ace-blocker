//! In-memory enforcement state and persistence-backed mutation operations.
//!
//! Request evaluation consults loaded rules and lazy site-pack membership;
//! it must not query the main database for each URL.

pub mod backup;
pub mod error;
pub mod exceptions;
pub mod lock;
pub mod network_blocks;
pub mod network_blocks_to_site_pack;
pub mod proxy_exclusions;
pub mod resistance_mode;
pub mod schedule;
pub mod site_pack;
pub mod user_defined;

use crate::core::SafeSearchEnforcement;
use crate::core::common::injectable_scripts::InjectableScriptTree;
use crate::manager::resistance::{PunishmentIntensity, PunishmentMode, ResistanceModeManager};
use axum::http;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, RwLock},
};

use crate::core::{
    NetworkBlock, NetworkBlockId, RemoteSitePackId, Schedule, SitePack, SitePackId,
    SitePackVersion,
    importer::{SitePackFormat, SitePackSource, SitePackSourceId, SitePackSourceType},
};
use crate::settings::Settings;
use crate::{
    db::{EngineDB, models},
    manager::AccountManager,
};

use error::NetworkManagerOpErr;
use tracing::warn;

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum DebugProPolicy {
    Pro,
    Free,
    Default,
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum DebugResistanceLevel {
    None,
    Low,
    Medium,
    High,
    Extreme,
    Default,
}

/// Owns the live policy shared by the local API and proxy.
///
/// Site-pack handles can be shared between blocks. The service protects this
/// manager with an async read/write lock.
pub struct NetworkManager {
    network_blocks: HashMap<NetworkBlockId, NetworkBlock>,
    site_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>>,
    db: Arc<EngineDB>,

    /// Site packs with an import/resync stream currently in flight. Guarded by
    /// its own mutex because streams outlive the manager write lock (they run
    /// on the SSE response body); the set is what prevents concurrent or
    /// replayed resyncs of the same pack from running duplicates.
    in_flight_site_pack_syncs: Arc<Mutex<HashSet<SitePackId>>>,

    /// Manages resistance mode including scripts, settings, and static intensity
    resistance_mode: ResistanceModeManager,

    account_manager: AccountManager,
    safe_search_enforcements: HashMap<String, SafeSearchEnforcement>,
    #[cfg(debug_assertions)]
    debug_pro_policy: DebugProPolicy,
    #[cfg(debug_assertions)]
    debug_resistance_level: DebugResistanceLevel,
}

impl NetworkManager {
    pub fn from(
        db: Arc<EngineDB>,
        account_manager: AccountManager,
    ) -> Result<NetworkManager, NetworkManagerOpErr> {
        let site_packs = Self::load_site_packs(&db)?;

        let network_blocks = Self::load_net_lists(&db, &site_packs)?;

        let mut resistance_mode = Self::load_resistance_manager(&db);

        // Load the script tree into the resistance mode manager
        let script_tree = Self::try_load_resistance_mode(&db).unwrap_or_else(|error| {
            warn!(
                "Initial resistance engine load failed: {}. Using default.",
                error
            );
            resistance_mode.scripts_load_error = Some(error.to_string());
            InjectableScriptTree::default()
        });
        resistance_mode.script_tree = script_tree;

        // Load resistance mode settings from DB
        if let Ok(settings) = db.get_resistance_mode_settings() {
            resistance_mode.enabled = settings.enabled;
            if let Ok(lock) = crate::core::Lock::from_raw(settings.lock_state, settings.lock_end) {
                resistance_mode.lock = lock;
            }
        }

        let manager = NetworkManager {
            network_blocks,
            site_packs,
            db: db.clone(),
            in_flight_site_pack_syncs: Arc::new(Mutex::new(HashSet::new())),
            resistance_mode,
            account_manager,
            safe_search_enforcements: Self::load_safe_search_enforcements(&db)?,
            #[cfg(debug_assertions)]
            debug_pro_policy: DebugProPolicy::Default,
            #[cfg(debug_assertions)]
            debug_resistance_level: DebugResistanceLevel::Default,
        };

        Ok(manager)
    }

    pub fn new_empty(db: Arc<EngineDB>, account_manager: AccountManager) -> NetworkManager {
        NetworkManager {
            network_blocks: HashMap::new(),
            site_packs: HashMap::new(),
            db: db.clone(),
            in_flight_site_pack_syncs: Arc::new(Mutex::new(HashSet::new())),
            resistance_mode: ResistanceModeManager::default(),
            account_manager,
            safe_search_enforcements: HashMap::new(),
            #[cfg(debug_assertions)]
            debug_pro_policy: DebugProPolicy::Default,
            #[cfg(debug_assertions)]
            debug_resistance_level: DebugResistanceLevel::Default,
        }
    }

    pub fn get_db(&self) -> Arc<EngineDB> {
        self.db.clone()
    }

    pub fn get_account_manager(&self) -> &AccountManager {
        &self.account_manager
    }

    pub fn get_resistance_mode(&self) -> &ResistanceModeManager {
        &self.resistance_mode
    }

    pub fn get_resistance_mode_mut(&mut self) -> &mut ResistanceModeManager {
        &mut self.resistance_mode
    }

    #[cfg(debug_assertions)]
    pub fn set_debug_pro_policy(&mut self, policy: DebugProPolicy) {
        self.debug_pro_policy = policy;
    }

    #[cfg(debug_assertions)]
    pub fn get_debug_pro_policy(&self) -> DebugProPolicy {
        self.debug_pro_policy
    }

    #[cfg(debug_assertions)]
    pub fn set_debug_resistance_level(&mut self, level: DebugResistanceLevel) {
        self.debug_resistance_level = level;
    }

    #[cfg(debug_assertions)]
    pub fn get_debug_resistance_level(&self) -> DebugResistanceLevel {
        self.debug_resistance_level
    }

    pub fn is_pro_user_cached(&self) -> bool {
        #[cfg(debug_assertions)]
        match self.debug_pro_policy {
            DebugProPolicy::Pro => return true,
            DebugProPolicy::Free => return false,
            DebugProPolicy::Default => {}
        }
        self.account_manager.get_cached_pro_status()
    }

    pub async fn is_pro_user(&self) -> bool {
        #[cfg(debug_assertions)]
        match self.debug_pro_policy {
            DebugProPolicy::Pro => return true,
            DebugProPolicy::Free => return false,
            DebugProPolicy::Default => {}
        }
        self.account_manager.is_pro().await
    }

    pub fn get_effective_intensity(&self) -> PunishmentIntensity {
        #[cfg(debug_assertions)]
        match self.debug_resistance_level {
            DebugResistanceLevel::None => return PunishmentIntensity::None,
            DebugResistanceLevel::Low => return PunishmentIntensity::Low,
            DebugResistanceLevel::Medium => return PunishmentIntensity::Medium,
            DebugResistanceLevel::High => return PunishmentIntensity::High,
            DebugResistanceLevel::Extreme => return PunishmentIntensity::Extreme,
            DebugResistanceLevel::Default => {}
        }
        self.resistance_mode.get_intensity()
    }

    pub async fn set_api_key(&self, key: crate::core::ApiKey) -> Result<(), NetworkManagerOpErr> {
        self.account_manager
            .update_account(Some(key), None)
            .await
            .map_err(|error| NetworkManagerOpErr::InternalError { message: error })
    }

    /// Evaluates every enabled and scheduled block; any positive match wins.
    /// A lock affects edit permissions, not this enforcement decision.
    pub fn is_uri_blocked(&self, uri: &http::uri::Uri) -> crate::core::BlockingResult {
        let now = chrono::Local::now();

        for block in self.network_blocks.values() {
            if !block.is_active(&now) {
                continue;
            }
            match block.is_content_blocked(uri) {
                crate::core::BlockingResult::NotBlocked => continue,
                result => return result,
            }
        }
        crate::core::BlockingResult::NotBlocked
    }

    /// Evaluate only rules that are safe to decide from an HTTPS CONNECT host.
    /// This deliberately excludes regex/path rules and any hostname block that
    /// has URL exceptions, because CONNECT does not reveal the request path.
    pub fn is_host_blocked_at_connect(&self, host: &str) -> crate::core::BlockingResult {
        let now = chrono::Local::now();
        let host = host.trim_end_matches('.').to_ascii_lowercase();

        for block in self.network_blocks.values() {
            if block.is_active(&now) && block.blocks_connect_host(&host) {
                return crate::core::BlockingResult::Blocked;
            }
        }

        crate::core::BlockingResult::NotBlocked
    }

    /// Whether HTTPS for this host must be decrypted for a feature that cannot
    /// be implemented from CONNECT metadata alone.
    pub fn requires_tls_interception(&self, host: &str) -> bool {
        let now = chrono::Local::now();
        let host = host.trim_end_matches('.').to_ascii_lowercase();

        // DNS snooping cannot observe encrypted DNS or a browser's cached
        // resolution. An IP CONNECT therefore says nothing about the site's
        // policy; recover its hostname from decrypted HTTP instead.
        if host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok()
        {
            return true;
        }

        if self
            .network_blocks
            .values()
            .any(|block| block.is_active(&now) && block.requires_request_inspection_for_host(&host))
        {
            return true;
        }

        let safe_search_active = self
            .network_blocks
            .values()
            .any(|block| block.is_active(&now) && block.is_safe_search_enabled());
        if safe_search_active && self.safe_search_enforcements.contains_key(&host) {
            return true;
        }

        self.resistance_enforcement_status()
            == crate::manager::network::resistance_mode::EnforcementStatus::Operational
            && self.has_scripts_of(&host)
    }

    pub fn get_safe_search_enforcement(
        &self,
        uri: &http::uri::Uri,
    ) -> Option<SafeSearchEnforcement> {
        let now = chrono::Local::now();
        let mut is_safe_search_enabled = false;

        for block in self.network_blocks.values() {
            if block.is_active(&now) && block.is_safe_search_enabled() {
                is_safe_search_enabled = true;
                break;
            }
        }

        if !is_safe_search_enabled {
            return None;
        }

        let host = uri.host()?;
        self.safe_search_enforcements.get(host).cloned()
    }

    fn load_site_packs(
        db: &Arc<EngineDB>,
    ) -> Result<HashMap<SitePackId, Arc<RwLock<SitePack>>>, NetworkManagerOpErr> {
        let mut site_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>> = HashMap::new();

        let hl_models =
            db.get_all_site_packs()
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("DB Error fetching site packs: {:?}", error),
                })?;

        tracing::info!("Loading {} site packs from database", hl_models.len());

        for model in hl_models {
            let site_pack_id = SitePackId::from(model.id.clone());

            let source_models = match db.get_site_pack_sources(&model.id) {
                Ok(sources) => sources,
                Err(error) => {
                    tracing::error!(
                        "Failed to fetch sources for site pack {}: {}",
                        site_pack_id,
                        error
                    );
                    Vec::new()
                }
            };

            tracing::info!(
                "Loading site pack: {} ({}) with {} sources",
                model.name,
                site_pack_id,
                source_models.len()
            );

            let sources = source_models
                .into_iter()
                .map(|source_model| {
                    let format = SitePackFormat::try_from(source_model.format.as_str())
                        .unwrap_or(SitePackFormat::Standard);
                    let source_type =
                        SitePackSourceType::try_from(source_model.source_type.as_str())
                            .unwrap_or(SitePackSourceType::Url);
                    let source_id = uuid::Uuid::parse_str(&source_model.id).map_err(|error| {
                        NetworkManagerOpErr::InternalError {
                            message: format!(
                                "Invalid source UUID '{}' in database: {error}",
                                source_model.id
                            ),
                        }
                    })?;

                    Ok(SitePackSource {
                        id: SitePackSourceId::from(source_id),
                        site_pack_id: SitePackId::from(source_model.site_pack_id),
                        source: source_model.source,
                        source_type,
                        format,
                    })
                })
                .collect::<Result<Vec<_>, NetworkManagerOpErr>>()?;

            let remote_id = model
                .remote_id
                .map(RemoteSitePackId::try_new)
                .transpose()
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("Invalid remote site-pack ID in database: {error}"),
                })?;
            let version = model
                .version
                .map(SitePackVersion::try_new)
                .transpose()
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("Invalid site-pack version in database: {error}"),
                })?;
            let last_updated_at = model.last_updated_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Local))
            });

            let site_pack = SitePack::from(
                site_pack_id.clone(),
                remote_id,
                model.name,
                model.description,
                model.requires_pro,
                version,
                model.icon,
                &Settings::get_site_pack_path(&model.id),
                sources,
                last_updated_at,
            );

            site_packs.insert(site_pack_id, Arc::new(RwLock::new(site_pack)));
        }

        Ok(site_packs)
    }

    /// Helper: Loads all NetLists and splits them into White/Block lists
    fn load_net_lists(
        db: &Arc<EngineDB>,
        site_packs: &HashMap<SitePackId, Arc<RwLock<SitePack>>>,
    ) -> Result<HashMap<NetworkBlockId, NetworkBlock>, NetworkManagerOpErr> {
        let mut block_lists = HashMap::new();

        // Fetch basic models and the many-to-many relationship map
        let net_list_models =
            db.get_all_network_blocks()
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("DB Error fetching net lists: {:?}", error),
                })?;

        let net_list_to_site_pack_assignments = db
            .get_network_blocks_to_site_packs()
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: format!("DB Error fetching assignments: {:?}", error),
            })?
            .into_iter()
            .map(|(net_list_id, site_packs)| {
                let net_list_uuid = uuid::Uuid::parse_str(&net_list_id).map_err(|error| {
                    NetworkManagerOpErr::InternalError {
                        message: format!(
                            "Invalid network-block UUID '{}' in assignment table: {error}",
                            net_list_id
                        ),
                    }
                })?;
                Ok((
                    NetworkBlockId::from(net_list_uuid),
                    site_packs.into_iter().map(SitePackId::from).collect(),
                ))
            })
            .collect::<Result<HashMap<_, _>, NetworkManagerOpErr>>()?;

        for model in net_list_models {
            // Delegate the complexity of building a single list to another function
            let net_list = Self::build_single_net_list(
                db,
                model,
                &net_list_to_site_pack_assignments,
                site_packs,
            )?;

            block_lists.insert(net_list.get_id(), net_list);
        }

        Ok(block_lists)
    }

    fn build_single_net_list(
        db: &Arc<EngineDB>,
        model: models::NetworkBlockModel,
        assignments: &HashMap<NetworkBlockId, Vec<SitePackId>>,
        loaded_site_packs: &HashMap<SitePackId, Arc<RwLock<SitePack>>>,
    ) -> Result<NetworkBlock, NetworkManagerOpErr> {
        let block_id = uuid::Uuid::parse_str(&model.id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Invalid UUID in DB for network block: {}", error),
            }
        })?;

        // Propagate errors using '?' instead of unwrap
        let schedule_entries = Self::load_schedule_entries_of(db, &block_id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Failed to load schedule: {:?}", error),
            }
        })?;

        let user_def_entries =
            Self::load_user_defined_rules_of(db, &block_id).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("Failed to load rules: {:?}", error),
                }
            })?;

        let lock = Self::load_lock_of(db, &block_id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Failed to load lock: {:?}", error),
            }
        })?;

        // Resolve the assigned SitePacks. A dangling reference (pack row
        // deleted/never downloaded while the join row survived) must NOT fail
        // the whole manager — skip it with a warning instead of dropping the
        // engine into Safe Mode (MGR-01 defense in depth).
        let assigned_site_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>> = assignments
            .get(&NetworkBlockId::from(block_id))
            .map(|ids| {
                ids.iter()
                    .filter_map(|hl_id| {
                        loaded_site_packs
                            .get(hl_id)
                            .map(|pack| (hl_id.clone(), pack.clone()))
                            .or_else(|| {
                                warn!(
                                    "NetBlock {} references missing SitePack {} — skipping the dangling reference",
                                    model.id, hl_id
                                );
                                None
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let exceptions = Self::load_exceptions_of(db, &block_id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Failed to load exceptions: {:?}", error),
            }
        })?;

        Ok(NetworkBlock::from_with_icon(
            NetworkBlockId::from(block_id),
            model.name,
            model.enabled,
            Schedule::from(schedule_entries),
            lock,
            assigned_site_packs,
            user_def_entries,
            exceptions,
            model.safe_search,
            model.icon,
        ))
    }

    /// Loads the resistance manager from the database.
    ///
    /// v1 beta ships static mode only (MGR-24): the persisted row carries a
    /// single `static_intensity`; there is no dynamic ladder or usage history
    /// to reconstruct.
    fn load_resistance_manager(db: &Arc<EngineDB>) -> ResistanceModeManager {
        let mut resistance_manager = ResistanceModeManager::new_static(PunishmentIntensity::None);

        if let Ok(settings) = db.get_resistance_mode_settings() {
            let static_intensity = std::str::FromStr::from_str(&settings.static_intensity)
                .unwrap_or(PunishmentIntensity::None);

            let mode = PunishmentMode::Static {
                intensity: static_intensity,
            };

            let mut punishment = resistance_manager.get_punishment().clone();
            punishment.mode = mode;
            punishment.static_intensity = static_intensity;
            resistance_manager.set_punishment(punishment);
            resistance_manager.scripts_version = settings.scripts_version.clone();
        }

        resistance_manager
    }

    /// Increments the aggregate blocked-attempts counter for today (UTC).
    ///
    /// The proxy calls this on every block decision. The counter is a single
    /// per-day aggregate row in `blocked_attempts` — no per-site history.
    pub fn increment_blocked_attempt(&mut self) -> Result<(), NetworkManagerOpErr> {
        let today = chrono::Utc::now().date_naive();
        self.db.increment_blocked_attempts(today)?;
        Ok(())
    }

    /// Returns today's aggregate blocked-attempts count.
    pub fn get_blocked_attempts_today(&self) -> u64 {
        let today = chrono::Utc::now().date_naive();
        self.db.get_blocked_attempts_for_date(today).unwrap_or(0)
    }

    fn load_safe_search_enforcements(
        db: &Arc<EngineDB>,
    ) -> Result<HashMap<String, SafeSearchEnforcement>, NetworkManagerOpErr> {
        let mut enforcements = HashMap::new();

        let models = db.get_all_safe_search_enforcements().map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("DB Error fetching safe search enforcements: {:?}", error),
            }
        })?;

        for model in models {
            let enforcement = match model.enforcement_type.as_str() {
                "domain_redirection" => {
                    if let Some(target) = model.target_host {
                        SafeSearchEnforcement::DomainRedirection {
                            target_host: target,
                        }
                    } else {
                        tracing::error!(
                            "Invalid domain_redirection enforcement for domain {}: target_host is missing",
                            model.domain
                        );
                        continue;
                    }
                }
                "query_parameter" => {
                    if let (Some(key), Some(value)) =
                        (model.query_param_key, model.query_param_value)
                    {
                        SafeSearchEnforcement::QueryParameter { key, value }
                    } else {
                        tracing::error!(
                            "Invalid query_parameter enforcement for domain {}: key or value is missing",
                            model.domain
                        );
                        continue;
                    }
                }
                _ => {
                    tracing::error!(
                        "Unknown enforcement type for domain {}: {}",
                        model.domain,
                        model.enforcement_type
                    );
                    continue;
                }
            };
            enforcements.insert(model.domain, enforcement);
        }

        Ok(enforcements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Lock, LockState, UserDefinedRule, UserDefinedRuleType};
    use crate::db::EngineDB;
    use crate::settings::Settings;
    use chrono::Local;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct TestContext {
        manager: NetworkManager,
        _temp_dir: tempfile::TempDir,
    }

    fn setup_test_manager() -> TestContext {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");

        // One shared process-wide sites dir for all unit tests (TESTINFRA-01).
        Settings::set_test_sites_dir(Settings::shared_test_sites_dir());

        let db = EngineDB::from(&db_path).unwrap();
        let db_lock = Arc::new(db);
        let account_manager = crate::manager::AccountManager::from(db_lock.clone()).unwrap();
        let manager = NetworkManager::from(db_lock, account_manager).unwrap();

        TestContext {
            manager,
            _temp_dir: temp_dir,
        }
    }
    #[test]
    fn test_network_manager_from_db() {
        let mut ctx = setup_test_manager();

        // Create some initial data in DB directly or via manager
        let list_id = ctx
            .manager
            .create_network_block("Test List".into())
            .unwrap();

        // Re-create manager from DB to test loading
        let db = ctx.manager.db.clone();
        let account_manager = AccountManager::from(db.clone()).unwrap();
        let new_manager = NetworkManager::from(db, account_manager).unwrap();

        assert!(new_manager.network_blocks.contains_key(&list_id));
    }

    #[test]
    fn test_lock_persists_across_manager_reload() {
        // Spec §3.3/§5: a time-lock must survive an engine restart — otherwise
        // restarting the engine would defeat the lock. Rebuilding the manager
        // from the same DB simulates a restart; the reloaded lock must still
        // be active, keep its end time, and still constrain the block.
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Persistent Lock List".into())
            .unwrap();

        // Lock until a defined end time (second precision, like the DB stores it).
        let lock_end = (Local::now() + chrono::Duration::hours(1))
            .format(crate::core::constants::DATETIME_FORMAT)
            .to_string();
        let lock_end_time = chrono::NaiveDateTime::parse_from_str(
            &lock_end,
            crate::core::constants::DATETIME_FORMAT,
        )
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
        ctx.manager
            .set_lock(
                &list_id,
                Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time }),
            )
            .unwrap();

        // Simulate a restart: reload the manager from the same database.
        let db = ctx.manager.db.clone();
        let account_manager = AccountManager::from(db.clone()).unwrap();
        let new_manager = NetworkManager::from(db, account_manager).unwrap();

        // The lock is still loaded, active, and carries the same end time.
        let reloaded_lock = new_manager.get_lock_of(&list_id).unwrap();
        assert!(matches!(
            reloaded_lock.get_state(),
            LockState::ActiveUntilLockEnd { .. }
        ));
        assert_eq!(
            reloaded_lock.get_end_time().map(|end| end
                .format(crate::core::constants::DATETIME_FORMAT)
                .to_string()),
            Some(lock_end.clone()),
            "the lock's end time must survive a reload"
        );
        assert!(
            reloaded_lock.is_active(
                new_manager
                    .get_network_block_from(&list_id)
                    .unwrap()
                    .get_schedule(),
                &Local::now()
            )
        );

        // Enforcement must survive too: deletion, exception-add, and lock
        // removal/downgrade are all refused after the reload.
        let mut reloaded = new_manager;
        assert!(reloaded.delete_network_block(&list_id).is_err());
        assert!(
            reloaded
                .add_exceptions(&list_id, vec!["allowed.example.com".into()])
                .is_err()
        );
        assert!(
            reloaded
                .set_lock(&list_id, Lock::from(LockState::Inactive))
                .is_err()
        );
        assert!(
            reloaded
                .set_lock(&list_id, Lock::from(LockState::ActiveDuringSchedule))
                .is_err()
        );
    }

    #[test]
    fn test_get_mut_net_list_from() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Mutable List".into())
            .unwrap();

        let list = ctx.manager.get_mut_network_block_from(&list_id);
        assert!(list.is_some());

        let list = ctx
            .manager
            .get_mut_network_block_from(&NetworkBlockId::from(uuid::Uuid::nil()));
        assert!(list.is_none());
    }

    #[tokio::test]
    async fn test_is_host_blocked() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Block List".into())
            .unwrap();

        // A block is only active while its schedule matches (a block with no
        // schedule never matches). Activate it with an all-day entry.
        ctx.manager
            .add_schedule_entries(
                &list_id,
                vec![
                    crate::core::ScheduleEntry::from(
                        crate::core::ScheduleEntryId::from(uuid::Uuid::now_v7()),
                        list_id.clone(),
                        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                        chrono::WeekdaySet::ALL,
                    )
                    .unwrap(),
                ],
            )
            .unwrap();

        // Add a user defined entry to block "example.com"
        ctx.manager
            .add_user_defined_entries(
                &list_id,
                vec![UserDefinedRule::new(
                    "example.com".into(),
                    UserDefinedRuleType::Domain,
                )],
            )
            .unwrap();

        // Lock the list to make it active
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock).unwrap();
        let uri = "http://example.com".parse::<axum::http::Uri>().unwrap();
        assert!(!matches!(
            ctx.manager.is_uri_blocked(&uri),
            crate::core::BlockingResult::NotBlocked
        ));
        assert!(!matches!(
            ctx.manager.is_host_blocked_at_connect("example.com"),
            crate::core::BlockingResult::NotBlocked
        ));
        assert!(
            !ctx.manager.requires_tls_interception("example.com"),
            "a pure domain block must be enforceable before TLS"
        );

        let uri_safe = "http://google.com".parse::<axum::http::Uri>().unwrap();
        assert!(matches!(
            ctx.manager.is_uri_blocked(&uri_safe),
            crate::core::BlockingResult::NotBlocked
        ));
    }

    #[tokio::test]
    async fn test_exceptions_override_blocklist() {
        let mut ctx = setup_test_manager();

        // Block List blocking example.com and other-blocked.com
        let list_id = ctx
            .manager
            .create_network_block("Block List".into())
            .unwrap();
        ctx.manager
            .add_user_defined_entries(
                &list_id,
                vec![
                    UserDefinedRule::new("example.com".into(), UserDefinedRuleType::Domain),
                    UserDefinedRule::new("other-blocked.com".into(), UserDefinedRuleType::Domain),
                ],
            )
            .unwrap();

        // Exception allowing example.com only
        ctx.manager
            .add_exceptions(&list_id, vec!["example.com".into()])
            .unwrap();

        // Activate the block (all-day schedule + lock) so the exception is
        // really exercised — an inactive block would pass for the wrong
        // reason.
        ctx.manager
            .add_schedule_entries(
                &list_id,
                vec![
                    crate::core::ScheduleEntry::from(
                        crate::core::ScheduleEntryId::from(uuid::Uuid::now_v7()),
                        list_id.clone(),
                        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                        chrono::WeekdaySet::ALL,
                    )
                    .unwrap(),
                ],
            )
            .unwrap();

        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock_block = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock_block).unwrap();

        let uri = "http://example.com".parse::<axum::http::Uri>().unwrap();
        // Should NOT be blocked because it's in the whitelist/exceptions
        assert!(matches!(
            ctx.manager.is_uri_blocked(&uri),
            crate::core::BlockingResult::NotBlocked
        ));

        // A non-excepted host on the same active block IS blocked — proves
        // the block is genuinely active and the exception is what allows
        // example.com through.
        let other = "http://other-blocked.com"
            .parse::<axum::http::Uri>()
            .unwrap();
        assert!(!matches!(
            ctx.manager.is_uri_blocked(&other),
            crate::core::BlockingResult::NotBlocked
        ));

        // CONNECT cannot see URL exceptions, so hostname blocking is deferred
        // to the decrypted request path for hosts covered by this block.
        assert!(matches!(
            ctx.manager.is_host_blocked_at_connect("example.com"),
            crate::core::BlockingResult::NotBlocked
        ));
        assert!(ctx.manager.requires_tls_interception("example.com"));
        assert!(ctx.manager.requires_tls_interception("other-blocked.com"));
    }

    #[test]
    fn test_blocked_attempts_aggregate_increments() {
        // The aggregate blocked-attempts counter is a single per-day row in
        // `blocked_attempts`, derived only from block decisions.
        let mut ctx = setup_test_manager();
        assert_eq!(ctx.manager.get_blocked_attempts_today(), 0);

        ctx.manager.increment_blocked_attempt().unwrap();
        ctx.manager.increment_blocked_attempt().unwrap();
        ctx.manager.increment_blocked_attempt().unwrap();

        assert_eq!(ctx.manager.get_blocked_attempts_today(), 3);
    }
}
