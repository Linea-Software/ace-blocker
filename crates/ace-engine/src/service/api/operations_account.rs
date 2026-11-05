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
