use axum::{
    Json, Router,
    extract::{Path, State},
    response::sse::{Event, Sse},
    routing::{delete, get, post},
};
use futures_util::{Stream, StreamExt};
use std::convert::Infallible;

use crate::core::importer::{SitePackSource, SitePackSourceId};
use crate::core::{SitePackId, network_block::site_pack::SitePackCreationStatus};
use crate::service::api::{AppState, error::ApiError, schemas};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/site-lists",
            get(get_all_site_packs).post(create_site_pack),
        )
        .route("/site-lists/{site_pack_id}", delete(delete_site_pack))
        .route("/site-lists/{site_pack_id}/resync", post(resync_site_pack))
        .route(
            "/site-lists/{site_pack_id}/sources",
            get(get_site_pack_sources).post(add_site_pack_source),
        )
        .route(
            "/site-lists/sources/{source_id}",
            delete(delete_site_pack_source),
        )
        .route(
            "/site-lists/remote/re-enable",
            post(re_enable_remote_site_packs),
        )
}

#[utoipa::path(
    get,
    path = "/site-lists",
    tag = "Site Lists",
    responses(
        (status = 200, description = "List of all site lists", body = schemas::GetAllSitePacksResponse)
    )
)]
pub(super) async fn get_all_site_packs(
    State(state): State<AppState>,
) -> Result<Json<schemas::GetAllSitePacksResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;

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

    Ok(Json(schemas::GetAllSitePacksResponse {
        site_packs: site_packs_data,
    }))
}

#[utoipa::path(
    post,
    path = "/site-lists",
    tag = "Site Lists",
    request_body = schemas::CreateSitePackRequest,
    responses(
        (status = 200, description = "Create a new site list, returns an SSE Stream", body = SitePackCreationStatus, content_type = "text/event-stream")
    )
)]
pub(super) async fn create_site_pack(
    State(state): State<AppState>,
    Json(payload): Json<schemas::CreateSitePackRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
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

    let stream = {
        let mut manager = state.network_manager.write().await;
        manager
            .import_site_pack(
                &payload.name,
                payload.description,
                payload.icon,
                core_sources,
            )
            .await?
    };

    let stream = stream.map(|status| {
        Ok(Event::default()
            .json_data(status)
            .unwrap_or_else(|_| Event::default()))
    });

    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()))
}

#[utoipa::path(
    delete,
    path = "/site-lists/{site_pack_id}",
    tag = "Site Lists",
    params(
        ("site_pack_id" = String, Path, description = "Site List ID")
    ),
    responses(
        (status = 200, description = "Delete a site list. For remote site lists, marks them as ignored instead of deleting."),
        (status = 409, description = "Site list is in use by a network block and cannot be deleted")
    )
)]
pub(super) async fn delete_site_pack(
    State(state): State<AppState>,
    Path(site_pack_id): Path<String>,
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

#[utoipa::path(
    post,
    path = "/site-lists/{site_pack_id}/resync",
    tag = "Site Lists",
    params(
        ("site_pack_id" = String, Path, description = "Site List ID")
    ),
    responses(
        (status = 200, description = "Resync a site list, returns an SSE Stream", body = SitePackCreationStatus, content_type = "text/event-stream")
    )
)]
pub(super) async fn resync_site_pack(
    State(state): State<AppState>,
    Path(site_pack_id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    let stream = {
        let mut manager = state.network_manager.write().await;
        manager.resync_site_pack(&site_pack_id).await?
    };
    let stream = stream.map(|status| {
        Ok(Event::default()
            .json_data(status)
            .unwrap_or_else(|_| Event::default()))
    });

    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()))
}

#[utoipa::path(
    get,
    path = "/site-lists/{site_pack_id}/sources",
    tag = "Site Lists",
    params(
        ("site_pack_id" = String, Path, description = "Site List ID")
    ),
    responses(
        (status = 200, description = "Get sources of a site list", body = schemas::GetSitePackSourcesResponse)
    )
)]
pub(super) async fn get_site_pack_sources(
    State(state): State<AppState>,
    Path(site_pack_id): Path<String>,
) -> Result<Json<schemas::GetSitePackSourcesResponse>, ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    let mut manager = state.network_manager.write().await;
    let sources = manager.get_site_pack_sources(&site_pack_id)?;

    let dtos = sources
        .into_iter()
        .map(|s| schemas::SitePackSourceSchema::from(&s))
        .collect();

    Ok(Json(schemas::GetSitePackSourcesResponse { sources: dtos }))
}

#[utoipa::path(
    post,
    path = "/site-lists/{site_pack_id}/sources",
    tag = "Site Lists",
    params(
        ("site_pack_id" = String, Path, description = "Site List ID")
    ),
    request_body = schemas::AddSitePackSourceRequest,
    responses(
        (status = 200, description = "Add a source to a site list", body = schemas::AddSitePackSourceResponse)
    )
)]
pub(super) async fn add_site_pack_source(
    State(state): State<AppState>,
    Path(site_pack_id): Path<String>,
    Json(payload): Json<schemas::AddSitePackSourceRequest>,
) -> Result<Json<schemas::AddSitePackSourceResponse>, ApiError> {
    let site_pack_id = SitePackId::from(site_pack_id);
    let mut manager = state.network_manager.write().await;

    if !manager.is_pro_user().await {
        return Err(ApiError::Forbidden("Pro subscription required".to_string()));
    }

    let source = manager.add_site_pack_source(
        &site_pack_id,
        &payload.source,
        payload.source_type,
        payload.format,
    )?;

    Ok(Json(schemas::AddSitePackSourceResponse {
        source: schemas::SitePackSourceSchema::from(&source),
    }))
}

#[utoipa::path(
    delete,
    path = "/site-lists/sources/{source_id}",
    tag = "Site Lists",
    params(
        ("source_id" = uuid::Uuid, Path, description = "Site List Source ID")
    ),
    responses(
        (status = 200, description = "Delete a site list source")
    )
)]
pub(super) async fn delete_site_pack_source(
    State(state): State<AppState>,
    Path(source_id): Path<uuid::Uuid>,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_site_pack_source(source_id)?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/site-lists/remote/re-enable",
    tag = "Site Lists",
    responses(
        (status = 200, description = "Re-enable all ignored remote site lists and re-fetch their metadata")
    )
)]
pub(super) async fn re_enable_remote_site_packs(
    State(state): State<AppState>,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.re_enable_all_remote_site_packs().await?;
    Ok(())
}
