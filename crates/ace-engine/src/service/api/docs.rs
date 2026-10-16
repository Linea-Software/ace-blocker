use crate::service::api::schemas::*;
use crate::service::api::{
    route_account, route_backup, route_dashboard, route_lock, route_net_blocks_to_site_packs,
    route_network_blocks, route_resistance_mode, route_schedule, route_site_pack, route_system,
    route_unlock, route_user_rules,
};

use crate::core::LockState;
use crate::core::network_block::site_pack::SitePackCreationStatus;

#[cfg(debug_assertions)]
use crate::service::api::route_dev;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        route_network_blocks::get_all_network_blocks,
        route_network_blocks::create_network_block,
        route_network_blocks::rename_network_block,
        		route_network_blocks::delete_network_block,
        		route_network_blocks::set_network_block_safe_search,
        route_network_blocks::set_network_block_enabled,
        		route_network_blocks::set_network_block_icon,
        route_site_pack::get_all_site_packs,
        route_site_pack::create_site_pack,
        route_site_pack::delete_site_pack,
        route_site_pack::get_site_pack_sources,
        route_site_pack::add_site_pack_source,
        route_site_pack::delete_site_pack_source,
        route_site_pack::resync_site_pack,
        route_site_pack::re_enable_remote_site_packs,
        route_user_rules::get_user_rules,
        route_user_rules::add_user_rules,
        route_user_rules::delete_user_rule,
        route_user_rules::get_resistance_mode_type,
        route_user_rules::set_resistance_mode_type,
        route_schedule::get_schedule_entries,
        route_schedule::add_schedule_entries,
        route_schedule::edit_schedule_entry,
        route_schedule::delete_schedule_entry,
        route_lock::get_lock,
        route_lock::set_lock,
        route_unlock::initiate_unlock,
        route_unlock::confirm_unlock,
        route_net_blocks_to_site_packs::get_net_block_to_site_pack_connections,
        route_net_blocks_to_site_packs::add_net_block_to_site_pack_connection,
        route_net_blocks_to_site_packs::remove_net_block_to_site_pack_connection,
        route_resistance_mode::get_resistance_mode_settings,
        route_resistance_mode::set_resistance_mode_enabled,
        route_resistance_mode::set_resistance_mode_lock,
        route_resistance_mode::get_all_scripts,
        route_resistance_mode::get_script,
        route_resistance_mode::set_script_enabled,
        route_resistance_mode::download_resistance_scripts,
        		route_account::get_account_info,
        		route_account::refresh_account,
        		route_account::set_api_key,
        		route_account::logout,
        		route_account::accept_tos,
        route_backup::export_backup,
        route_backup::import_backup,
        route_backup::export_backup_to_cloud,
        route_backup::import_backup_from_cloud,

        route_system::ping,
        route_system::get_version,
        route_system::open_pro_page,
        route_system::check_updates,
        route_system::trigger_content_update,
        route_system::shutdown,
        route_dashboard::get_dashboard_snapshot,
    ),
    components(
        schemas(
            NetworkBlockSnapshot,
            DashboardSnapshotResponse,
            NetworkBlockSchema,
            GetAllNetworkBlocksResponse,
            CreateNetworkBlockRequest,
            CreateNetworkBlockResponse,
            RenameNetworkBlockRequest,
            SitePackSchema,
            GetAllSitePacksResponse,
            BaseSitePackSourceSchema,
            CreateSitePackRequest,
            SitePackSourceType,
            SitePackFormat,
            SitePackCreationStatus,
            GetSitePackSourcesResponse,
            AddSitePackSourceRequest,
            AddSitePackSourceResponse,
            UserDefinedRuleSchema,
            GetUserDefinedRulesResponse,
            AddUserDefinedRulesRequest,
            AddUserDefinedRulesResponse,
            ScheduleEntrySchema,
            GetScheduleEntriesResponse,
            BaseScheduleEntrySchema,
            AddScheduleEntriesRequest,
            NetworkListLockSchema,
            LockState,
            GetNetworkLockResponse,
            SetNetworkLockRequest,
            SetNetworkLockResponse,
            UnlockChallengeInitiateResponse,
            UnlockChallengeConfirmRequest,
            UnlockChallengeConfirmResponse,
            TooEarlyResponse,
            NetworkBlockToSitePackConnection,
            GetNetListToSitePacksResponse,
            AddNetListToSitePackRequest,
            AddNetListToSitePackResponse,
            ResistanceModeSettings,
            ScriptInfo,
            		SetEnabledRequest,
            		SetIconRequest,
            GetAllScriptsResponse,
            ResistanceModeTypeResponse,
            AccountInfoResponse,
            SetApiKeyRequest,
            EngineBackupSchema,
            BinaryUpdateInfo,
            SitePackUpdateEntry,
            ContentUpdateInfo,
            UpdateCheckResponse,
            VersionInfo,
        )
    ),
    tags(
        (name = "Network Blocks", description = "Network Block management items"),
        (name = "Sites Lists", description = "Sites List management items"),
        (name = "User Rules", description = "User Defined Rules management items"),
        (name = "Schedule", description = "Schedule management items"),
        (name = "Locks", description = "Lock management items"),
        (name = "Connections", description = "Connection management items"),
        (name = "Resistance Mode", description = "Resistance engine management items"),
        (name = "Backup", description = "Backup and restore items"),
        (name = "System", description = "System utilities"),
        (name = "Dashboard", description = "Aggregated dashboard data"),
    ),
    modifiers(&OpenApiModifier)
)]
pub struct ApiDoc;

#[cfg(debug_assertions)]
#[derive(OpenApi)]
#[openapi(
    paths(
        route_dev::set_debug_pro_policy,
        route_dev::get_debug_pro_policy,
        route_dev::set_debug_resistance_level,
        route_dev::get_debug_resistance_level,
        route_dev::get_update_check_interval,
        		route_dev::set_update_check_interval,
        		route_dev::reset_tos,
    ),
    components(schemas(
        crate::manager::network::DebugProPolicy,
        crate::manager::network::DebugResistanceLevel,
        route_dev::UpdateCheckIntervalResponse,
    )),
    tags((name = "Dev", description = "Development only items"))
)]
struct DevApiDoc;

/// Dev docs for the Windows-only file-hiding endpoints (the handlers are
/// `cfg(target_os = "windows")`, see `route_dev`).
#[cfg(all(debug_assertions, target_os = "windows"))]
#[derive(OpenApi)]
#[openapi(
    paths(route_dev::hide_files, route_dev::show_files),
    tags((name = "Dev", description = "Development only items"))
)]
struct DevApiDocFileHiding;

pub struct OpenApiModifier;

impl utoipa::Modify for OpenApiModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        #[cfg(debug_assertions)]
        openapi.merge(DevApiDoc::openapi());
        #[cfg(all(debug_assertions, target_os = "windows"))]
        openapi.merge(DevApiDocFileHiding::openapi());

        use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};

        let datetime_format = crate::core::constants::DATETIME_FORMAT;
        let time_format = crate::core::constants::TIME_FORMAT;

        let extra_info = format!(
            "Datetime format: `{}`\nTime format: `{}`",
            datetime_format, time_format
        );

        match &mut openapi.info.description {
            Some(desc) => {
                desc.push_str("\n\n");
                desc.push_str(&extra_info);
            }
            None => {
                openapi.info.description = Some(extra_info);
            }
        }

        // Set a default title if missing, identifying the API
        if openapi.info.title.is_empty() {
            openapi.info.title = "Ace Engine API".to_string();
        }

        // Inject the engine version into the OpenAPI spec
        openapi.info.version = crate::core::constants::ENGINE_VERSION.to_string();

        // Add license information
        let mut license = utoipa::openapi::info::License::new("MIT");
        license.identifier = Some("MIT".to_string());
        openapi.info.license = Some(license);

        // Add constants schema
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);

        let constants_schema = ObjectBuilder::new()
            .schema_type(Type::Object)
            .required("datetime_format")
            .required("time_format")
            .property(
                "datetime_format",
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .default(Some(serde_json::Value::String(datetime_format.to_string())))
                    .examples(Some(vec![serde_json::Value::String(
                        datetime_format.to_string(),
                    )]))
                    .enum_values(Some(vec![datetime_format.to_string()]))
                    .description(Some("The configured datetime format string"))
                    .build(),
            )
            .property(
                "time_format",
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .default(Some(serde_json::Value::String(time_format.to_string())))
                    .examples(Some(vec![serde_json::Value::String(
                        time_format.to_string(),
                    )]))
                    .enum_values(Some(vec![time_format.to_string()]))
                    .description(Some("The configured time format string"))
                    .build(),
            )
            .description(Some("Application configuration constants"))
            .build();

        components.schemas.insert(
            "ApiConstants".to_string(),
            Schema::Object(constants_schema).into(),
        );
    }
}
