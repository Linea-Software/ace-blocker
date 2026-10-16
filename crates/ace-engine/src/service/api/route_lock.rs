use crate::core::{Lock, NetworkBlockId};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/network-blocks/{network_block_id}/lock",
        get(get_lock).post(set_lock),
    )
}

#[utoipa::path(
    get,
    path = "/network-blocks/{network_block_id}/lock",
    tag = "Locks",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network List ID")
    ),
    responses(
        (status = 200, description = "Get lock status of a network list", body = GetNetworkLockResponse)
    )
)]
pub(super) async fn get_lock(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
) -> Result<Json<GetNetworkLockResponse>, ApiError> {
    let manager = state.network_manager.read().await;

    let network_block = manager
        .get_network_block_from(&NetworkBlockId::from(network_block_id))
        .ok_or("Internal server error")?;
    let lock = network_block.get_lock();

    Ok(Json(GetNetworkLockResponse {
        lock: NetworkListLockSchema::from_lock(
            lock,
            network_block_id,
            network_block.is_locked(&chrono::Local::now()),
        ),
    }))
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}/lock",
    tag = "Locks",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network block ID")
    ),
    request_body = SetNetworkLockRequest,
    responses(
        (status = 200, description = "Set lock status of a network list", body = SetNetworkLockResponse)
    )
)]
pub(super) async fn set_lock(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<SetNetworkLockRequest>,
) -> Result<Json<SetNetworkLockResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;

    let lock = Lock::from_raw(payload.lock_state.to_string(), payload.lock_end.clone())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    manager.set_lock(&NetworkBlockId::from(network_block_id), lock)?;

    let network_block = manager
        .get_network_block_from(&NetworkBlockId::from(network_block_id))
        .ok_or("Internal server error")?;

    Ok(Json(SetNetworkLockResponse {
        lock: NetworkListLockSchema {
            network_block_id,
            is_active: network_block.is_locked(&chrono::Local::now()),
            lock_state: network_block.get_lock().get_state().into(),
            lock_end: payload.lock_end,
        },
    }))
}
