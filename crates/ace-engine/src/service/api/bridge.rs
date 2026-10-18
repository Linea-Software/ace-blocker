//! Generated explicit Tauri-boundary dispatch to the typed remoc RTC methods.
//! This is not an IPC protocol or a string-based wire router.
use super::rpc::{EngineControl, EngineControlClient, RpcError, RpcResponse, UnlockConfirmOutcome};
use super::schemas::*;
use crate::core::network_block::site_pack::SitePackCreationStatus;
use serde_json::Value;

fn transport_error(error: impl std::fmt::Display) -> RpcError {
    RpcError {
        status: 503,
        code: "transport_unavailable".into(),
        error: error.to_string(),
    }
}
fn bad_args(error: impl std::fmt::Display) -> RpcError {
    RpcError {
        status: 400,
        code: "invalid_request_body".into(),
        error: error.to_string(),
    }
}

pub async fn call(op: &str, args: Value, capability: Option<String>) -> Result<Value, RpcError> {
    let client: EngineControlClient = super::transport::connect().await.map_err(transport_error)?;
    match op {
        "getAccountInfo" => {
            let result = client
                .get_account_info(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "refreshAccount" => {
            let result = client
                .refresh_account(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "acceptTos" => {
            let result = client
                .accept_tos(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setApiKey" => {
            let payload: SetApiKeyRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_api_key(capability, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "logout" => {
            let result = client.logout(capability).await.map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "exportBackup" => {
            let result = client
                .export_backup(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "exportBackupToCloud" => {
            let result = client
                .export_backup_to_cloud(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "importBackupFromCloud" => {
            let result = client
                .import_backup_from_cloud(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "importBackup" => {
            let payload: crate::manager::network::backup::EngineBackup =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .import_backup(capability, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getDashboardSnapshot" => {
            let result = client
                .get_dashboard_snapshot(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getDebugProPolicy" => {
            let result = client
                .get_debug_pro_policy(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setDebugProPolicy" => {
            let policy: crate::manager::network::DebugProPolicy =
                serde_json::from_value(args.get("policy").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_debug_pro_policy(capability, policy)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getDebugResistanceLevel" => {
            let result = client
                .get_debug_resistance_level(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setDebugResistanceLevel" => {
            let level: crate::manager::network::DebugResistanceLevel =
                serde_json::from_value(args.get("level").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_debug_resistance_level(capability, level)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "hideFiles" => {
            let result = client
                .hide_files(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "showFiles" => {
            let result = client
                .show_files(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getUpdateCheckInterval" => {
            let result = client
                .get_update_check_interval(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setUpdateCheckInterval" => {
            let seconds: u64 =
                serde_json::from_value(args.get("seconds").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_update_check_interval(capability, seconds)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "resetTos" => {
            let result = client
                .reset_tos(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getLock" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .get_lock(capability, network_block_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setLock" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: SetNetworkLockRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_lock(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getNetBlockToSitePackConnections" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .get_net_block_to_site_pack_connections(capability, network_block_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "addNetBlockToSitePackConnection" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: AddNetListToSitePackRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .add_net_block_to_site_pack_connection(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "removeNetBlockToSitePackConnection" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let site_pack_id: String =
                serde_json::from_value(args.get("site_pack_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .remove_net_block_to_site_pack_connection(
                    capability,
                    network_block_id,
                    site_pack_id,
                )
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "renameNetworkBlock" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: RenameNetworkBlockRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .rename_network_block(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getAllNetworkBlocks" => {
            let result = client
                .get_all_network_blocks(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "createNetworkBlock" => {
            let payload: CreateNetworkBlockRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .create_network_block(capability, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "deleteNetworkBlock" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .delete_network_block(capability, network_block_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setNetworkBlockSafeSearch" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: SetEnabledRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_network_block_safe_search(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setNetworkBlockEnabled" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: SetEnabledRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_network_block_enabled(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setNetworkBlockIcon" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: SetIconRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_network_block_icon(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getResistanceModeSettings" => {
            let result = client
                .get_resistance_mode_settings(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setResistanceModeEnabled" => {
            let payload: SetEnabledRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_resistance_mode_enabled(capability, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setResistanceModeLock" => {
            let payload: SetNetworkLockRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_resistance_mode_lock(capability, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getAllScripts" => {
            let result = client
                .get_all_scripts(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getScript" => {
            let script_id: String =
                serde_json::from_value(args.get("script_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .get_script(capability, script_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setScriptEnabled" => {
            let script_id: String =
                serde_json::from_value(args.get("script_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let payload: SetEnabledRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_script_enabled(capability, script_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "downloadResistanceScripts" => {
            let result = client
                .download_resistance_scripts(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getScheduleEntries" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .get_schedule_entries(capability, network_block_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "addScheduleEntries" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: AddScheduleEntriesRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .add_schedule_entries(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "editScheduleEntry" => {
            let schedule_entry_id: uuid::Uuid = serde_json::from_value(
                args.get("schedule_entry_id")
                    .cloned()
                    .unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let entry: BaseScheduleEntrySchema =
                serde_json::from_value(args.get("entry").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .edit_schedule_entry(capability, schedule_entry_id, entry)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "deleteScheduleEntry" => {
            let schedule_entry_id: uuid::Uuid = serde_json::from_value(
                args.get("schedule_entry_id")
                    .cloned()
                    .unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .delete_schedule_entry(capability, schedule_entry_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getAllSitePacks" => {
            let result = client
                .get_all_site_packs(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "createSitePack" => Err(RpcError {
            status: 400,
            code: "stream_operation".into(),
            error: "Use engine_stream for this operation".into(),
        }),
        "deleteSitePack" => {
            let site_pack_id: String =
                serde_json::from_value(args.get("site_pack_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .delete_site_pack(capability, site_pack_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "resyncSitePack" => Err(RpcError {
            status: 400,
            code: "stream_operation".into(),
            error: "Use engine_stream for this operation".into(),
        }),
        "getSitePackSources" => {
            let site_pack_id: String =
                serde_json::from_value(args.get("site_pack_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .get_site_pack_sources(capability, site_pack_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "addSitePackSource" => {
            let site_pack_id: String =
                serde_json::from_value(args.get("site_pack_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let payload: AddSitePackSourceRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .add_site_pack_source(capability, site_pack_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "deleteSitePackSource" => {
            let source_id: uuid::Uuid =
                serde_json::from_value(args.get("source_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .delete_site_pack_source(capability, source_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "reEnableRemoteSitePacks" => {
            let result = client
                .re_enable_remote_site_packs(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "ping" => {
            let result = client.ping(capability).await.map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getVersion" => {
            let result = client
                .get_version(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "openProPage" => {
            let result = client
                .open_pro_page(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "checkUpdates" => {
            let result = client
                .check_updates(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "triggerContentUpdate" => {
            let result = client
                .trigger_content_update(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "shutdown" => {
            let result = client
                .shutdown(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "initiateUnlock" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .initiate_unlock(capability, network_block_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "confirmUnlock" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: UnlockChallengeConfirmRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .confirm_unlock(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            // JSON response shape remains unchanged for the existing UI.
            // serde_json::Value must not be sent over remoc's binary codec.
            let result = match result {
                UnlockConfirmOutcome::TooEarly(body) => RpcResponse {
                    status: 425,
                    body: serde_json::to_value(body).map_err(transport_error)?,
                },
                UnlockConfirmOutcome::Unlocked(body) => RpcResponse {
                    status: 200,
                    body: serde_json::to_value(body).map_err(transport_error)?,
                },
            };
            serde_json::to_value(result).map_err(transport_error)
        }
        "getUserRules" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let result = client
                .get_user_rules(capability, network_block_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "addUserRules" => {
            let network_block_id: uuid::Uuid = serde_json::from_value(
                args.get("network_block_id").cloned().unwrap_or(Value::Null),
            )
            .map_err(bad_args)?;
            let payload: AddUserDefinedRulesRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .add_user_rules(capability, network_block_id, payload)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "deleteUserRule" => {
            let user_rule_id: uuid::Uuid =
                serde_json::from_value(args.get("user_rule_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .delete_user_rule(capability, user_rule_id)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "getResistanceModeType" => {
            let result = client
                .get_resistance_mode_type(capability)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        "setResistanceModeType" => {
            let mode: crate::manager::resistance::punishment::PunishmentMode =
                serde_json::from_value(args.get("mode").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .set_resistance_mode_type(capability, mode)
                .await
                .map_err(transport_error)??;
            serde_json::to_value(result).map_err(transport_error)
        }
        _ => Err(RpcError {
            status: 404,
            code: "unknown_operation".into(),
            error: format!("Unknown engine operation: {op}"),
        }),
    }
}

pub async fn stream(
    op: &str,
    args: Value,
    capability: Option<String>,
) -> Result<remoc::rch::mpsc::Receiver<SitePackCreationStatus>, RpcError> {
    let client: EngineControlClient = super::transport::connect().await.map_err(transport_error)?;
    match op {
        "createSitePack" => {
            let payload: CreateSitePackRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            client
                .create_site_pack(capability, payload)
                .await
                .map_err(transport_error)?
        }
        "resyncSitePack" => {
            let site_pack_id: String =
                serde_json::from_value(args.get("site_pack_id").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            client
                .resync_site_pack(capability, site_pack_id)
                .await
                .map_err(transport_error)?
        }
        _ => Err(RpcError {
            status: 404,
            code: "unknown_operation".into(),
            error: format!("Unknown streaming operation: {op}"),
        }),
    }
}
