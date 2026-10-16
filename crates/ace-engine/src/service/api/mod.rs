use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::core::{NetworkBlockId, common::unlock_challenge::UnlockChallenge};
use crate::manager::{AccountManager, NetworkManager};

pub mod docs;
pub mod error;
pub mod route_account;
pub mod route_backup;
pub mod route_dashboard;
#[cfg(debug_assertions)]
pub mod route_dev;
pub mod route_lock;
pub mod route_net_blocks_to_site_packs;
pub mod route_network_blocks;
pub mod route_resistance_mode;
pub mod route_schedule;
pub mod route_site_pack;
pub mod route_system;
pub mod route_unlock;
pub mod route_user_rules;
pub mod schemas;
pub mod security;

use crate::service::content_updater::{SharedContentUpdateService, SharedUpdateInterval};
use docs::ApiDoc;
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
    /// Per-install capability token required on state-changing requests.
    /// `Some` enables enforcement; `None` (test servers) disables it.
    /// See `docs/local-api-protection-design.md`.
    pub capability_token: Option<String>,
    /// Whether token-less requests are rejected. Release builds enforce;
    /// debug builds accept token-less requests so browser-based development
    /// (no Tauri IPC to read the token file) keeps working — same convention
    /// as the compile-gated `/dev/*` endpoints.
    pub enforce_capability: bool,
}

pub fn create_router(state: AppState) -> Router {
    let router = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(route_network_blocks::router())
        .merge(route_site_pack::router())
        .merge(route_user_rules::router())
        .merge(route_schedule::router())
        .merge(route_lock::router())
        .merge(route_net_blocks_to_site_packs::router())
        .merge(route_account::router())
        .merge(route_resistance_mode::router())
        .merge(route_backup::router())
        .merge(route_system::router())
        .merge(route_unlock::router())
        .merge(route_dashboard::router());

    #[cfg(debug_assertions)]
    let router = router.merge(route_dev::router());

    // Security layers (innermost first): capability token on state-changing
    // routes, then the origin-aware CORS policy. See `security.rs` and
    // `docs/local-api-protection-design.md`.
    router
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::capability_check,
        ))
        .layer(axum::middleware::from_fn(security::cors_policy))
        .with_state(state)
}
