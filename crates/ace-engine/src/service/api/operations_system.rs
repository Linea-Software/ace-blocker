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
    let (mut content_service, gate) = state.content_update_service.lock().await.detached();
    let _single_flight = gate.lock().await;

    // Metadata and local DB failures remain errors; empty lists/versions are
    // reserved for genuinely empty results, never as error sentinels.
    let engine = BinaryUpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: content_service
            .get_latest_installer_version()
            .await
            .map_err(|error| {
                ApiError::Internal(format!("Installer update check failed: {error}"))
            })?,
        available: false,
    };

    let site_packs_to_update = content_service
        .get_site_packs_to_update()
        .await
        .map_err(|error| ApiError::Internal(format!("Site-pack update check failed: {error}")))?
        .into_iter()
        .map(|(id, latest_version)| {
            let current_version = content_service
                .db_get_site_pack_version(&id)
                .map_err(|error| {
                    ApiError::Internal(format!("Site-pack version lookup failed: {error}"))
                })?
                .unwrap_or_default(); // Absent pack has no current version.
            Ok(SitePackUpdateEntry {
                current_version,
                id: id.to_string(),
                name: id.to_string(),
                latest_version: latest_version.to_string(),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    let resistance_scripts_need_update = content_service
        .resistance_scripts_require_update()
        .await
        .map_err(|error| ApiError::Internal(format!("Resistance update check failed: {error}")))?;

    state
        .content_update_service
        .lock()
        .await
        .publish_metadata_from(&content_service);
    Ok(UpdateCheckResponse {
        engine,
        content: ContentUpdateInfo {
            site_packs_needing_update: site_packs_to_update,
            resistance_scripts_need_update,
        },
    })
}

pub(super) async fn trigger_content_update(state: AppState) -> Result<(), ApiError> {
    // Share the same single-flight path as the background task. No shared
    // metadata mutex is held during downloads, staging or commit.
    crate::service::content_updater::run_update_pass(
        &state.content_update_service,
        &state.network_manager,
        &state.account_manager,
    )
    .await
    .map_err(|error| ApiError::Internal(format!("Content update failed: {error}")))
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
