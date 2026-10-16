use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::time::Duration;

use crate::service::api::{AppState, error::ApiError};

pub fn router() -> Router<AppState> {
    let mut router = Router::new()
        .route("/dev/pro-policy", get(get_debug_pro_policy))
        .route("/dev/pro-policy/{policy}", post(set_debug_pro_policy))
        .route("/dev/resistance-level", get(get_debug_resistance_level))
        .route(
            "/dev/resistance-level/{level}",
            post(set_debug_resistance_level),
        )
        .route("/dev/update-check-interval", get(get_update_check_interval))
        .route(
            "/dev/update-check-interval/{seconds}",
            post(set_update_check_interval),
        );

    // File-hiding controls use Windows-only handlers (HIDDEN+SYSTEM attributes);
    // the routes exist only where the handlers do.
    #[cfg(target_os = "windows")]
    {
        router = router
            .route("/dev/protection/hide-files", post(hide_files))
            .route("/dev/protection/show-files", post(show_files));
    }

    #[cfg(debug_assertions)]
    {
        router = router.route("/dev/reset-tos", post(reset_tos));
    }

    router
}

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

#[utoipa::path(
    get,
    path = "/dev/pro-policy",
    tag = "Dev",
    responses(
        (status = 200, description = "Current debug licence policy", body = crate::manager::network::DebugProPolicy)
    )
)]
async fn get_debug_pro_policy(
    State(app_state): State<AppState>,
) -> Result<Json<crate::manager::network::DebugProPolicy>, ApiError> {
    let manager = app_state.network_manager.read().await;
    Ok(Json(manager.get_debug_pro_policy()))
}

#[utoipa::path(
    post,
    path = "/dev/pro-policy/{policy}",
    tag = "Dev",
    params(
        ("policy" = crate::manager::network::DebugProPolicy, Path, description = "The debug licence policy to set")
    ),
    responses(
        (status = 200, description = "Debug licence policy set successfully")
    )
)]
async fn set_debug_pro_policy(
    State(app_state): State<AppState>,
    axum::extract::Path(policy): axum::extract::Path<crate::manager::network::DebugProPolicy>,
) -> Result<Json<()>, ApiError> {
    let mut manager = app_state.network_manager.write().await;
    manager.set_debug_pro_policy(policy);
    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/dev/resistance-level",
    tag = "Dev",
    responses(
        (status = 200, description = "Current debug resistance level", body = crate::manager::network::DebugResistanceLevel)
    )
)]
async fn get_debug_resistance_level(
    State(app_state): State<AppState>,
) -> Result<Json<crate::manager::network::DebugResistanceLevel>, ApiError> {
    let manager = app_state.network_manager.read().await;
    Ok(Json(manager.get_debug_resistance_level()))
}

#[utoipa::path(
    post,
    path = "/dev/resistance-level/{level}",
    tag = "Dev",
    params(
        ("level" = crate::manager::network::DebugResistanceLevel, Path, description = "The debug resistance level to set")
    ),
    responses(
        (status = 200, description = "Debug resistance level set successfully")
    )
)]
async fn set_debug_resistance_level(
    State(app_state): State<AppState>,
    axum::extract::Path(level): axum::extract::Path<crate::manager::network::DebugResistanceLevel>,
) -> Result<Json<()>, ApiError> {
    let mut manager = app_state.network_manager.write().await;
    manager.set_debug_resistance_level(level);
    Ok(Json(()))
}

#[cfg(target_os = "windows")]
#[utoipa::path(
    post,
    path = "/dev/protection/hide-files",
    tag = "Dev",
    responses(
        (status = 200, description = "Files hidden successfully")
    )
)]
async fn hide_files() -> Result<Json<()>, ApiError> {
    use crate::platform::set_file_hide;

    let files_to_hide = get_important_files();

    for file_path in files_to_hide {
        if file_path.exists() && !set_file_hide(&file_path) {
            tracing::warn!("Failed to hide file: {:?}", file_path);
        }
    }
    Ok(Json(()))
}

#[cfg(target_os = "windows")]
#[utoipa::path(
    post,
    path = "/dev/protection/show-files",
    tag = "Dev",
    responses(
        (status = 200, description = "Files restored successfully")
    )
)]
async fn show_files() -> Result<Json<()>, ApiError> {
    use crate::platform::restore_file_hide;

    let files_to_show = get_important_files();

    for file_path in files_to_show {
        if file_path.exists() && !restore_file_hide(&file_path) {
            tracing::warn!("Failed to show file: {:?}", file_path);
        }
    }
    Ok(Json(()))
}

// ── update-check-interval controls ──

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckIntervalResponse {
    pub interval_seconds: u64,
}

#[utoipa::path(
    get,
    path = "/dev/update-check-interval",
    tag = "Dev",
    responses(
        (status = 200, description = "Current background update-check interval in seconds", body = UpdateCheckIntervalResponse)
    )
)]
async fn get_update_check_interval(
    State(app_state): State<AppState>,
) -> Result<Json<UpdateCheckIntervalResponse>, ApiError> {
    let interval = *app_state.update_check_interval.read().await;
    Ok(Json(UpdateCheckIntervalResponse {
        interval_seconds: interval.as_secs(),
    }))
}

#[utoipa::path(
    post,
    path = "/dev/update-check-interval/{seconds}",
    tag = "Dev",
    params(
        ("seconds" = u64, Path, description = "New interval in seconds for the background update check")
    ),
    responses(
        (status = 200, description = "Interval updated successfully")
    )
)]
async fn set_update_check_interval(
    State(app_state): State<AppState>,
    axum::extract::Path(seconds): axum::extract::Path<u64>,
) -> Result<Json<()>, ApiError> {
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
    Ok(Json(()))
}

#[cfg(debug_assertions)]
#[utoipa::path(
    		post,
    		path = "/dev/reset-tos",
    		tag = "Dev",
    		responses(
    			(status = 200, description = "TOS and privacy policy acceptance has been reset")
    		)
    	)]
async fn reset_tos(State(app_state): State<AppState>) -> Result<Json<()>, ApiError> {
    app_state
        .account_manager
        .reset_tos()
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(()))
}
