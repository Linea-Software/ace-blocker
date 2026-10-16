use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use std::collections::HashMap;

use crate::core::ApiKey;

use crate::service::api::{
    AppState,
    error::ApiError,
    schemas::{AccountInfoResponse, SetApiKeyRequest},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/account/info", get(get_account_info))
        .route("/account/refresh", post(refresh_account))
        .route("/account/api-key", post(set_api_key))
        .route("/account/logout", post(logout))
        .route("/account/accept-tos", post(accept_tos))
}

#[utoipa::path(
	get,
	path = "/account/info",
	tag = "Account",
	responses(
		(status = 200, description = "Get current account information", body = AccountInfoResponse)
	)
)]
pub(super) async fn get_account_info(State(state): State<AppState>) -> Json<AccountInfoResponse> {
    let manager = state.network_manager.read().await;
    let is_pro = manager.is_pro_user().await;
    let tos_accepted = state.account_manager.has_accepted_tos().await;

    Json(AccountInfoResponse {
        is_pro,
        tos_accepted,
    })
}

#[utoipa::path(
	post,
	path = "/account/refresh",
	tag = "Account",
	responses(
		(status = 200, description = "Re-read account status; remote re-verification runs only when the last successful check is older than one day", body = AccountInfoResponse)
	)
)]
/// Re-read the account's Pro status after an event that may have changed it
/// (login, logout, subscription change, focus regain).
///
/// The engine never calls the Ace server when the last successful
/// verification is less than one day old: this endpoint returns the cached
/// status in that case and performs exactly one single-flight remote
/// re-verification only when the cache is stale or absent. Transient errors
/// keep the previous entitlement; authorization failures clear the API key
/// and transition the account to free.
pub(super) async fn refresh_account(State(state): State<AppState>) -> Json<AccountInfoResponse> {
    let manager = state.network_manager.read().await;
    let is_pro = manager.is_pro_user().await;
    let tos_accepted = state.account_manager.has_accepted_tos().await;

    Json(AccountInfoResponse {
        is_pro,
        tos_accepted,
    })
}

#[utoipa::path(
	post,
	path = "/account/accept-tos",
	tag = "Account",
	responses(
		(status = 200, description = "Terms of Service accepted")
	)
)]
pub(super) async fn accept_tos(State(state): State<AppState>) -> Result<(), ApiError> {
    state
        .account_manager
        .accept_tos()
        .await
        .map_err(ApiError::Internal)?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/account/api-key",
    tag = "Account",
    request_body = SetApiKeyRequest,
    responses(
        (status = 200, description = "Set API key and fetch account details")
    )
)]
pub(super) async fn set_api_key(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    body: Option<Json<SetApiKeyRequest>>,
) -> Result<(), ApiError> {
    // Prefer the JSON body (website + protocol handler deliver the key there
    // so it never appears in query strings); fall back to the legacy query
    // parameter for older clients.
    let raw_api_key = body
        .map(|Json(payload)| payload.api_key)
        .or_else(|| query.get("apiKey").cloned())
        .ok_or_else(|| ApiError::BadRequest("Missing apiKey".to_string()))?;
    let api_key = ApiKey::try_new(raw_api_key)
        .map_err(|_| ApiError::BadRequest("apiKey must not be empty".to_string()))?;

    // Save the new key and clear the previous verification timestamp so the
    // entitlement check below validates this credential immediately.
    state
        .account_manager
        .update_account(Some(api_key), None)
        .await
        .map_err(ApiError::Internal)?;

    // Trigger pro check
    state.account_manager.is_pro().await;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/account/logout",
    tag = "Account",
    responses(
        (status = 200, description = "Logout and clear the local API key")
    )
)]
pub(super) async fn logout(State(state): State<AppState>) -> Result<(), ApiError> {
    // The Ace server revoke contract accepts a Better Auth API-key record ID,
    // not the credential value stored by the engine. The linking contract does
    // not currently provide that ID, so logout must not send the credential as keyId.

    state
        .account_manager
        .update_account(None, None)
        .await
        .map_err(ApiError::Internal)?;

    Ok(())
}
