#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::time::Duration;

use crate::service::api::{AppState, error::ApiError};

/// Returns the parent of app data directory that should be hidden/shown for tamper protection
#[cfg(target_os = "windows")]
fn get_important_files() -> Vec<PathBuf> {
    use crate::settings::Settings;

    // Hide the parent directory to protect the entire app data structure
    let app_data_dir = Settings::get_app_data_dir();
    let parent_dir = app_data_dir
        .parent()
        .expect("App data dir should have a parent")
        .to_path_buf();

    vec![parent_dir]
}

pub(super) async fn get_debug_pro_policy(
    app_state: AppState,
) -> Result<crate::manager::network::DebugProPolicy, ApiError> {
    let manager = app_state.network_manager.read().await;
    Ok(manager.get_debug_pro_policy())
}

pub(super) async fn set_debug_pro_policy(
    app_state: AppState,
    policy: crate::manager::network::DebugProPolicy,
) -> Result<(), ApiError> {
    let mut manager = app_state.network_manager.write().await;
    manager.set_debug_pro_policy(policy);
    Ok(())
}

pub(super) async fn get_debug_resistance_level(
    app_state: AppState,
) -> Result<crate::manager::network::DebugResistanceLevel, ApiError> {
    let manager = app_state.network_manager.read().await;
    Ok(manager.get_debug_resistance_level())
}

pub(super) async fn set_debug_resistance_level(
    app_state: AppState,
    level: crate::manager::network::DebugResistanceLevel,
) -> Result<(), ApiError> {
    let mut manager = app_state.network_manager.write().await;
    manager.set_debug_resistance_level(level);
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) async fn hide_files() -> Result<(), ApiError> {
    use crate::platform::set_file_hide;

    let files_to_hide = get_important_files();

    for file_path in files_to_hide {
        if file_path.exists() && !set_file_hide(&file_path) {
            tracing::warn!("Failed to hide file: {:?}", file_path);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) async fn show_files() -> Result<(), ApiError> {
    use crate::platform::restore_file_hide;

    let files_to_show = get_important_files();

    for file_path in files_to_show {
        if file_path.exists() && !restore_file_hide(&file_path) {
            tracing::warn!("Failed to show file: {:?}", file_path);
        }
    }
    Ok(())
}

// ── update-check-interval controls ──

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckIntervalResponse {
    pub interval_seconds: u64,
}

pub(super) async fn get_update_check_interval(
    app_state: AppState,
) -> Result<UpdateCheckIntervalResponse, ApiError> {
    let interval = *app_state.update_check_interval.read().await;
    Ok(UpdateCheckIntervalResponse {
        interval_seconds: interval.as_secs(),
    })
}

pub(super) async fn set_update_check_interval(
    app_state: AppState,
    seconds: u64,
) -> Result<(), ApiError> {
    if seconds == 0 {
        return Err(ApiError::BadRequest(
            "Interval must be greater than 0 seconds".to_string(),
        ));
    }
    let mut interval = app_state.update_check_interval.write().await;
    *interval = Duration::from_secs(seconds);
    tracing::info!(
        "Background update check interval set to {} seconds",
        seconds
    );
    Ok(())
}

#[cfg(debug_assertions)]
pub(super) async fn reset_tos(app_state: AppState) -> Result<(), ApiError> {
    app_state
        .account_manager
        .reset_tos()
        .await
        .map_err(ApiError::Internal)?;
    Ok(())
}
