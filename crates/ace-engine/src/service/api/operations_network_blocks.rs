use chrono::Local;

use crate::core::NetworkBlockId;
use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn rename_network_block(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: RenameNetworkBlockRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.rename_network_block(
        &NetworkBlockId::from(network_block_id),
        payload.new_name.clone(),
    )?;
    Ok(())
}

pub(super) async fn get_all_network_blocks(state: AppState) -> GetAllNetworkBlocksResponse {
    let manager = state.network_manager.read().await;
    // Use models to get names
    let now = Local::now();

    let network_blocks = manager
        .get_all_network_blocks()
        .into_iter()
        .map(|net_block| NetworkBlockSchema::from((net_block, &now)))
        .collect();

    GetAllNetworkBlocksResponse { network_blocks }
}

pub(super) async fn create_network_block(
    state: AppState,
    payload: CreateNetworkBlockRequest,
) -> Result<CreateNetworkBlockResponse, ApiError> {
    let mut manager = state.network_manager.write().await;

    let block_id = manager.create_network_block(payload.block_name.clone())?;

    // Set icon if provided during creation
    if let Some(ref icon) = payload.icon {
        manager.set_network_block_icon(&block_id, Some(icon.clone()))?;
    }

    Ok(CreateNetworkBlockResponse {
        network_block: NetworkBlockSchema {
            id: *block_id,
            name: payload.block_name,
            enabled: true,
            schedule_mode: crate::core::ScheduleMode::Always,
            is_active: false,
            is_locked: false,
            lock_state: crate::core::LockState::default().into(),
            safe_search: false,
            icon: payload.icon.clone(),
        },
    })
}

pub(super) async fn delete_network_block(
    state: AppState,
    network_block_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_network_block(&NetworkBlockId::from(network_block_id))?;
    Ok(())
}

pub(super) async fn set_network_block_safe_search(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: SetEnabledRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager
        .set_network_block_safe_search(&NetworkBlockId::from(network_block_id), payload.enabled)?;
    Ok(())
}

pub(super) async fn set_network_block_enabled(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: SetEnabledRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.set_network_block_enabled(&NetworkBlockId::from(network_block_id), payload.enabled)?;
    Ok(())
}

pub(super) async fn set_network_block_icon(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: SetIconRequest,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.set_network_block_icon(&NetworkBlockId::from(network_block_id), payload.icon)?;
    Ok(())
}

/// Commit a complete Network Block editor draft as one database transaction.
pub(super) async fn save_network_block(
    state: AppState,
    payload: SaveNetworkBlockRequest,
) -> Result<SaveNetworkBlockResponse, ApiError> {
    use crate::core::{ScheduleEntry, SitePackId};
    use crate::manager::network::save_network_block::NetworkBlockDraft;
    use chrono::Weekday;

    if payload.schedule_mode == crate::core::ScheduleMode::Scheduled
        && payload.schedule_entries.is_empty()
    {
        return Err(ApiError::BadRequest(
            "Scheduled blocks require a time period".into(),
        ));
    }

    // Entitlement may perform remote I/O: never hold the manager write lock
    // while checking it.
    let entitled = state.is_pro_account_entitled().await;
    let mut schedule = Vec::with_capacity(payload.schedule_entries.len());
    for entry in payload.schedule_entries {
        let mut days = 0;
        if entry.monday {
            days |= 1 << Weekday::Mon as u8;
        }
        if entry.tuesday {
            days |= 1 << Weekday::Tue as u8;
        }
        if entry.wednesday {
            days |= 1 << Weekday::Wed as u8;
        }
        if entry.thursday {
            days |= 1 << Weekday::Thu as u8;
        }
        if entry.friday {
            days |= 1 << Weekday::Fri as u8;
        }
        if entry.saturday {
            days |= 1 << Weekday::Sat as u8;
        }
        if entry.sunday {
            days |= 1 << Weekday::Sun as u8;
        }
        let proposed = ScheduleEntry::from_raw(
            uuid::Uuid::now_v7().to_string(),
            payload.id.unwrap_or(uuid::Uuid::nil()).to_string(),
            &entry.start_time.format("%H:%M:%S").to_string(),
            &entry.end_time.format("%H:%M:%S").to_string(),
            days,
        )
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
        schedule.push(proposed);
    }
    let site_pack_ids = payload
        .site_pack_ids
        .into_iter()
        .map(|value| {
            SitePackId::try_new(value)
                .map_err(|error| ApiError::BadRequest(format!("Invalid site pack ID: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut manager = state.network_manager.write().await;
    let id = manager.save_network_block(
        NetworkBlockDraft {
            id: payload.id.map(NetworkBlockId::from),
            name: payload.name,
            icon: payload.icon,
            safe_search: payload.safe_search,
            rules: payload.rules,
            schedule,
            schedule_mode: payload.schedule_mode,
            site_pack_ids,
        },
        entitled,
    )?;
    let block = manager
        .get_network_block_from(&id)
        .expect("saved block must be published after transaction");
    Ok(SaveNetworkBlockResponse {
        network_block: NetworkBlockSchema::from((block, &Local::now())),
    })
}
