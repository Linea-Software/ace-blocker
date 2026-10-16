use crate::core::NetworkBlockId;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};

use crate::service::api::{AppState, error::ApiError, schemas::*};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/network-blocks/{network_block_id}/user-rules",
            get(get_user_rules).post(add_user_rules),
        )
        .route("/user-rules/{user_rule_id}", delete(delete_user_rule))
        .route(
            "/resistance-mode/type",
            get(get_resistance_mode_type).post(set_resistance_mode_type),
        )
}

#[utoipa::path(
    get,
    path = "/network-blocks/{network_block_id}/user-rules",
    tag = "User Rules",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    responses(
        (status = 200, description = "Get user defined rules for a network list", body = GetUserDefinedRulesResponse)
    )
)]
pub(super) async fn get_user_rules(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
) -> Result<Json<GetUserDefinedRulesResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;

    let list_id = NetworkBlockId::from(network_block_id);
    let rules = manager.get_user_defined_entries_of(&list_id)?;

    let rules = rules
        .into_iter()
        .map(UserDefinedRuleSchema::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;

    Ok(Json(GetUserDefinedRulesResponse {
        user_defined_rules: rules,
    }))
}

#[utoipa::path(
    post,
    path = "/network-blocks/{network_block_id}/user-rules",
    tag = "User Rules",
    params(
        ("network_block_id" = uuid::Uuid, Path, description = "Network Block ID")
    ),
    request_body = AddUserDefinedRulesRequest,
    responses(
        (status = 200, description = "Add user defined rules to a network list", body = AddUserDefinedRulesResponse)
    )
)]
pub(super) async fn add_user_rules(
    State(state): State<AppState>,
    Path(network_block_id): Path<uuid::Uuid>,
    Json(payload): Json<AddUserDefinedRulesRequest>,
) -> Result<Json<AddUserDefinedRulesResponse>, ApiError> {
    let mut manager = state.network_manager.write().await;
    let list_id = NetworkBlockId::from(network_block_id);
    let models = manager.add_user_defined_entries(&list_id, payload.rules)?;

    let rules = models
        .into_iter()
        .map(UserDefinedRuleSchema::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;

    Ok(Json(AddUserDefinedRulesResponse {
        user_defined_rules: rules,
    }))
}

#[utoipa::path(
    delete,
    path = "/user-rules/{user_rule_id}",
    tag = "User Rules",
    params(
        ("user_rule_id" = uuid::Uuid, Path, description = "User Rule ID")
    ),
    responses(
        (status = 200, description = "Delete a user defined rule")
    )
)]
pub(super) async fn delete_user_rule(
    State(state): State<AppState>,
    Path(user_rule_id): Path<uuid::Uuid>,
) -> Result<(), ApiError> {
    let mut manager = state.network_manager.write().await;
    manager.delete_user_defined_entry(user_rule_id)?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/resistance-mode/type",
    tag = "User Rules",
    responses(
        (status = 200, description = "Get current resistance mode type", body = ResistanceModeTypeResponse)
    )
)]
pub(super) async fn get_resistance_mode_type(
    State(state): State<AppState>,
) -> Result<Json<ResistanceModeTypeResponse>, ApiError> {
    let manager = state.network_manager.read().await;

    let resistance_mode = manager.get_resistance_mode();

    Ok(Json(ResistanceModeTypeResponse {
        mode: resistance_mode.get_mode(),
        current_intensity: resistance_mode.get_intensity(),
    }))
}

#[utoipa::path(
    post,
    path = "/resistance-mode/type",
    tag = "User Rules",
    request_body = PunishmentMode,
    responses(
        (status = 200, description = "Set resistance mode type", body = ResistanceModeTypeResponse),
        (status = 400, description = "Invalid mode type or unsupported mode (dynamic is not available in v1 beta)"),
        (status = 403, description = "Resistance mode is locked")
    )
)]
pub(super) async fn set_resistance_mode_type(
    State(state): State<AppState>,
    mode: crate::service::api::error::ValidatedJson<
        crate::manager::resistance::punishment::PunishmentMode,
    >,
) -> Result<Json<ResistanceModeTypeResponse>, ApiError> {
    let mode = mode.0;

    let mut manager = state.network_manager.write().await;

    manager.set_resistance_mode_type(mode)?;

    let resistance_mode = manager.get_resistance_mode();
    Ok(Json(ResistanceModeTypeResponse {
        mode: resistance_mode.get_mode(),
        current_intensity: resistance_mode.get_intensity(),
    }))
}
