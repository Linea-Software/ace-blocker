use crate::core::{Lock, NetworkBlockId};

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn get_lock(
    state: AppState,
    network_block_id: uuid::Uuid,
) -> Result<GetNetworkLockResponse, ApiError> {
    let manager = state.network_manager.read().await;

    let network_block = manager
        .get_network_block_from(&NetworkBlockId::from(network_block_id))
        .ok_or("Internal server error")?;
    let lock = network_block.get_lock();

    Ok(GetNetworkLockResponse {
        lock: NetworkListLockSchema::from_lock(
            lock,
            network_block_id,
            network_block.is_locked(&crate::trusted_time::locking_now()),
        ),
    })
}

pub(super) async fn set_lock(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: SetNetworkLockRequest,
) -> Result<SetNetworkLockResponse, ApiError> {
    let mut manager = state.network_manager.write().await;

    let lock = Lock::from_user_raw(payload.lock_state.to_string(), payload.lock_end.clone())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    manager.set_lock(&NetworkBlockId::from(network_block_id), lock)?;

    let network_block = manager
        .get_network_block_from(&NetworkBlockId::from(network_block_id))
        .ok_or("Internal server error")?;

    Ok(SetNetworkLockResponse {
        lock: NetworkListLockSchema {
            network_block_id,
            is_active: network_block.is_locked(&crate::trusted_time::locking_now()),
            lock_state: network_block.get_lock().get_state().into(),
            lock_end: payload.lock_end,
        },
    })
}
