use crate::core::{NetworkBlockId, SitePackId};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/network-blocks/{network_block_id}/site-lists",
            get(get_net_block_to_site_pack_connections).post(add_net_block_to_site_pack_connection),
        )
        .route(
            "/network-blocks/{network_block_id}/site-lists/{site_pack_id}",
            delete(remove_net_block_to_site_pack_connection),
        )
}

#[utoipa::path(
    get,
    path = "/network-blocks/{network_block_id}/site-lists",
    tag = "Network block to site list",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    responses(
        (status = 200, description = "Get site lists connected to a network block", body = GetNetListToSitePacksResponse)
    )
)]
pub(super) async fn get_net_block_to_site_pack_connections(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
) -> Result<Json<GetNetListToSitePacksResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let connections = manager.get_network_block_to_site_packs(&list_id)?;

    let dtos = connections
        .into_iter()
        .map(|c| NetworkBlockToSitePackConnection {
            network_block_id,
            site_pack_id: c.read().unwrap().get_id().to_string(),
        })
        .collect();

    Ok(Json(GetNetListToSitePacksResponse { connections: dtos }))
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}/site-lists",
    tag = "Network block to sites list",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    request_body = AddNetListToSitePackRequest,
    responses(
        (status = 200, description = "Add a connection between a network block and a sites list", body = AddNetListToSitePackResponse)
    )
)]
pub(super) async fn add_net_block_to_site_pack_connection(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<AddNetListToSitePackRequest>,
) -> Result<Json<AddNetListToSitePackResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let site_pack_id = SitePackId::from(payload.sites_list_id);
    let site_pack_id_str = site_pack_id.to_string();
    manager.add_network_block_to_site_pack(&list_id, &site_pack_id)?;

    Ok(Json(AddNetListToSitePackResponse {
        connection: NetworkBlockToSitePackConnection {
            network_block_id,
            site_pack_id: site_pack_id_str,
        },
    }))
}

#[utoipa::path(
    delete,
    path = "/network-blocks/{network_block_id}/site-lists/{site_pack_id}",
    tag = "Network block to sites list",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID"),
        ("site_pack_id" = String, Path, description = "Sites List ID")
    ),
    responses(
        (status = 200, description = "Remove the connection between a network block and a sites list")
    )
)]
pub(super) async fn remove_net_block_to_site_pack_connection(
    State(state): State<AppState>,
    Path((network_block_id, site_pack_id)): Path<(uuid::Uuid, String)>,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let site_pack_id = SitePackId::from(site_pack_id);
    manager.remove_network_block_to_site_pack(&list_id, &site_pack_id)?;
    Ok(())
}
