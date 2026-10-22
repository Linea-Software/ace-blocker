pub use crate::core::importer::{SitePackFormat, SitePackSourceType};
pub use crate::core::{UserDefinedRule, UserDefinedRuleType};
pub use crate::manager::resistance::{PunishmentIntensity, PunishmentMode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- network_list.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkBlockSchema {
    pub id: uuid::Uuid,
    pub name: String,
    pub enabled: bool,
    pub is_active: bool,
    pub is_locked: bool,
    pub lock_state: LockState,
    pub safe_search: bool,
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetAllNetworkBlocksResponse {
    pub network_blocks: Vec<NetworkBlockSchema>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNetworkBlockRequest {
    pub block_name: String,
    pub icon: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetIconRequest {
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNetworkBlockResponse {
    pub network_block: NetworkBlockSchema,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RenameNetworkBlockRequest {
    pub new_name: String,
}

// --- sites_list.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackSchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub is_synced: bool,
    /// True when the local data was installed from the installer's offline
    /// bundle (a `.bundled` marker next to `pack.db`). Cleared automatically
    /// whenever the engine swaps the pack directory with downloaded data.
    pub is_bundled: bool,
    pub entries_count: u64,
    pub requires_pro: bool,
    pub last_updated_at: Option<String>,
    /// Content version of the local pack data (`site_packs.version`): the
    /// server release version when downloaded/updated, or the bundle content
    /// version (`YYYY.MM.DD`) when installed from the installer.
    pub version: Option<String>,
    /// Lock-state exposure (canonical invariant): true when the pack is
    /// synchronized AND at least one currently-locked network block
    /// references it. Locked packs must not be deleted or resynced; the
    /// engine enforces this server-side regardless of the UI state.
    pub is_locked: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetAllSitePacksResponse {
    pub site_packs: Vec<SitePackSchema>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseSitePackSourceSchema {
    pub source: String,
    pub source_type: SitePackSourceType,
    pub format: SitePackFormat,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSitePackRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub custom_folder: Option<String>,
    pub sources: Vec<BaseSitePackSourceSchema>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackSourceSchema {
    pub id: String,
    pub site_pack_id: String,
    pub source: String,
    pub source_type: SitePackSourceType,
    pub format: SitePackFormat,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetSitePackSourcesResponse {
    pub sources: Vec<SitePackSourceSchema>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddSitePackSourceRequest {
    pub source: String,
    pub source_type: SitePackSourceType,
    pub format: SitePackFormat,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddSitePackSourceResponse {
    pub source: SitePackSourceSchema,
}

// --- user_rules.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserDefinedRuleSchema {
    pub id: uuid::Uuid,
    pub network_block_id: uuid::Uuid,
    pub rule: String,
    pub rule_type: UserDefinedRuleType,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetUserDefinedRulesResponse {
    pub user_defined_rules: Vec<UserDefinedRuleSchema>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddUserDefinedRulesRequest {
    pub rules: Vec<UserDefinedRule>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddUserDefinedRulesResponse {
    pub user_defined_rules: Vec<UserDefinedRuleSchema>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserRuleRequest {
    pub rule_id: uuid::Uuid,
}

// --- exceptions.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Exception {
    pub id: uuid::Uuid,
    pub rule: String,
    pub network_block_id: uuid::Uuid,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetExceptionsResponse {
    pub exceptions: Vec<Exception>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddExceptionsRequest {
    pub exceptions: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddExceptionsResponse {
    pub exceptions: Vec<Exception>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteExceptionRequest {
    pub exception_id: uuid::Uuid,
}

// --- schedule.rs ---

/// Represents the data of a ScheduleEntry without the ID
#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseScheduleEntrySchema {
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEntrySchema {
    pub id: uuid::Uuid,
    pub network_block_id: uuid::Uuid,
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetScheduleEntriesResponse {
    pub schedule_entries: Vec<ScheduleEntrySchema>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddScheduleEntriesRequest {
    pub schedule_entries: Vec<BaseScheduleEntrySchema>,
}

// --- lock.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkListLockSchema {
    pub network_block_id: uuid::Uuid,
    pub is_active: bool,
    pub lock_state: LockState,
    pub lock_end: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetNetworkLockResponse {
    pub lock: NetworkListLockSchema,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkLockRequest {
    pub lock_state: String,
    pub lock_end: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkLockResponse {
    pub lock: NetworkListLockSchema,
}

// --- connection.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkBlockToSitePackConnection {
    pub network_block_id: uuid::Uuid,
    pub site_pack_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetNetListToSitePacksResponse {
    pub connections: Vec<NetworkBlockToSitePackConnection>,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddNetListToSitePackRequest {
    pub sites_list_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddNetListToSitePackResponse {
    pub connection: NetworkBlockToSitePackConnection,
}

// --- route_resistance_mode.rs ---

/// Keep the UI's internally tagged JSON mode representation, while sending
/// a regular enum over remoc's binary codec (which cannot deserialize an
/// internally tagged enum via Serde's `deserialize_any`).
pub(crate) mod punishment_mode_wire {
    use super::{PunishmentIntensity, PunishmentMode};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    enum BinaryMode {
        Static { intensity: PunishmentIntensity },
    }

    pub fn serialize<S: Serializer>(
        mode: &PunishmentMode,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            mode.serialize(serializer)
        } else {
            match mode {
                PunishmentMode::Static { intensity } => BinaryMode::Static {
                    intensity: *intensity,
                }
                .serialize(serializer),
            }
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PunishmentMode, D::Error> {
        if deserializer.is_human_readable() {
            PunishmentMode::deserialize(deserializer)
        } else {
            match BinaryMode::deserialize(deserializer)? {
                BinaryMode::Static { intensity } => Ok(PunishmentMode::Static { intensity }),
            }
        }
    }
}

/// Machine-readable enforcement status for Resistance Mode.
///
/// The engine never reports `operational` merely because the master setting
/// is enabled: scripts must be loaded, eligible for injection, and a
/// non-`none` intensity must be configured. Priority order when multiple
/// conditions fail: `disabled` > `not_pro` > `scripts_missing` >
/// `scripts_invalid` > `no_scripts_enabled` > `intensity_none`.
pub use crate::manager::network::resistance_mode::EnforcementStatus;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResistanceModeSettings {
    pub enabled: bool,
    pub is_locked: bool,
    pub lock_state: LockState,
    pub lock_end: Option<String>,
    /// Current mode (static only in v1 beta)
    #[serde(with = "punishment_mode_wire")]
    #[schemars(with = "PunishmentMode")]
    pub mode: PunishmentMode,
    /// Currently effective intensity (static intensity)
    pub current_intensity: PunishmentIntensity,
    /// Static mode intensity
    pub static_intensity: PunishmentIntensity,
    /// Installed scripts version (from the last successful download).
    pub scripts_version: Option<String>,
    /// True when a valid script catalog/tree is loaded in memory.
    pub scripts_loaded: bool,
    /// Total number of loaded scripts (all domains, including generic).
    pub scripts_total: u32,
    /// Number of loaded scripts that are enabled.
    pub scripts_enabled: u32,
    /// Whether the user is currently eligible under the Pro state (cached).
    pub pro_eligible: bool,
    /// Whether enforcement is operational right now, plus a stable reason.
    pub enforcement_status: EnforcementStatus,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInfo {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub description: String,
    pub is_enabled: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetEnabledRequest {
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetAllScriptsResponse {
    pub scripts: Vec<ScriptInfo>,
}

// --- resistance mode type ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResistanceModeTypeResponse {
    #[serde(with = "punishment_mode_wire")]
    #[schemars(with = "PunishmentMode")]
    pub mode: PunishmentMode,
    pub current_intensity: PunishmentIntensity,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiDocError {
    pub error: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfoResponse {
    pub is_pro: bool,
    pub tos_accepted: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetApiKeyRequest {
    pub api_key: String,
}

// --- route_backup.rs ---
pub use crate::manager::network::backup::{
    BackupNetworkBlock, BackupScheduleEntry, BackupSitePack, BackupSitePackSource, EngineBackup,
};
pub type EngineBackupSchema = EngineBackup;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LockState {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

impl From<crate::core::LockState> for LockState {
    fn from(state: crate::core::LockState) -> Self {
        match state {
            crate::core::LockState::Inactive => Self {
                state: "inactive".to_string(),
                end: None,
            },
            crate::core::LockState::SoftUnlockable => Self {
                state: "soft_unlockable".to_string(),
                end: None,
            },
            crate::core::LockState::ActiveDuringSchedule => Self {
                state: "active_during_schedule".to_string(),
                end: None,
            },
            crate::core::LockState::ActiveUntilLockEnd { end } => Self {
                state: "active_until_lock_end".to_string(),
                end: Some(
                    end.format(crate::core::constants::DATETIME_FORMAT)
                        .to_string(),
                ),
            },
        }
    }
}

// ── Core → Schema From impls ──

impl From<&crate::core::ScheduleEntry> for ScheduleEntrySchema {
    fn from(entry: &crate::core::ScheduleEntry) -> Self {
        let weekdays = entry.get_weekday_set();
        Self {
            id: *entry.get_id(),
            network_block_id: *entry.get_parent_list_id(),
            start_time: entry.get_start(),
            end_time: entry.get_end(),
            monday: weekdays.contains(chrono::Weekday::Mon),
            tuesday: weekdays.contains(chrono::Weekday::Tue),
            wednesday: weekdays.contains(chrono::Weekday::Wed),
            thursday: weekdays.contains(chrono::Weekday::Thu),
            friday: weekdays.contains(chrono::Weekday::Fri),
            saturday: weekdays.contains(chrono::Weekday::Sat),
            sunday: weekdays.contains(chrono::Weekday::Sun),
        }
    }
}

impl TryFrom<crate::db::models::UserDefinedRuleModel> for UserDefinedRuleSchema {
    type Error = String;

    fn try_from(model: crate::db::models::UserDefinedRuleModel) -> Result<Self, Self::Error> {
        Ok(Self {
            id: model
                .id
                .parse()
                .map_err(|error| format!("invalid user rule id: {error}"))?,
            network_block_id: model
                .network_block_id
                .parse()
                .map_err(|error| format!("invalid network block id on user rule: {error}"))?,
            rule: model.rule,
            rule_type: model
                .rule_type
                .parse()
                .map_err(|error| format!("invalid user rule type: {error}"))?,
        })
    }
}

impl From<(&crate::core::NetworkBlock, &chrono::DateTime<chrono::Local>)> for NetworkBlockSchema {
    fn from((block, now): (&crate::core::NetworkBlock, &chrono::DateTime<chrono::Local>)) -> Self {
        Self {
            id: *block.get_id(),
            name: block.get_name(),
            enabled: block.is_enabled(),
            is_active: block.is_active(now),
            is_locked: block.is_locked(now),
            lock_state: block.get_lock().get_state().into(),
            safe_search: block.is_safe_search_enabled(),
            icon: block.get_icon(),
        }
    }
}

impl TryFrom<&crate::core::network_block::site_pack::SitePack> for SitePackSchema {
    type Error = crate::db::error::DBError;
    fn try_from(
        pack: &crate::core::network_block::site_pack::SitePack,
    ) -> Result<Self, Self::Error> {
        // The bundle-origin label reflects disk state: a `.bundled` marker
        // file exists next to `pack.db` exactly while the data came from the
        // installer bundle (any engine update swaps the whole directory and
        // clears it).
        let is_bundled = pack
            .get_db_path()
            .join(crate::service::bundled_site_packs::BUNDLED_MARKER_FILE)
            .is_file();
        Ok(Self {
            id: pack.get_id().to_string(),
            name: pack.get_name().to_string(),
            description: pack.get_description().unwrap_or_default().to_string(),
            icon: pack.get_icon().map(|v| v.to_string()),
            is_synced: pack.is_synced(),
            is_bundled,
            entries_count: pack.get_entry_count()?,
            requires_pro: pack.requires_pro(),
            last_updated_at: pack.get_last_updated_at().map(|dt| dt.to_rfc3339()),
            version: pack.get_version().map(|v| v.to_string()),
            // Default false; the route enriches it with the manager's lock
            // predicate (a bare SitePack cannot know which blocks reference it).
            is_locked: false,
        })
    }
}

impl From<&crate::core::importer::SitePackSource> for SitePackSourceSchema {
    fn from(source: &crate::core::importer::SitePackSource) -> Self {
        Self {
            id: source.id.to_string(),
            site_pack_id: source.site_pack_id.to_string(),
            source: source.source.clone(),
            source_type: source.source_type,
            format: source.format,
        }
    }
}

impl From<crate::core::common::injectable_scripts::ScriptEntry> for ScriptInfo {
    fn from(entry: crate::core::common::injectable_scripts::ScriptEntry) -> Self {
        Self {
            id: (*entry.id).to_string(),
            domain: entry.domain,
            name: entry.name,
            description: entry.description,
            is_enabled: entry.enabled,
        }
    }
}

// --- Lock state schema (already has From<core::LockState>) ---

impl NetworkListLockSchema {
    pub fn from_lock(
        lock: &crate::core::Lock,
        network_block_id: uuid::Uuid,
        is_active: bool,
    ) -> Self {
        Self {
            network_block_id,
            is_active,
            lock_state: lock.get_state().into(),
            lock_end: lock.get_end_time().map(|end| {
                end.format(crate::core::constants::DATETIME_FORMAT)
                    .to_string()
            }),
        }
    }
}

// --- System / Update routes ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BinaryUpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub available: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackUpdateEntry {
    pub id: String,
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdateInfo {
    pub site_packs_needing_update: Vec<SitePackUpdateEntry>,
    pub resistance_scripts_need_update: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResponse {
    pub engine: BinaryUpdateInfo,
    pub content: ContentUpdateInfo,
}

// --- route_system.rs (version) ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    /// Semantic version of the engine (from Cargo.toml)
    pub version: String,
    /// Build profile: "debug" or "release"
    pub build: String,
}

// --- route_unlock.rs ---

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnlockChallengeInitiateResponse {
    pub challenge_id: uuid::Uuid,
    pub reflection_prompt: String,
    pub min_words: u32,
    pub cooldown_seconds: i64,
    pub initiated_at: String,
}

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnlockChallengeConfirmRequest {
    pub challenge_id: uuid::Uuid,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnlockChallengeConfirmResponse {
    pub network_block: NetworkBlockSchema,
}

// --- route_dashboard.rs ---

/// A flattened snapshot of a single network block with all nested data included.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetworkBlockSnapshot {
    pub block: NetworkBlockSchema,
    pub lock: Option<NetworkListLockSchema>,
    pub schedule_entries: Vec<ScheduleEntrySchema>,
    pub site_pack_connection_count: usize,
    pub user_rules_count: usize,
}

/// Single-call response that returns all data the dashboard needs.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshotResponse {
    pub blocks: Vec<NetworkBlockSnapshot>,
    /// Today's aggregate blocked-attempts count.
    pub blocked_attempts: u64,
    pub resistance_mode_settings: Option<ResistanceModeSettings>,
}

/// Body for the 425 Too Early response — includes how many seconds to wait.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TooEarlyResponse {
    pub error: String,
    pub retry_after_seconds: i64,
}
