use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};

use crate::core::common::injectable_scripts::ScriptId;
use crate::service::api::{AppState, error::ApiError, schemas::*};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/resistance-mode/settings",
            get(get_resistance_mode_settings),
        )
        .route(
            "/resistance-mode/set-enabled",
            post(set_resistance_mode_enabled),
        )
        .route("/resistance-mode/set-lock", post(set_resistance_mode_lock))
        .route("/resistance-mode/scripts", get(get_all_scripts))
        .route("/resistance-mode/scripts/{script_id}", get(get_script))
        .route(
            "/resistance-mode/scripts/{script_id}/set-enabled",
            post(set_script_enabled),
        )
        .route(
            "/resistance-mode/download-scripts",
            post(download_resistance_scripts),
        )
}

#[utoipa::path(
    get,
    path = "/resistance-mode/settings",
    tag = "Resistance Mode",
    responses(
        (status = 200, description = "Get resistance engine settings", body = ResistanceModeSettings)
    )
)]
pub(super) async fn get_resistance_mode_settings(
    State(state): State<AppState>,
) -> Json<ResistanceModeSettings> {
    let manager = state.network_manager.read().await;
    let engine = manager.get_resistance_mode();
    let punishment = engine.get_punishment();

    let current_intensity = manager.get_effective_intensity();

    let entries = engine.script_tree.get_all_script_entries();
    let scripts_total = entries.len() as u32;
    let scripts_enabled = entries.iter().filter(|entry| entry.enabled).count() as u32;

    Json(ResistanceModeSettings {
        enabled: engine.enabled,
        is_locked: engine.is_locked(),
        lock_state: engine.lock.get_state().into(),
        lock_end: engine
            .lock
            .get_end_time()
            .map(|dt: chrono::DateTime<chrono::Local>| dt.naive_local().to_string()),
        mode: punishment.mode,
        current_intensity,
        static_intensity: punishment.static_intensity,
        scripts_version: engine.scripts_version.clone(),
        scripts_loaded: !entries.is_empty(),
        scripts_total,
        scripts_enabled,
        pro_eligible: manager.is_pro_user_cached(),
        enforcement_status: manager.resistance_enforcement_status(),
    })
}

#[utoipa::path(
    post,
    path = "/resistance-mode/set-enabled",
    tag = "Resistance Mode",
    request_body = SetEnabledRequest,
    responses(
        (status = 200, description = "Resistance engine enabled/disabled successfully"),
        (status = 403, description = "Resistance engine is locked")
    )
)]
pub(super) async fn set_resistance_mode_enabled(
    State(state): State<AppState>,
    payload: crate::service::api::error::ValidatedJson<SetEnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.set_resistance_mode_enabled(payload.0.enabled)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/resistance-mode/set-lock",
    tag = "Resistance Mode",
    request_body = SetNetworkLockRequest,
    responses(
        (status = 200, description = "Resistance engine lock updated successfully"),
        (status = 403, description = "Resistance engine is locked or disabled")
    )
)]
pub(super) async fn set_resistance_mode_lock(
    State(state): State<AppState>,
    payload: crate::service::api::error::ValidatedJson<SetNetworkLockRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    let lock = crate::core::Lock::from_raw(payload.0.lock_state, payload.0.lock_end)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    manager.set_resistance_mode_lock(lock)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/resistance-mode/scripts",
    tag = "Resistance Mode",
    responses(
        (status = 200, description = "Get all scripts", body = GetAllScriptsResponse)
    )
)]
pub(super) async fn get_all_scripts(State(state): State<AppState>) -> Json<GetAllScriptsResponse> {
    let manager = state.network_manager.read().await;

    let entries = manager
        .get_resistance_mode()
        .script_tree
        .get_all_script_entries();

    let scripts = entries.into_iter().map(ScriptInfo::from).collect();

    Json(GetAllScriptsResponse { scripts })
}

#[utoipa::path(
    get,
    path = "/resistance-mode/scripts/{script_id}",
    tag = "Resistance Mode",
    params(
        ("script_id" = String, Path, description = "Script ID")
    ),
    responses(
        (status = 200, description = "Get script by ID", body = ScriptInfo),
        (status = 404, description = "Script not found")
    )
)]
pub(super) async fn get_script(
    State(state): State<AppState>,
    Path(script_id): Path<String>,
) -> Result<Json<ScriptInfo>, ApiError> {
    let manager = state.network_manager.read().await;
    let engine = manager.get_resistance_mode();

    let script_id = ScriptId::from(script_id);

    if let Some(entry) = engine.script_tree.get_script_by_id(&script_id) {
        Ok(Json(ScriptInfo {
            id: (*entry.id).to_string(),
            domain: entry.domain,
            name: entry.name,
            description: entry.description,
            is_enabled: entry.enabled,
        }))
    } else {
        Err(ApiError::NotFound(format!(
            "Script with ID {} not found",
            *script_id
        )))
    }
}

#[utoipa::path(
    post,
    path = "/resistance-mode/scripts/{script_id}/set-enabled",
    tag = "Resistance Mode",
    params(
        ("script_id" = String, Path, description = "Script ID")
    ),
    request_body = SetEnabledRequest,
    responses(
        (status = 200, description = "Script enabled/disabled successfully"),
        (status = 404, description = "Script not found"),
        (status = 403, description = "Resistance engine is locked")
    )
)]
pub(super) async fn set_script_enabled(
    State(state): State<AppState>,
    Path(script_id): Path<String>,
    payload: crate::service::api::error::ValidatedJson<SetEnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    let script_id = ScriptId::from(script_id);

    match manager.set_script_enabled(&script_id, payload.0.enabled) {
        Ok(true) => Ok(StatusCode::OK),
        Ok(false) => Err(ApiError::NotFound(format!(
            "Script with ID {} not found",
            *script_id
        ))),
        Err(error) => Err(ApiError::from(error)),
    }
}

#[utoipa::path(
    post,
    path = "/resistance-mode/download-scripts",
    tag = "Resistance Mode",
    responses(
        (status = 200, description = "Resistance scripts downloaded successfully"),
    )
)]
pub(super) async fn download_resistance_scripts(
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    // Download, verify, and extract OUTSIDE the manager write lock (SVC-02);
    // only the commit — local disk swap + reload + version persist — happens
    // under it.
    let api_key = state
        .account_manager
        .get_api_key()
        .await
        .map_err(ApiError::Internal)?;
    let content_service = state.content_update_service.lock().await;
    let staged = content_service
        .stage_resistance_scripts(api_key.as_ref())
        .await
        .map_err(ApiError::from)?;
    drop(content_service);

    let mut manager = state.network_manager.write().await;
    manager
        .commit_resistance_scripts(staged)
        .map_err(ApiError::from)?;
    Ok(StatusCode::OK)
}
