use diesel::prelude::*;

#[derive(Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = crate::schema_main_db::network_blocks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NetworkBlockModel {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub safe_search: bool,
    pub icon: Option<String>,
    pub schedule_timezone: String,
    pub schedule_mode: String,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema_main_db::resistance_mode_settings)]
#[diesel(primary_key(table_lock))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ResistanceModeSettingsModel {
    pub table_lock: String,
    pub enabled: bool,
    pub lock_state: String,
    pub lock_end: Option<String>,
    pub scripts_version: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema_main_db::network_block_schedule_entries)]
#[diesel(belongs_to(NetworkBlockModel, foreign_key = network_block_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NetworkBlockScheduleEntryModel {
    pub id: String,
    pub network_block_id: String,
    pub start_time: String,
    pub end_time: String,
    pub schedule_days_bitmask: i32,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema_main_db::network_block_locks)]
#[diesel(belongs_to(NetworkBlockModel, foreign_key = network_block_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NetworkBlockLockModel {
    pub network_block_id: String,
    pub lock_state: String,
    pub lock_end: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema_main_db::user_defined_rules)]
#[diesel(belongs_to(NetworkBlockModel))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserDefinedRuleModel {
    pub id: String,
    pub network_block_id: String,
    pub rule: String,
    pub rule_type: String,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema_main_db::network_block_exceptions)]
#[diesel(belongs_to(NetworkBlockModel))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ExceptionModel {
    pub id: String,
    pub network_block_id: String,
    pub rule: String,
}

#[derive(Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = crate::schema_main_db::site_packs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SitePackModel {
    pub id: String,
    pub remote_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub requires_pro: bool,
    pub last_updated_at: Option<String>,
    pub version: Option<String>,
    pub is_ignored: bool,
    pub icon: Option<String>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema_main_db::network_block_to_site_pack)]
#[diesel(primary_key(network_block_id, site_pack_id))]
#[diesel(belongs_to(NetworkBlockModel))]
#[diesel(belongs_to(SitePackModel))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NetworkBlockToSitePackModel {
    pub network_block_id: String,
    pub site_pack_id: String,
}

#[derive(Queryable, Selectable, Insertable, Identifiable)]
#[diesel(table_name = crate::schema_main_db::site_pack_sources)]
#[diesel(belongs_to(SitePackModel, foreign_key = site_pack_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SitePackSourceModel {
    pub id: String,
    pub site_pack_id: String,
    pub source: String,
    pub source_type: String,
    pub format: String,
}

#[derive(Queryable)]
#[diesel(table_name = crate::schema_main_db::proxy_exclusions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ProxyExlusionModel {
    pub host: String,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema_main_db::settings)]
#[diesel(primary_key(table_lock))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SettingsModel {
    pub table_lock: String,
    pub is_pro: bool,
    pub is_seeded: bool,
    pub latest_update_check_at: Option<String>,
}

#[derive(Queryable, Selectable, Insertable, Identifiable)]
#[diesel(table_name = crate::schema_main_db::blocked_attempts)]
#[diesel(primary_key(date))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BlockedAttemptsModel {
    pub date: String,
    pub attempts_count: i32,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema_main_db::script_states)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ScriptStateModel {
    pub script_id: String,
    pub enabled: bool,
}
#[derive(Queryable, Selectable, Insertable, AsChangeset, Default)]
#[diesel(table_name = crate::schema_main_db::accounts)]
#[diesel(primary_key(table_lock))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AccountModel {
    pub table_lock: String,
    pub api_key: Option<String>,
    pub last_pro_check: Option<String>,
    pub tos_accepted_at: Option<String>,
    pub entitlement_token: Option<String>,
    pub entitlement_public_key: Option<String>,
    pub clock_high_watermark: Option<String>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema_main_db::safe_search_enforcements)]
#[diesel(primary_key(domain))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SafeSearchEnforcementModel {
    pub domain: String,
    pub enforcement_type: String,
    pub target_host: Option<String>,
    pub query_param_key: Option<String>,
    pub query_param_value: Option<String>,
}
