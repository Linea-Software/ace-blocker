use futures_util::Stream;

use crate::core::importer::{SitePackSource, SitePackSourceId};
use crate::core::{SitePackId, network_block::site_pack::SitePackCreationStatus};
use crate::service::api::{AppState, error::ApiError, schemas};

pub(super) async fn get_all_site_packs(
    state: AppState,
) -> Result<schemas::GetAllSitePacksResponse, ApiError> {
    let manager = state.network_manager.read().await;

    // Drop the read guards first: enriching each schema needs a second,
    // immutable manager call (`is_site_pack_locked`), which cannot run while
    // the guards from `get_all_site_packs` are still alive.
    let packs: Vec<(schemas::SitePackSchema, crate::core::SitePackId)> = manager
        .get_all_site_packs()?
        .into_iter()
        .map(|site_pack| {
            let schema = schemas::SitePackSchema::from(&*site_pack);
            let site_pack_id = site_pack.get_id().clone();
            (schema, site_pack_id)
        })
        .collect();

    let site_packs_data = packs
        .into_iter()
        .map(|(mut schema, site_pack_id)| {
            // Enrich with the canonical lock state so the app can disable
            // delete/resync controls without reconstructing block refs.
            schema.is_locked = manager.is_site_pack_locked(&site_pack_id);
            schema
        })
        .collect();

    Ok(schemas::GetAllSitePacksResponse {
        site_packs: site_packs_data,
    })
}

pub(super) async fn create_site_pack(
    state: AppState,
    payload: schemas::CreateSitePackRequest,
) -> Result<impl Stream<Item = SitePackCreationStatus>, ApiError> {
    if payload.sources.is_empty() {
        return Err(ApiError::BadRequest("No sources provided".to_string()));
    }

    let mut core_sources = Vec::new();
    for source in payload.sources {
        core_sources.push(SitePackSource {
            id: SitePackSourceId::from(uuid::Uuid::now_v7()),
            site_pack_id: SitePackId::from(uuid::Uuid::nil().to_string()),
            source: source.source,
            source_type: source.source_type,
            format: source.format,
        });
    }

    if !state.is_pro_account_entitled().await {
        return Err(
            crate::manager::network::error::NetworkManagerOpErr::InternalError {
                message: "Failed to add site list".to_string(),
            }
            .into(),
        );
    }

    let stream = {
        let mut manager = state.network_manager.write().await;
        manager.import_site_pack_for_verified_pro(
            &payload.name,
            payload.description,
            payload.icon,
            core_sources,
        )?
    };

    Ok(stream)
}

pub(super) async fn delete_site_pack(
    state: AppState,
    site_pack_id: String,
) -> Result<(), ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    let mut manager = state.network_manager.write().await;

    // The backend enforces the lock rule — a stale frontend that believes a
    // pack is deletable must still be refused when the pack is locked (423
    // "locked"), not silently succeed or get a generic error.
    if manager.is_site_pack_locked(&site_pack_id) {
        return Err(ApiError::Locked(format!(
            "Cannot delete site list {} while it is referenced by a locked network block",
            site_pack_id
        )));
    }

    // Prevent deletion if the site list is attached to any network block: the
    // user must disconnect it from each block first (the disconnect operation
    // is the one gated by that block's lock).
    if manager.is_site_pack_being_used(&site_pack_id) {
        return Err(ApiError::Conflict(
            "Cannot delete a site list that is currently in use by a network block".to_string(),
        ));
    }

    // Check if this is a remote site list
    let is_remote = {
        let site_pack = manager.get_site_pack(&site_pack_id);
        site_pack.map(|s| s.is_remote()).unwrap_or(false)
    };

    if is_remote {
        // For remote site lists, mark as ignored instead of deleting
        manager.mark_remote_site_pack_as_ignored(&site_pack_id)?;
    } else {
        // For local site lists, delete normally
        manager.delete_site_pack(&site_pack_id)?;
    }

    Ok(())
}

pub(super) async fn resync_site_pack(
    state: AppState,
    site_pack_id: String,
) -> Result<impl Stream<Item = SitePackCreationStatus>, ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    let stream = {
        let mut manager = state.network_manager.write().await;
        manager.resync_site_pack(&site_pack_id).await?
    };
    Ok(stream)
}

pub(super) async fn get_site_pack_sources(
    state: AppState,
    site_pack_id: String,
) -> Result<schemas::GetSitePackSourcesResponse, ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    let manager = state.network_manager.read().await;
    let sources = manager.get_site_pack_sources(&site_pack_id)?;

    let dtos = sources
        .into_iter()
        .map(|s| schemas::SitePackSourceSchema::from(&s))
        .collect();

    Ok(schemas::GetSitePackSourcesResponse { sources: dtos })
}

pub(super) async fn add_site_pack_source(
    state: AppState,
    site_pack_id: String,
    payload: schemas::AddSitePackSourceRequest,
) -> Result<schemas::AddSitePackSourceResponse, ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    if !state.is_pro_account_entitled().await {
        return Err(ApiError::Forbidden("Pro subscription required".to_string()));
    }
    let mut manager = state.network_manager.write().await;

    let source = manager.add_site_pack_source(
        &site_pack_id,
        &payload.source,
        payload.source_type,
        payload.format,
    )?;

    Ok(schemas::AddSitePackSourceResponse {
        source: schemas::SitePackSourceSchema::from(&source),
    })
}

pub(super) async fn delete_site_pack_source(
    state: AppState,
    source_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_site_pack_source(source_id)?;
    Ok(())
}

pub(super) async fn re_enable_remote_site_packs(state: AppState) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.re_enable_all_remote_site_packs().await?;
    Ok(())
}
