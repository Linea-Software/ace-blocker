use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{NetworkBlockId, common::unlock_challenge::UnlockChallenge};
#[cfg(debug_assertions)]
use crate::manager::network::DebugProPolicy;
use crate::manager::{AccountManager, NetworkManager};

pub mod bridge;
pub mod error;
mod operations_account;
mod operations_backup;
mod operations_dashboard;
#[cfg(debug_assertions)]
mod operations_dev;
mod operations_lock;
mod operations_net_blocks_to_site_packs;
mod operations_network_blocks;
mod operations_resistance_mode;
mod operations_schedule;
mod operations_site_pack;
mod operations_system;
mod operations_unlock;
mod operations_user_rules;
pub mod rpc;
pub mod schemas;
pub mod security;
pub mod transport;

use crate::service::content_updater::{SharedContentUpdateService, SharedUpdateInterval};
use tokio::sync::watch;

#[derive(Clone)]
pub struct AppState {
    pub network_manager: Arc<RwLock<NetworkManager>>,
    pub account_manager: AccountManager,
    pub content_update_service: SharedContentUpdateService,
    pub update_check_interval: SharedUpdateInterval,
    pub shutdown_tx: watch::Sender<bool>,
    /// In-memory soft-unlock challenges, keyed by network block ID.
    /// Ephemeral: not persisted to DB. Cleared on engine restart.
    pub unlock_challenges: Arc<tokio::sync::Mutex<HashMap<NetworkBlockId, UnlockChallenge>>>,
    /// Per-install token required by modifying remoc RPC calls.
    /// `None` is reserved for test/development environments.
    pub capability_token: Option<String>,
    /// Whether token-less requests are rejected. Release builds enforce;
    /// debug builds may accept token-less development calls; release builds
    /// require the capability. Debug-only methods are separately compile-gated.
    pub enforce_capability: bool,
}

impl AppState {
    /// Initial account reads must never wait for the Ace server: legal
    /// acceptance and the desktop shell are local state. Background/explicit
    /// refresh remains responsible for remote entitlement validation.
    async fn cached_pro_account_entitled(&self) -> bool {
        #[cfg(debug_assertions)]
        match self.network_manager.read().await.get_debug_pro_policy() {
            DebugProPolicy::Pro => return true,
            DebugProPolicy::Free => return false,
            DebugProPolicy::Default => {}
        }

        self.account_manager.get_cached_pro_entitlement().await
    }

    /// Verify Pro entitlement without holding the policy lock across remote I/O.
    /// Debug overrides are read before and after verification so concurrent
    /// changes to an override cannot return an outdated effective policy.
    async fn is_pro_account_entitled(&self) -> bool {
        self.pro_account_entitled(false).await
    }

    /// Use the shorter, rate-limited refresh path on return from checkout.
    async fn refresh_pro_account_entitled(&self) -> bool {
        self.pro_account_entitled(true).await
    }

    async fn pro_account_entitled(&self, refresh: bool) -> bool {
        #[cfg(debug_assertions)]
        match self.network_manager.read().await.get_debug_pro_policy() {
            DebugProPolicy::Pro => return true,
            DebugProPolicy::Free => return false,
            DebugProPolicy::Default => {}
        }

        let verified = if refresh {
            self.account_manager.refresh_pro_status().await
        } else {
            self.account_manager.is_pro().await
        };
        #[cfg(debug_assertions)]
        match self.network_manager.read().await.get_debug_pro_policy() {
            DebugProPolicy::Pro => return true,
            DebugProPolicy::Free => return false,
            DebugProPolicy::Default => {}
        }
        verified
    }
}
