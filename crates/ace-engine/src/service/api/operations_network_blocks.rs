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
