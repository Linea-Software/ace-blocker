use crate::core::{NetworkBlockId, SitePackId};

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn get_net_block_to_site_pack_connections(
    state: AppState,
    network_block_id: uuid::Uuid,
) -> Result<GetNetListToSitePacksResponse, ApiError> {
    let manager = state.network_manager.read().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let connections = manager.get_network_block_to_site_packs(&list_id)?;

    let dtos = connections
        .into_iter()
        .map(|c| NetworkBlockToSitePackConnection {
            network_block_id,
            site_pack_id: c.read().unwrap().get_id().to_string(),
        })
        .collect();

    Ok(GetNetListToSitePacksResponse { connections: dtos })
}

pub(super) async fn add_net_block_to_site_pack_connection(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: AddNetListToSitePackRequest,
) -> Result<AddNetListToSitePackResponse, ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let site_pack_id = SitePackId::try_new(payload.sites_list_id)
        .map_err(|error| ApiError::BadRequest(format!("Invalid site pack ID: {error}")))?;
    let site_pack_id_str = site_pack_id.to_string();
    manager.add_network_block_to_site_pack(&list_id, &site_pack_id)?;

    Ok(AddNetListToSitePackResponse {
        connection: NetworkBlockToSitePackConnection {
            network_block_id,
            site_pack_id: site_pack_id_str,
        },
    })
}

pub(super) async fn remove_net_block_to_site_pack_connection(
    state: AppState,
    (network_block_id, site_pack_id): (uuid::Uuid, String),
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let site_pack_id = SitePackId::try_new(site_pack_id)
        .map_err(|error| ApiError::BadRequest(format!("Invalid site pack ID: {error}")))?;
    manager.remove_network_block_to_site_pack(&list_id, &site_pack_id)?;
    Ok(())
}
