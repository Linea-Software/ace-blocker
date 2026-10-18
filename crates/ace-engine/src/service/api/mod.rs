use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{NetworkBlockId, common::unlock_challenge::UnlockChallenge};
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
