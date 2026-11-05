use crate::core::ApiKey;

use crate::service::api::{
    AppState,
    error::ApiError,
    schemas::{AccountInfoResponse, SetApiKeyRequest},
};

pub(super) async fn get_account_info(state: AppState) -> AccountInfoResponse {
    // This first paint also gates legal acceptance. Use the local persisted
    // snapshot: a slow/offline Pro verification must not indefinitely block
    // the rest of the desktop UI behind the legal-loading dialog.
    let is_pro = state.cached_pro_account_entitled().await;
    let tos_accepted = state.account_manager.has_accepted_tos().await;
    AccountInfoResponse {
        is_pro,
        tos_accepted,
    }
}

/// Re-read the account's Pro status after an event that may have changed it
/// (login, logout, subscription change, focus regain).
///
/// Explicit requests use a 30-second freshness window so returning from
/// checkout can detect a Pro upgrade without waiting for the proxy's 24-hour
/// lazy cache. Concurrent refreshes are coalesced by the AccountManager.
pub(super) async fn refresh_account(state: AppState) -> AccountInfoResponse {
    let is_pro = state.refresh_pro_account_entitled().await;
    let tos_accepted = state.account_manager.has_accepted_tos().await;
    AccountInfoResponse {
        is_pro,
        tos_accepted,
    }
}

pub(super) async fn accept_tos(state: AppState) -> Result<(), ApiError> {
    state
        .account_manager
        .accept_tos()
        .await
        .map_err(ApiError::Internal)?;
    Ok(())
}

pub(super) async fn set_api_key(
    state: AppState,
    payload: SetApiKeyRequest,
) -> Result<(), ApiError> {
    // Credentials arrive only as a typed IPC payload; never through a URL.
    let api_key = ApiKey::try_new(payload.api_key)
        .map_err(|_| ApiError::BadRequest("apiKey must not be empty".to_string()))?;

    // Validate against the site's key-only entitlement endpoint and verify
    // the signed lease before replacing any saved account. An HTTP 401 must
    // reach the protocol receiver as a failure, never "callback accepted".
    state
        .account_manager
        .import_api_key(api_key)
        .await
        .map_err(|error| {
            if error.contains("HTTP 401") || error.contains("HTTP 403") {
                ApiError::Unauthorized("The account key was rejected. Sign in again.".to_owned())
            } else {
                ApiError::ProStatusCheckFailed(error)
            }
        })?;

    Ok(())
}

pub(super) async fn logout(state: AppState) -> Result<(), ApiError> {
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
