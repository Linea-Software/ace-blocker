use crate::core::common::constants::ENGINE_VERSION;
use crate::service::api::{
    AppState,
    error::ApiError,
    schemas::{
        BinaryUpdateInfo, ContentUpdateInfo, SitePackUpdateEntry, UpdateCheckResponse, VersionInfo,
    },
};

pub(super) async fn ping() -> &'static str {
    "pong"
}

pub(super) async fn get_version() -> VersionInfo {
    VersionInfo {
        version: ENGINE_VERSION.to_string(),
        build: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_string(),
    }
}

pub(super) async fn open_pro_page() -> Result<&'static str, ApiError> {
    #[cfg(debug_assertions)]
    let url = "http://localhost:5173/get-pro?origin=aceblocker".to_string();
    #[cfg(not(debug_assertions))]
    let url = "https://aceblocker.com/get-pro?origin=aceblocker".to_string();

    webbrowser::open(&url).map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok("Pro page opened")
}

pub(super) async fn check_updates(state: AppState) -> Result<UpdateCheckResponse, ApiError> {
    let mut content_service = state.content_update_service.lock().await;

    // Engine binary info — Phase 4 not yet implemented, so always return not-available.
    let engine = BinaryUpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: content_service.get_latest_installer_version().await,
        available: false,
    };

    // Content update info.
    let site_packs_to_update: Vec<SitePackUpdateEntry> = content_service
        .get_site_packs_to_update()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(id, latest_version)| SitePackUpdateEntry {
            current_version: content_service
                .db_get_site_pack_version(&id)
                .unwrap_or_default(),
            id: id.to_string(),
            name: id.to_string(), // remote_id doubles as name for now
            latest_version: latest_version.to_string(),
        })
        .collect();

    let resistance_scripts_need_update = content_service.resistance_scripts_require_update().await;

    Ok(UpdateCheckResponse {
        engine,
        content: ContentUpdateInfo {
            site_packs_needing_update: site_packs_to_update,
            resistance_scripts_need_update,
        },
    })
}

pub(super) async fn trigger_content_update(state: AppState) -> Result<(), ApiError> {
    // Metadata fetch + staging (downloads/extraction) happen WITHOUT the
    // manager write lock; only the commit runs under it (SVC-02). Failures
    // are propagated: a partial failure must not report success (API-02).
    let mut content_service = state.content_update_service.lock().await;

    content_service
        .check_for_updates()
        .await
        .map_err(|error| ApiError::Internal(format!("Update check failed: {}", error)))?;

    let api_key = state
        .account_manager
        .get_api_key()
        .await
        .map_err(ApiError::Internal)?;
    let is_pro = state.account_manager.is_pro().await;
    let staged = content_service
        .stage_pending_updates(api_key.as_ref(), is_pro)
        .await
        .map_err(|error| ApiError::Internal(format!("Update staging failed: {}", error)))?;

    let mut manager = state.network_manager.write().await;
    manager
        .apply_staged_content_updates(staged)
        .await
        .map_err(|error| ApiError::Internal(format!("Content update failed: {}", error)))?;

    // Register any new remote site lists (metadata + DB only).
    if let Err(error) = content_service
        .fetch_and_register_remote_site_packs(&mut manager)
        .await
    {
        return Err(ApiError::Internal(format!(
            "Failed to register new site lists: {}",
            error
        )));
    }

    Ok(())
}

pub(super) async fn shutdown(state: AppState) -> Result<(), ApiError> {
    let manager = state.network_manager.read().await;

    let has_locked_blocks = manager.any_network_block_locked();
    let resistance_locked = manager.is_resistance_mode_locked();

    if has_locked_blocks || resistance_locked {
        let mut reasons = Vec::new();
        if has_locked_blocks {
            reasons.push("one or more network blocks are locked");
        }
        if resistance_locked {
            reasons.push("resistance mode is locked");
        }
        return Err(ApiError::Locked(format!(
            "Cannot shut down: {}",
            reasons.join(" and ")
        )));
    }

    drop(manager);

    // The engine remains authoritative for whether shutdown is allowed. Only
    // after both lock checks pass do we ask the production watchdog for a short
    // in-memory maintenance lease so it will not resurrect this intentional
    // stop. In service mode the acknowledgement is required: exiting without
    // it would race watchdog resurrection during update/uninstall.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    if crate::platform::in_service_mode()
        && let Err(error) = crate::watchdog::request_maintenance(
            crate::watchdog_protocol::MaintenanceReason::UnlockedShutdown,
            crate::watchdog_protocol::DEFAULT_MAINTENANCE_TTL,
        )
        .await
    {
        tracing::warn!("Watchdog maintenance acknowledgement failed before shutdown: {error:#}");
        return Err(ApiError::Internal(
            "Watchdog maintenance is unavailable; shutdown was not started".to_string(),
        ));
    }

    // Signal the main run() loop to begin graceful shutdown.
    let _ = state.shutdown_tx.send(true);

    Ok(())
}
