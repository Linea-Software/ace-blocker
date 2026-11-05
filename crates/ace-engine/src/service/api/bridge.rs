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

// One typed remoc call + standard JSON serialization. Only argument-free
// bridge arms use this; complex responses and streaming remain explicit.
macro_rules! no_arg_call {
    ($call:expr) => {{
        let result = ($call).await.map_err(transport_error)??;
        serde_json::to_value(result).map_err(transport_error)
    }};
}

pub async fn call(op: &str, args: Value, capability: Option<String>) -> Result<Value, RpcError> {
    let client: EngineControlClient = super::transport::connect().await.map_err(transport_error)?;
    match op {
        "getAccountInfo" => no_arg_call!(client.get_account_info(capability)),
        "refreshAccount" => no_arg_call!(client.refresh_account(capability)),
        "acceptTos" => no_arg_call!(client.accept_tos(capability)),
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
        "logout" => no_arg_call!(client.logout(capability)),
        "exportBackup" => no_arg_call!(client.export_backup(capability)),
        "exportBackupToCloud" => no_arg_call!(client.export_backup_to_cloud(capability)),
        "importBackupFromCloud" => no_arg_call!(client.import_backup_from_cloud(capability)),
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
        "getDashboardSnapshot" => no_arg_call!(client.get_dashboard_snapshot(capability)),
        "getDebugProPolicy" => no_arg_call!(client.get_debug_pro_policy(capability)),
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
        "getDebugResistanceLevel" => no_arg_call!(client.get_debug_resistance_level(capability)),
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
        "hideFiles" => no_arg_call!(client.hide_files(capability)),
        "showFiles" => no_arg_call!(client.show_files(capability)),
        "getUpdateCheckInterval" => no_arg_call!(client.get_update_check_interval(capability)),
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
        "resetTos" => no_arg_call!(client.reset_tos(capability)),
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
        "getAllNetworkBlocks" => no_arg_call!(client.get_all_network_blocks(capability)),
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
        "saveNetworkBlock" => {
            let payload: SaveNetworkBlockRequest =
                serde_json::from_value(args.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(bad_args)?;
            let result = client
                .save_network_block(capability, payload)
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
            no_arg_call!(client.get_resistance_mode_settings(capability))
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
        "getAllScripts" => no_arg_call!(client.get_all_scripts(capability)),
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
        "downloadResistanceScripts" => no_arg_call!(client.download_resistance_scripts(capability)),
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
        "getAllSitePacks" => no_arg_call!(client.get_all_site_packs(capability)),
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
        "reEnableRemoteSitePacks" => no_arg_call!(client.re_enable_remote_site_packs(capability)),
        "ping" => no_arg_call!(client.ping(capability)),
        "getVersion" => no_arg_call!(client.get_version(capability)),
        "openProPage" => no_arg_call!(client.open_pro_page(capability)),
        "checkUpdates" => no_arg_call!(client.check_updates(capability)),
        "triggerContentUpdate" => no_arg_call!(client.trigger_content_update(capability)),
        "shutdown" => no_arg_call!(client.shutdown(capability)),
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
        "getResistanceModeType" => no_arg_call!(client.get_resistance_mode_type(capability)),
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn tauri_bridge_covers_every_typed_remoc_operation() {
        let rpc = include_str!("rpc.rs");
        let bridge = include_str!("bridge.rs");
        let trait_body = rpc
            .split_once("pub trait EngineControl {")
            .expect("typed remoc trait exists")
            .1
            .split_once("\n}")
            .expect("typed remoc trait closes")
            .0;
        let mut expected = HashSet::new();
        for line in trait_body.lines() {
            let Some(method) = line.trim().strip_prefix("async fn ") else {
                continue;
            };
            let method = method.split('(').next().unwrap();
            let mut operation = String::new();
            let mut uppercase_next = false;
            for ch in method.chars() {
                if ch == '_' {
                    uppercase_next = true;
                } else if uppercase_next {
                    operation.push(ch.to_ascii_uppercase());
                    uppercase_next = false;
                } else {
                    operation.push(ch);
                }
            }
            expected.insert(operation);
        }

        let declared: HashSet<_> = bridge
            .lines()
            .filter_map(|line| {
                line.trim_start()
                    .strip_prefix('"')?
                    .split_once("\" =>")
                    .map(|(operation, _)| operation.to_string())
            })
            .collect();
        assert_eq!(expected.len(), 63, "remoc operation inventory changed");
        assert_eq!(
            expected, declared,
            "bridge operation inventory drifted from the typed remoc trait"
        );
    }
}
