use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::Local;

use crate::core::NetworkBlockId;
use crate::service::api::{AppState, error::ApiError, schemas::*};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/network-blocks",
            get(get_all_network_blocks).post(create_network_block),
        )
        .route(
            "/network-blocks/{network_block_id}",
            delete(delete_network_block).post(rename_network_block),
        )
        .route(
            "/network-blocks/{network_block_id}/safe-search",
            post(set_network_block_safe_search),
        )
        .route(
            "/network-blocks/{network_block_id}/enabled",
            post(set_network_block_enabled),
        )
        .route(
            "/network-blocks/{network_block_id}/icon",
            post(set_network_block_icon),
        )
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}",
    tag = "Network Blocks",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    request_body = RenameNetworkBlockRequest,
    responses(
        (status = 200, description = "Network block renamed successfully"),
        (status = 404, description = "Network block not found"),
        (status = 403, description = "Network block is locked")
    )
)]
pub(super) async fn rename_network_block(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<RenameNetworkBlockRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.rename_network_block(
        &NetworkBlockId::from(network_block_id),
        payload.new_name.clone(),
    )?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/network-blocks",
    tag = "Network Blocks",
    responses(
        (status = 200, description = "List of all network blocks", body = GetAllNetworkBlocksResponse)
    )
)]
pub(super) async fn get_all_network_blocks(
    State(state): State<AppState>,
) -> Json<GetAllNetworkBlocksResponse> {
    let manager = state.network_manager.read().await;
    // Use models to get names
    let now = Local::now();

    let network_blocks = manager
        .get_all_network_blocks()
        .into_iter()
        .map(|net_block| NetworkBlockSchema::from((net_block, &now)))
        .collect();

    Json(GetAllNetworkBlocksResponse { network_blocks })
}

#[utoipa::path(
    post,
    path = "/network-blocks",
    tag = "Network Blocks",
    request_body = CreateNetworkBlockRequest,
    responses(
        (status = 200, description = "Create a new network block", body = CreateNetworkBlockResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub(super) async fn create_network_block(
    State(state): State<AppState>,
    Json(payload): Json<CreateNetworkBlockRequest>,
) -> Result<Json<CreateNetworkBlockResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;

    let block_id = manager.create_network_block(payload.block_name.clone())?;

    // Set icon if provided during creation
    if let Some(ref icon) = payload.icon {
        manager.set_network_block_icon(&block_id, Some(icon.clone()))?;
    }

    Ok(Json(CreateNetworkBlockResponse {
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
    }))
}

#[utoipa::path(
    delete,
    path = "/network-blocks/{network_block_id}",
    tag = "Network Blocks",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    responses(
        (status = 200, description = "Delete a network block"),
        (status = 500, description = "Internal server error")
    )
)]
pub(super) async fn delete_network_block(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_network_block(&NetworkBlockId::from(network_block_id))?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}/safe-search",
    tag = "Network Blocks",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    request_body = SetEnabledRequest,
    responses(
        (status = 200, description = "Safe search setting updated successfully"),
        (status = 404, description = "Network block not found"),
        (status = 403, description = "Network block is locked")
    )
)]
pub(super) async fn set_network_block_safe_search(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<SetEnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    manager
        .set_network_block_safe_search(&NetworkBlockId::from(network_block_id), payload.enabled)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}/enabled",
    tag = "Network Blocks",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    request_body = SetEnabledRequest,
    responses(
        (status = 200, description = "Network block enabled state updated successfully"),
        (status = 404, description = "Network block not found"),
        (status = 403, description = "Network block is locked and cannot be disabled")
    )
)]
pub(super) async fn set_network_block_enabled(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<SetEnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.set_network_block_enabled(&NetworkBlockId::from(network_block_id), payload.enabled)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    		post,
    		path = "/network-blocks/{network_block_id}/icon",
    		tag = "Network Blocks",
    		params(
    			("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    		),
    		request_body = SetIconRequest,
    		responses(
    			(status = 200, description = "Icon updated successfully"),
    			(status = 404, description = "Network block not found")
    		)
    	)]
pub(super) async fn set_network_block_icon(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<SetIconRequest>,
) -> Result<StatusCode, ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.set_network_block_icon(&NetworkBlockId::from(network_block_id), payload.icon)?;
    Ok(StatusCode::OK)
}
