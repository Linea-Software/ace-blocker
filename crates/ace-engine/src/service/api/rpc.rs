//! Typed remoc RPC facade over transport-independent policy-enforcing operations.
//! No REST routes or JSON RPC wire protocol.
//! All state-changing calls require the existing installation capability.
//! The Windows named-pipe ACL is additional OS-layer protection, not an
//! authorization substitute.
#[cfg(debug_assertions)]
use super::operations_dev::UpdateCheckIntervalResponse;
use super::{AppState, schemas::*};
use crate::core::network_block::site_pack::SitePackCreationStatus;
#[cfg(not(debug_assertions))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateCheckIntervalResponse {
    pub interval_seconds: u64,
}
use remoc::rtc::CallError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcError {
    pub status: u16,
    pub code: String,
    pub error: String,
}

impl From<super::error::ApiError> for RpcError {
    fn from(error: super::error::ApiError) -> Self {
        Self {
            status: error.status(),
            code: error.code().to_owned(),
            error: error.message(),
        }
    }
}

impl RpcError {
    fn forbidden() -> Self {
        super::error::ApiError::CapabilityRequired(
            "This action requires a valid capability token".to_string(),
        )
        .into()
    }

    // Debug operations unavailable in release builds or on unsupported
    // platforms return the same stable RPC error envelope.
    fn internal(error: impl std::fmt::Display) -> Self {
        super::error::ApiError::Internal(error.to_string()).into()
    }
}

pub type RpcResult<T> = Result<T, RpcError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

/// Wire-safe result for the unlock flow. The Tauri bridge preserves the
/// existing HTTP-shaped {status, body} UI contract at the presentation edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnlockConfirmOutcome {
    TooEarly(TooEarlyResponse),
    Unlocked(UnlockChallengeConfirmResponse),
}

/// One remote trait method per original control operation. This is *not*
/// a JSON command router: Remoc generates request enums and dispatch.
#[remoc::rtc::remote]
pub trait EngineControl {
    async fn get_account_info(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<AccountInfoResponse>, CallError>;
    async fn refresh_account(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<AccountInfoResponse>, CallError>;
    async fn accept_tos(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError>;
    async fn set_api_key(
        &self,
        capability: Option<String>,
        payload: SetApiKeyRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn logout(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError>;
    async fn export_backup(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<crate::manager::network::backup::EngineBackup>, CallError>;
    async fn export_backup_to_cloud(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError>;
    async fn import_backup_from_cloud(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError>;
    async fn import_backup(
        &self,
        capability: Option<String>,
        payload: crate::manager::network::backup::EngineBackup,
    ) -> Result<RpcResult<()>, CallError>;
    async fn get_dashboard_snapshot(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<DashboardSnapshotResponse>, CallError>;
    async fn get_debug_pro_policy(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<crate::manager::network::DebugProPolicy>, CallError>;
    async fn set_debug_pro_policy(
        &self,
        capability: Option<String>,
        policy: crate::manager::network::DebugProPolicy,
    ) -> Result<RpcResult<()>, CallError>;
    async fn hide_files(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError>;
    async fn show_files(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError>;
    async fn get_update_check_interval(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<UpdateCheckIntervalResponse>, CallError>;
    async fn set_update_check_interval(
        &self,
        capability: Option<String>,
        seconds: u64,
    ) -> Result<RpcResult<()>, CallError>;
    async fn reset_tos(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError>;
    async fn get_lock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetNetworkLockResponse>, CallError>;
    async fn set_lock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetNetworkLockRequest,
    ) -> Result<RpcResult<SetNetworkLockResponse>, CallError>;
    async fn get_net_block_to_site_pack_connections(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetNetListToSitePacksResponse>, CallError>;
    async fn add_net_block_to_site_pack_connection(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: AddNetListToSitePackRequest,
    ) -> Result<RpcResult<AddNetListToSitePackResponse>, CallError>;
    async fn remove_net_block_to_site_pack_connection(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        site_pack_id: String,
    ) -> Result<RpcResult<()>, CallError>;
    async fn rename_network_block(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: RenameNetworkBlockRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn get_all_network_blocks(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<GetAllNetworkBlocksResponse>, CallError>;
    async fn create_network_block(
        &self,
        capability: Option<String>,
        payload: CreateNetworkBlockRequest,
    ) -> Result<RpcResult<CreateNetworkBlockResponse>, CallError>;
    async fn save_network_block(
        &self,
        capability: Option<String>,
        payload: SaveNetworkBlockRequest,
    ) -> Result<RpcResult<SaveNetworkBlockResponse>, CallError>;
    async fn delete_network_block(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError>;
    async fn set_network_block_safe_search(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn set_network_block_enabled(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn set_network_block_icon(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetIconRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn get_resistance_mode_settings(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<ResistanceModeSettings>, CallError>;
    async fn set_resistance_mode_enabled(
        &self,
        capability: Option<String>,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn set_resistance_mode_lock(
        &self,
        capability: Option<String>,
        payload: SetNetworkLockRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn get_all_scripts(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<GetAllScriptsResponse>, CallError>;
    async fn get_script(
        &self,
        capability: Option<String>,
        script_id: String,
    ) -> Result<RpcResult<ScriptInfo>, CallError>;
    async fn set_script_enabled(
        &self,
        capability: Option<String>,
        script_id: String,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn download_resistance_scripts(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError>;
    async fn get_schedule_entries(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetScheduleEntriesResponse>, CallError>;
    async fn add_schedule_entries(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: AddScheduleEntriesRequest,
    ) -> Result<RpcResult<()>, CallError>;
    async fn edit_schedule_entry(
        &self,
        capability: Option<String>,
        schedule_entry_id: uuid::Uuid,
        entry: BaseScheduleEntrySchema,
    ) -> Result<RpcResult<()>, CallError>;
    async fn delete_schedule_entry(
        &self,
        capability: Option<String>,
        schedule_entry_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError>;
    async fn get_all_site_packs(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<GetAllSitePacksResponse>, CallError>;
    async fn create_site_pack(
        &self,
        capability: Option<String>,
        payload: CreateSitePackRequest,
    ) -> Result<RpcResult<remoc::rch::mpsc::Receiver<SitePackCreationStatus>>, CallError>;
    async fn delete_site_pack(
        &self,
        capability: Option<String>,
        site_pack_id: String,
    ) -> Result<RpcResult<()>, CallError>;
    async fn resync_site_pack(
        &self,
        capability: Option<String>,
        site_pack_id: String,
    ) -> Result<RpcResult<remoc::rch::mpsc::Receiver<SitePackCreationStatus>>, CallError>;
    async fn get_site_pack_sources(
        &self,
        capability: Option<String>,
        site_pack_id: String,
    ) -> Result<RpcResult<GetSitePackSourcesResponse>, CallError>;
    async fn add_site_pack_source(
        &self,
        capability: Option<String>,
        site_pack_id: String,
        payload: AddSitePackSourceRequest,
    ) -> Result<RpcResult<AddSitePackSourceResponse>, CallError>;
    async fn delete_site_pack_source(
        &self,
        capability: Option<String>,
        source_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError>;
    async fn re_enable_remote_site_packs(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError>;
    async fn ping(&self, capability: Option<String>) -> Result<RpcResult<String>, CallError>;
    async fn get_version(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<VersionInfo>, CallError>;
    async fn open_pro_page(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<String>, CallError>;
    async fn check_updates(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<UpdateCheckResponse>, CallError>;
    async fn trigger_content_update(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError>;
    async fn shutdown(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError>;
    async fn initiate_unlock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<UnlockChallengeInitiateResponse>, CallError>;
    async fn confirm_unlock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: UnlockChallengeConfirmRequest,
    ) -> Result<RpcResult<UnlockConfirmOutcome>, CallError>;
    async fn get_user_rules(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetUserDefinedRulesResponse>, CallError>;
    async fn add_user_rules(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: AddUserDefinedRulesRequest,
    ) -> Result<RpcResult<AddUserDefinedRulesResponse>, CallError>;
    async fn delete_user_rule(
        &self,
        capability: Option<String>,
        user_rule_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError>;
}

pub struct EngineRpc {
    pub state: AppState,
}

impl EngineRpc {
    fn check_capability(&self, presented: Option<&str>, required: bool) -> RpcResult<()> {
        if !required {
            return Ok(());
        }
        let Some(expected) = self.state.capability_token.as_deref() else {
            return if self.state.enforce_capability {
                Err(RpcError::forbidden())
            } else {
                Ok(())
            };
        };
        match presented {
            Some(token) if super::security::tokens_match(token, expected) => Ok(()),
            None if !self.state.enforce_capability => Ok(()),
            _ => Err(RpcError::forbidden()),
        }
    }
}

impl EngineControl for EngineRpc {
    async fn get_account_info(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<AccountInfoResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(Ok(super::operations_account::get_account_info(
            self.state.clone(),
        )
        .await))
    }

    async fn refresh_account(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<AccountInfoResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(Ok(super::operations_account::refresh_account(
            self.state.clone(),
        )
        .await))
    }

    async fn accept_tos(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_account::accept_tos(self.state.clone())
            .await
            .map(|_| ())
            .map_err(Into::into))
    }

    async fn set_api_key(
        &self,
        capability: Option<String>,
        payload: SetApiKeyRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_account::set_api_key(self.state.clone(), payload)
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn logout(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_account::logout(self.state.clone())
            .await
            .map(|_| ())
            .map_err(Into::into))
    }

    async fn export_backup(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<crate::manager::network::backup::EngineBackup>, CallError> {
        // Export includes policy names, rules and schedules; this is a
        // sensitive read, not a public health/status endpoint.
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_backup::export_backup(self.state.clone())
            .await
            .map_err(Into::into))
    }

    async fn export_backup_to_cloud(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_backup::export_backup_to_cloud(self.state.clone())
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn import_backup_from_cloud(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_backup::import_backup_from_cloud(self.state.clone())
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn import_backup(
        &self,
        capability: Option<String>,
        payload: crate::manager::network::backup::EngineBackup,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_backup::import_backup(self.state.clone(), payload)
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn get_dashboard_snapshot(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<DashboardSnapshotResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_dashboard::get_dashboard_snapshot(self.state.clone())
                .await
                .map_err(Into::into),
        )
    }

    async fn get_debug_pro_policy(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<crate::manager::network::DebugProPolicy>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(debug_assertions)]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), false) {
                return Ok(Err(error));
            }
            Ok(
                super::operations_dev::get_debug_pro_policy(self.state.clone())
                    .await
                    .map_err(Into::into),
            )
        }
    }

    async fn set_debug_pro_policy(
        &self,
        capability: Option<String>,
        policy: crate::manager::network::DebugProPolicy,
    ) -> Result<RpcResult<()>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(debug_assertions)]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), true) {
                return Ok(Err(error));
            }
            Ok(
                super::operations_dev::set_debug_pro_policy(self.state.clone(), policy)
                    .await
                    .map_err(Into::into),
            )
        }
    }

    async fn hide_files(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(all(debug_assertions, target_os = "windows"))]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), true) {
                return Ok(Err(error));
            }
            Ok(super::operations_dev::hide_files()
                .await
                .map_err(Into::into))
        }
        #[cfg(all(debug_assertions, not(target_os = "windows")))]
        {
            let _ = capability;
            Ok(Err(RpcError::internal(
                "file visibility controls are only available on Windows",
            )))
        }
    }

    async fn show_files(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(all(debug_assertions, target_os = "windows"))]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), true) {
                return Ok(Err(error));
            }
            Ok(super::operations_dev::show_files()
                .await
                .map_err(Into::into))
        }
        #[cfg(all(debug_assertions, not(target_os = "windows")))]
        {
            let _ = capability;
            Ok(Err(RpcError::internal(
                "file visibility controls are only available on Windows",
            )))
        }
    }

    async fn get_update_check_interval(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<UpdateCheckIntervalResponse>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(debug_assertions)]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), false) {
                return Ok(Err(error));
            }
            Ok(
                super::operations_dev::get_update_check_interval(self.state.clone())
                    .await
                    .map_err(Into::into),
            )
        }
    }

    async fn set_update_check_interval(
        &self,
        capability: Option<String>,
        seconds: u64,
    ) -> Result<RpcResult<()>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(debug_assertions)]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), true) {
                return Ok(Err(error));
            }
            Ok(
                super::operations_dev::set_update_check_interval(self.state.clone(), seconds)
                    .await
                    .map_err(Into::into),
            )
        }
    }

    async fn reset_tos(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError> {
        #[cfg(not(debug_assertions))]
        {
            let _ = capability;
            return Ok(Err(RpcError::internal(
                "developer operations are not available in release builds",
            )));
        }
        #[cfg(debug_assertions)]
        {
            if let Err(error) = self.check_capability(capability.as_deref(), true) {
                return Ok(Err(error));
            }
            Ok(super::operations_dev::reset_tos(self.state.clone())
                .await
                .map_err(Into::into))
        }
    }

    async fn get_lock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetNetworkLockResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_lock::get_lock(self.state.clone(), network_block_id)
                .await
                .map_err(Into::into),
        )
    }

    async fn set_lock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetNetworkLockRequest,
    ) -> Result<RpcResult<SetNetworkLockResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_lock::set_lock(self.state.clone(), network_block_id, payload)
                .await
                .map_err(Into::into),
        )
    }

    async fn get_net_block_to_site_pack_connections(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetNetListToSitePacksResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_net_blocks_to_site_packs::get_net_block_to_site_pack_connections(
                self.state.clone(),
                network_block_id,
            )
            .await
            .map_err(Into::into),
        )
    }

    async fn add_net_block_to_site_pack_connection(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: AddNetListToSitePackRequest,
    ) -> Result<RpcResult<AddNetListToSitePackResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_net_blocks_to_site_packs::add_net_block_to_site_pack_connection(
                self.state.clone(),
                network_block_id,
                payload,
            )
            .await
            .map_err(Into::into),
        )
    }

    async fn remove_net_block_to_site_pack_connection(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        site_pack_id: String,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_net_blocks_to_site_packs::remove_net_block_to_site_pack_connection(
                self.state.clone(),
                (network_block_id, site_pack_id),
            )
            .await
            .map(|_| ())
            .map_err(Into::into),
        )
    }

    async fn rename_network_block(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: RenameNetworkBlockRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_network_blocks::rename_network_block(
            self.state.clone(),
            network_block_id,
            payload,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn get_all_network_blocks(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<GetAllNetworkBlocksResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(Ok(
            super::operations_network_blocks::get_all_network_blocks(self.state.clone()).await,
        ))
    }

    async fn create_network_block(
        &self,
        capability: Option<String>,
        payload: CreateNetworkBlockRequest,
    ) -> Result<RpcResult<CreateNetworkBlockResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_network_blocks::create_network_block(self.state.clone(), payload)
                .await
                .map_err(Into::into),
        )
    }

    async fn save_network_block(
        &self,
        capability: Option<String>,
        payload: SaveNetworkBlockRequest,
    ) -> Result<RpcResult<SaveNetworkBlockResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_network_blocks::save_network_block(self.state.clone(), payload)
                .await
                .map_err(Into::into),
        )
    }

    async fn delete_network_block(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_network_blocks::delete_network_block(
            self.state.clone(),
            network_block_id,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn set_network_block_safe_search(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_network_blocks::set_network_block_safe_search(
                self.state.clone(),
                network_block_id,
                payload,
            )
            .await
            .map(|_| ())
            .map_err(Into::into),
        )
    }

    async fn set_network_block_enabled(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_network_blocks::set_network_block_enabled(
            self.state.clone(),
            network_block_id,
            payload,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn set_network_block_icon(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: SetIconRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_network_blocks::set_network_block_icon(
            self.state.clone(),
            network_block_id,
            payload,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn get_resistance_mode_settings(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<ResistanceModeSettings>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(Ok(
            super::operations_resistance_mode::get_resistance_mode_settings(self.state.clone())
                .await,
        ))
    }

    async fn set_resistance_mode_enabled(
        &self,
        capability: Option<String>,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_resistance_mode::set_resistance_mode_enabled(
                self.state.clone(),
                payload,
            )
            .await
            .map(|_| ())
            .map_err(Into::into),
        )
    }

    async fn set_resistance_mode_lock(
        &self,
        capability: Option<String>,
        payload: SetNetworkLockRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_resistance_mode::set_resistance_mode_lock(
                self.state.clone(),
                payload,
            )
            .await
            .map(|_| ())
            .map_err(Into::into),
        )
    }

    async fn get_all_scripts(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<GetAllScriptsResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(Ok(super::operations_resistance_mode::get_all_scripts(
            self.state.clone(),
        )
        .await))
    }

    async fn get_script(
        &self,
        capability: Option<String>,
        script_id: String,
    ) -> Result<RpcResult<ScriptInfo>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_resistance_mode::get_script(self.state.clone(), script_id)
                .await
                .map_err(Into::into),
        )
    }

    async fn set_script_enabled(
        &self,
        capability: Option<String>,
        script_id: String,
        payload: SetEnabledRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_resistance_mode::set_script_enabled(
            self.state.clone(),
            script_id,
            payload,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn download_resistance_scripts(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_resistance_mode::download_resistance_scripts(self.state.clone())
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn get_schedule_entries(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetScheduleEntriesResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_schedule::get_schedule_entries(self.state.clone(), network_block_id)
                .await
                .map_err(Into::into),
        )
    }

    async fn add_schedule_entries(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: AddScheduleEntriesRequest,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_schedule::add_schedule_entries(
            self.state.clone(),
            network_block_id,
            payload,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn edit_schedule_entry(
        &self,
        capability: Option<String>,
        schedule_entry_id: uuid::Uuid,
        entry: BaseScheduleEntrySchema,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_schedule::edit_schedule_entry(
            self.state.clone(),
            schedule_entry_id,
            entry,
        )
        .await
        .map(|_| ())
        .map_err(Into::into))
    }

    async fn delete_schedule_entry(
        &self,
        capability: Option<String>,
        schedule_entry_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_schedule::delete_schedule_entry(
                self.state.clone(),
                schedule_entry_id,
            )
            .await
            .map(|_| ())
            .map_err(Into::into),
        )
    }

    async fn get_all_site_packs(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<GetAllSitePacksResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_site_pack::get_all_site_packs(self.state.clone())
                .await
                .map_err(Into::into),
        )
    }

    async fn create_site_pack(
        &self,
        capability: Option<String>,
        payload: CreateSitePackRequest,
    ) -> Result<RpcResult<remoc::rch::mpsc::Receiver<SitePackCreationStatus>>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            match super::operations_site_pack::create_site_pack(self.state.clone(), payload).await {
                Ok(stream) => {
                    use futures_util::StreamExt;
                    let mut body = stream;
                    let (tx, rx) = remoc::rch::mpsc::channel();
                    tokio::spawn(async move {
                        while let Some(status) = body.next().await {
                            if tx.send(status).await.is_err() {
                                break;
                            }
                        }
                    });
                    Ok(rx)
                }
                Err(error) => Err(error.into()),
            },
        )
    }

    async fn delete_site_pack(
        &self,
        capability: Option<String>,
        site_pack_id: String,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_site_pack::delete_site_pack(self.state.clone(), site_pack_id)
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn resync_site_pack(
        &self,
        capability: Option<String>,
        site_pack_id: String,
    ) -> Result<RpcResult<remoc::rch::mpsc::Receiver<SitePackCreationStatus>>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            match super::operations_site_pack::resync_site_pack(self.state.clone(), site_pack_id)
                .await
            {
                Ok(stream) => {
                    use futures_util::StreamExt;
                    let mut body = stream;
                    let (tx, rx) = remoc::rch::mpsc::channel();
                    tokio::spawn(async move {
                        while let Some(status) = body.next().await {
                            if tx.send(status).await.is_err() {
                                break;
                            }
                        }
                    });
                    Ok(rx)
                }
                Err(error) => Err(error.into()),
            },
        )
    }

    async fn get_site_pack_sources(
        &self,
        capability: Option<String>,
        site_pack_id: String,
    ) -> Result<RpcResult<GetSitePackSourcesResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_site_pack::get_site_pack_sources(self.state.clone(), site_pack_id)
                .await
                .map_err(Into::into),
        )
    }

    async fn add_site_pack_source(
        &self,
        capability: Option<String>,
        site_pack_id: String,
        payload: AddSitePackSourceRequest,
    ) -> Result<RpcResult<AddSitePackSourceResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_site_pack::add_site_pack_source(
            self.state.clone(),
            site_pack_id,
            payload,
        )
        .await
        .map_err(Into::into))
    }

    async fn delete_site_pack_source(
        &self,
        capability: Option<String>,
        source_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_site_pack::delete_site_pack_source(self.state.clone(), source_id)
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn re_enable_remote_site_packs(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_site_pack::re_enable_remote_site_packs(self.state.clone())
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn ping(&self, capability: Option<String>) -> Result<RpcResult<String>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(Ok(super::operations_system::ping().await.to_string()))
    }

    async fn get_version(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<VersionInfo>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(Ok(super::operations_system::get_version().await))
    }

    async fn open_pro_page(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<String>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(super::operations_system::open_pro_page()
            .await
            .map(str::to_owned)
            .map_err(Into::into))
    }

    async fn check_updates(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<UpdateCheckResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(super::operations_system::check_updates(self.state.clone())
            .await
            .map_err(Into::into))
    }

    async fn trigger_content_update(
        &self,
        capability: Option<String>,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_system::trigger_content_update(self.state.clone())
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

    async fn shutdown(&self, capability: Option<String>) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_system::shutdown(self.state.clone())
            .await
            .map(|_| ())
            .map_err(Into::into))
    }

    async fn initiate_unlock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<UnlockChallengeInitiateResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_unlock::initiate_unlock(self.state.clone(), network_block_id)
                .await
                .map_err(Into::into),
        )
    }

    async fn confirm_unlock(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: UnlockChallengeConfirmRequest,
    ) -> Result<RpcResult<UnlockConfirmOutcome>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_unlock::confirm_unlock(self.state.clone(), network_block_id, payload)
                .await
                .map_err(Into::into),
        )
    }

    async fn get_user_rules(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
    ) -> Result<RpcResult<GetUserDefinedRulesResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), false) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_user_rules::get_user_rules(self.state.clone(), network_block_id)
                .await
                .map_err(Into::into),
        )
    }

    async fn add_user_rules(
        &self,
        capability: Option<String>,
        network_block_id: uuid::Uuid,
        payload: AddUserDefinedRulesRequest,
    ) -> Result<RpcResult<AddUserDefinedRulesResponse>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(super::operations_user_rules::add_user_rules(
            self.state.clone(),
            network_block_id,
            payload,
        )
        .await
        .map_err(Into::into))
    }

    async fn delete_user_rule(
        &self,
        capability: Option<String>,
        user_rule_id: uuid::Uuid,
    ) -> Result<RpcResult<()>, CallError> {
        if let Err(error) = self.check_capability(capability.as_deref(), true) {
            return Ok(Err(error));
        }
        Ok(
            super::operations_user_rules::delete_user_rule(self.state.clone(), user_rule_id)
                .await
                .map(|_| ())
                .map_err(Into::into),
        )
    }

}

#[cfg(test)]
mod remoc_tests {
    use super::*;
    use remoc::ConnectExt;
    use remoc::rtc::ServerShared;
    use std::sync::Arc;
    use std::time::Duration;

    fn state() -> (tempfile::TempDir, AppState) {
        let temp = tempfile::tempdir().unwrap();
        crate::settings::Settings::set_test_sites_dir(
            crate::settings::Settings::shared_test_sites_dir(),
        );
        let db =
            Arc::new(crate::db::EngineDB::from(&temp.path().join("test.sqlite")).expect("test DB"));
        let account = crate::manager::AccountManager::from(db.clone()).expect("test account");
        let state = AppState {
            network_manager: Arc::new(tokio::sync::RwLock::new(
                crate::manager::NetworkManager::new_empty(db.clone(), account.clone()),
            )),
            account_manager: account,
            content_update_service: Arc::new(tokio::sync::Mutex::new(
                crate::service::content_updater::ContentUpdateService::from(db),
            )),
            update_check_interval: Arc::new(tokio::sync::RwLock::new(Duration::from_secs(3600))),
            shutdown_tx: tokio::sync::watch::channel(false).0,
            unlock_challenges: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            capability_token: Some("test-256-bit-capability".into()),
            enforce_capability: true,
        };
        (temp, state)
    }

    // Exercise the actual production RTC trait and Remoc's byte-stream
    // connection lifecycle without starting a privileged Windows service.
    async fn client(state: AppState) -> (EngineControlClient, tokio::task::JoinHandle<()>) {
        let (left, right) = tokio::io::duplex(64 * 1024);
        let task = tokio::spawn(async move {
            let (rx, tx) = tokio::io::split(left);
            let (server, client) =
                EngineControlServerShared::<_, remoc::codec::Default>::new(Arc::new(EngineRpc {
                    state,
                }));
            remoc::Connect::io(remoc::Cfg::default(), rx, tx)
                .provide(client)
                .await
                .expect("provide RTC capability");
            server.serve().await.expect("serve RTC");
        });
        let (rx, tx) = tokio::io::split(right);
        let remote: EngineControlClient = remoc::Connect::io(remoc::Cfg::default(), rx, tx)
            .consume()
            .await
            .expect("consume RTC capability");
        (remote, task)
    }

    #[tokio::test]
    async fn dashboard_snapshot_round_trips_over_remoc() {
        let (_temp, state) = state();
        let (client, server) = client(state).await;
        let created = client
            .create_network_block(
                Some("test-256-bit-capability".to_owned()),
                CreateNetworkBlockRequest {
                    block_name: "Dashboard RPC regression".to_owned(),
                    icon: None,
                },
            )
            .await
            .expect("create network block over remoc")
            .expect("create dashboard fixture");
        let snapshot = client
            .get_dashboard_snapshot(None)
            .await
            .expect("deserialize dashboard snapshot")
            .expect("dashboard operation");
        assert_eq!(snapshot.blocks.len(), 1);
        assert_eq!(snapshot.blocks[0].block.id, created.network_block.id);
        assert_eq!(
            serde_json::to_value(&snapshot).unwrap()["resistanceModeSettings"]["enabled"],
            false
        );

        drop(client);
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server shutdown deadline")
            .unwrap();
    }

    #[tokio::test]
    async fn remote_calls_preserve_authentication_and_error_codes() {
        let (_temp, state) = state();
        let (client, server) = client(state).await;
        assert_eq!(client.ping(None).await.unwrap().unwrap(), "pong");
        assert!(client.get_all_network_blocks(None).await.unwrap().is_ok());

        let rejected = client.accept_tos(None).await.unwrap().unwrap_err();
        assert_eq!(rejected.status, 401);
        assert_eq!(rejected.code, "capability_required");

        let rejected = client
            .accept_tos(Some("wrong".into()))
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(rejected.status, 401);

        client
            .accept_tos(Some("test-256-bit-capability".into()))
            .await
            .unwrap()
            .expect("accept ToS with authorized caller");
        drop(client);
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server shutdown deadline")
            .unwrap();
    }

    #[tokio::test]
    async fn independent_clients_and_engine_restart() {
        let (_temp, state) = state();
        let (left, task_a) = client(state.clone()).await;
        let (right, task_b) = client(state.clone()).await;
        let calls =
            futures_util::future::join(left.get_version(None), right.get_version(None)).await;
        assert!(calls.0.unwrap().is_ok());
        assert!(calls.1.unwrap().is_ok());
        drop(left);
        drop(right);
        tokio::time::timeout(Duration::from_secs(5), async {
            task_a.await.unwrap();
            task_b.await.unwrap();
        })
        .await
        .expect("clients disconnect");

        let (reconnected, task_c) = client(state).await;
        assert_eq!(reconnected.ping(None).await.unwrap().unwrap(), "pong");
        drop(reconnected);
        tokio::time::timeout(Duration::from_secs(5), task_c)
            .await
            .expect("reconnected session cleanup")
            .unwrap();
    }

    #[tokio::test]
    async fn remote_operations_preserve_lock_and_cooldown_enforcement() {
        let (_temp, state) = state();
        let (client, server) = client(state).await;
        let capability = Some("test-256-bit-capability".to_owned());

        let created = client
            .create_network_block(
                capability.clone(),
                CreateNetworkBlockRequest {
                    block_name: "RPC lock regression".to_owned(),
                    icon: None,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let block_id = created.network_block.id;
        assert!(
            client
                .get_all_network_blocks(None)
                .await
                .unwrap()
                .unwrap()
                .network_blocks
                .iter()
                .any(|block| block.id == block_id)
        );

        client
            .set_lock(
                capability.clone(),
                block_id,
                SetNetworkLockRequest {
                    lock_state: "soft_unlockable".to_owned(),
                    lock_end: None,
                },
            )
            .await
            .unwrap()
            .unwrap();

        let delete_error = client
            .delete_network_block(capability.clone(), block_id)
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(delete_error.status, 403);

        let shutdown_error = client
            .shutdown(capability.clone())
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(shutdown_error.status, 423);
        assert_eq!(shutdown_error.code, "locked");

        let challenge = client
            .initiate_unlock(capability.clone(), block_id)
            .await
            .unwrap()
            .unwrap();
        let confirmation = client
            .confirm_unlock(
                capability,
                block_id,
                UnlockChallengeConfirmRequest {
                    challenge_id: challenge.challenge_id,
                },
            )
            .await
            .unwrap()
            .unwrap();
        match confirmation {
            UnlockConfirmOutcome::TooEarly(response) => {
                assert!(response.retry_after_seconds > 0);
            }
            UnlockConfirmOutcome::Unlocked(_) => panic!("cooldown was bypassed"),
        }

        drop(client);
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server shutdown deadline")
            .unwrap();
    }

    #[tokio::test]
    async fn site_pack_import_stream_transmits_typed_statuses_over_remoc() {
        let (temp, state) = state();
        state
            .network_manager
            .write()
            .await
            .set_debug_pro_policy(crate::manager::network::DebugProPolicy::Pro);
        let source = temp.path().join("tiny-hosts.txt");
        std::fs::write(&source, "0.0.0.0 example.test\n0.0.0.0 another.example\n")
            .expect("write local test source");

        let (client, server) = client(state).await;
        let mut statuses = client
            .create_site_pack(
                Some("test-256-bit-capability".to_owned()),
                CreateSitePackRequest {
                    name: "Remoc event regression".to_owned(),
                    description: None,
                    icon: None,
                    custom_folder: None,
                    sources: vec![BaseSitePackSourceSchema {
                        source: source.to_string_lossy().to_string(),
                        source_type: SitePackSourceType::File,
                        format: SitePackFormat::Standard,
                    }],
                },
            )
            .await
            .expect("remoc call")
            .expect("valid site pack request");

        let first = tokio::time::timeout(Duration::from_secs(10), statuses.recv())
            .await
            .expect("stream event deadline")
            .expect("remoc receive")
            .expect("stream emitted at least one event");
        assert!(
            matches!(first, SitePackCreationStatus::Started { .. }),
            "first event was {first:?}"
        );
        let mut terminal = None;
        for _ in 0..100 {
            let status = tokio::time::timeout(Duration::from_secs(10), statuses.recv())
                .await
                .expect("stream event deadline")
                .expect("remoc receive");
            match status {
                Some(SitePackCreationStatus::Completed { .. }) => {
                    terminal = Some("completed");
                    break;
                }
                Some(SitePackCreationStatus::Failed { error, .. }) => {
                    panic!("test import failed: {error}");
                }
                Some(_) => {}
                None => break,
            }
        }
        assert_eq!(terminal, Some("completed"));
        drop(statuses);
        drop(client);
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server shutdown deadline")
            .unwrap();
    }

    #[tokio::test]
    async fn remote_mutations_validate_request_payloads() {
        let (_temp, state) = state();
        let (client, server) = client(state).await;
        let capability = Some("test-256-bit-capability".to_owned());

        let error = client
            .set_api_key(
                capability.clone(),
                SetApiKeyRequest {
                    api_key: "".to_owned(),
                },
            )
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.status, 400);
        assert_eq!(error.code, "bad_request");

        let error = client
            .create_site_pack(
                capability,
                CreateSitePackRequest {
                    name: "Empty sources".to_owned(),
                    description: None,
                    icon: None,
                    custom_folder: None,
                    sources: Vec::new(),
                },
            )
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.code, "bad_request");

        drop(client);
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server shutdown deadline")
            .unwrap();
    }
}
