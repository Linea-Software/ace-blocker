use crate::core::common::injectable_scripts::ScriptId;
use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn get_resistance_mode_settings(state: AppState) -> ResistanceModeSettings {
    let manager = state.network_manager.read().await;
    let engine = manager.get_resistance_mode();
    let punishment = engine.get_punishment();

    let current_intensity = manager.get_effective_intensity();

    let entries = engine.script_tree.get_all_script_entries();
    let scripts_total = entries.len() as u32;
    let scripts_enabled = entries.iter().filter(|entry| entry.enabled).count() as u32;

    ResistanceModeSettings {
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
    }
}

pub(super) async fn set_resistance_mode_enabled(
    state: AppState,
    payload: SetEnabledRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.set_resistance_mode_enabled(payload.enabled)?;
    Ok(())
}

pub(super) async fn set_resistance_mode_lock(
    state: AppState,
    payload: SetNetworkLockRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    let lock = crate::core::Lock::from_raw(payload.lock_state, payload.lock_end)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    manager.set_resistance_mode_lock(lock)?;
    Ok(())
}

pub(super) async fn get_all_scripts(state: AppState) -> GetAllScriptsResponse {
    let manager = state.network_manager.read().await;

    let entries = manager
        .get_resistance_mode()
        .script_tree
        .get_all_script_entries();

    let scripts = entries.into_iter().map(ScriptInfo::from).collect();

    GetAllScriptsResponse { scripts }
}

pub(super) async fn get_script(state: AppState, script_id: String) -> Result<ScriptInfo, ApiError> {
    let manager = state.network_manager.read().await;
    let engine = manager.get_resistance_mode();

    let script_id = ScriptId::from(script_id);

    if let Some(entry) = engine.script_tree.get_script_by_id(&script_id) {
        Ok(ScriptInfo {
            id: (*entry.id).to_string(),
            domain: entry.domain,
            name: entry.name,
            description: entry.description,
            is_enabled: entry.enabled,
        })
    } else {
        Err(ApiError::NotFound(format!(
            "Script with ID {} not found",
            *script_id
        )))
    }
}

pub(super) async fn set_script_enabled(
    state: AppState,
    script_id: String,
    payload: SetEnabledRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    let script_id = ScriptId::from(script_id);

    match manager.set_script_enabled(&script_id, payload.enabled) {
        Ok(true) => Ok(()),
        Ok(false) => Err(ApiError::NotFound(format!(
            "Script with ID {} not found",
            *script_id
        ))),
        Err(error) => Err(ApiError::from(error)),
    }
}

pub(super) async fn download_resistance_scripts(state: AppState) -> Result<(), ApiError> {
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
    Ok(())
}
