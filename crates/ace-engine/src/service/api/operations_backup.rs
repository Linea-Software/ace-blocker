use crate::core::SitePackId;
use crate::service::{
    api::{AppState, error::ApiError},
    clients::AceServerClient,
};

/// A restored block can reference a remote pack whose data is not present.
/// Recover it promptly without holding the policy lock or blocking the API.
/// The release-only guard matches the normal background update loop.
#[cfg(not(debug_assertions))]
fn schedule_restored_site_pack_recovery(state: &AppState) {
    let content_service = state.content_update_service.clone();
    let network_manager = state.network_manager.clone();
    let account_manager = state.account_manager.clone();
    tokio::spawn(async move {
        if let Err(error) = crate::service::content_updater::run_update_pass(
            &content_service,
            &network_manager,
            &account_manager,
        )
        .await
        {
            tracing::warn!(%error, "Content recovery after backup import failed");
        }
    });
}

pub(super) async fn export_backup(
    state: AppState,
) -> Result<crate::manager::network::backup::EngineBackup, ApiError> {
    let mut manager = state.policy_write().await?;
    let backup = manager.export_backup()?;
    Ok(backup)
}

/// The server never receives the account password or plaintext JSON.
fn cloud_backup_url() -> String {
    format!("{}/api/v1/backups", AceServerClient::base_url())
}

fn cloud_http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(35))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| ApiError::Internal("Could not initialize cloud backup transport".into()))
}

fn cloud_http_status(status: reqwest::StatusCode) -> ApiError {
    match status.as_u16() {
        401 => ApiError::Unauthorized("Please reconnect your Ace Blocker account".into()),
        403 => ApiError::Forbidden("Cloud backup requires an active Pro subscription".into()),
        404 => ApiError::NotFound("No cloud backup found".into()),
        413 => ApiError::BadRequest("Encrypted cloud backup is too large".into()),
        _ => ApiError::Internal(format!(
            "Cloud backup service returned HTTP {}",
            status.as_u16()
        )),
    }
}

/// Download only the authenticated encrypted backup envelope.
async fn download_cloud_backup(
    api_key: &crate::core::ApiKey,
) -> Result<Option<super::backup_encryption::EncryptedCloudBackup>, ApiError> {
    let response = cloud_http_client()?
        .get(cloud_backup_url())
        .header("x-api-key", api_key.as_ref())
        .send()
        .await
        .map_err(|_| ApiError::Internal("Could not connect to cloud backup service".into()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(cloud_http_status(response.status()));
    }
    if response
        .content_length()
        .is_some_and(|size| size > 2 * 1024 * 1024)
    {
        return Err(ApiError::BadRequest("Cloud backup is too large".into()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| ApiError::Internal("Could not download cloud backup".into()))?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(ApiError::BadRequest("Cloud backup is too large".into()));
    }
    let encrypted = serde_json::from_slice(&bytes)
        .map_err(|_| ApiError::BadRequest("Invalid encrypted cloud backup envelope".into()))?;
    Ok(Some(encrypted))
}

async fn upload_encrypted_backup(
    api_key: &crate::core::ApiKey,
    encrypted: &super::backup_encryption::EncryptedCloudBackup,
) -> Result<(), ApiError> {
    let response = cloud_http_client()?
        .post(cloud_backup_url())
        .header("x-api-key", api_key.as_ref())
        .json(encrypted)
        .send()
        .await
        .map_err(|_| ApiError::Internal("Could not connect to cloud backup service".into()))?;
    if !response.status().is_success() {
        return Err(cloud_http_status(response.status()));
    }
    Ok(())
}

pub(super) async fn export_backup_to_cloud(
    state: AppState,
    payload: super::backup_encryption::CloudBackupPasswordRequest,
) -> Result<(), ApiError> {
    if !state.is_pro_account_entitled().await {
        return Err(ApiError::Forbidden("Cloud backup is a Pro feature".into()));
    }
    let api_key = state
        .account_manager
        .get_api_key()
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::Unauthorized("Not logged in".into()))?;
    let password = zeroize::Zeroizing::new(payload.password);
    let backup = {
        let mut manager = state.policy_write().await?;
        manager.export_backup()?
    };
    // If there is a previous encrypted backup, its password must decrypt it
    // before we allow an overwrite. A mistyped password cannot silently
    // render a previously recoverable cloud backup inaccessible.
    if let Some(existing) = download_cloud_backup(&api_key).await? {
        super::backup_encryption::decrypt_backup(&existing, &password)?;
    }
    // Encrypt the *entire* EngineBackup, including timezone, enabled state,
    // pack icons and both Resistance script override lists.
    let encrypted = super::backup_encryption::encrypt_backup(&backup, &password)?;
    upload_encrypted_backup(&api_key, &encrypted).await
}

pub(super) async fn import_backup_from_cloud(
    state: AppState,
    payload: super::backup_encryption::CloudBackupPasswordRequest,
) -> Result<(), ApiError> {
    if !state.is_pro_account_entitled().await {
        return Err(ApiError::Forbidden("Cloud restore is a Pro feature".into()));
    }
    let api_key = state
        .account_manager
        .get_api_key()
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::Unauthorized("Not logged in".into()))?;
    let password = zeroize::Zeroizing::new(payload.password);
    let encrypted = download_cloud_backup(&api_key)
        .await?
        .ok_or_else(|| ApiError::NotFound("No cloud backup found".into()))?;
    let backup = super::backup_encryption::decrypt_backup(&encrypted, &password)?;

    let mut manager = state.policy_write().await?;
    manager.import_backup(backup).await?;
    drop(manager);

    #[cfg(not(debug_assertions))]
    schedule_restored_site_pack_recovery(&state);
    Ok(())
}

pub(super) async fn import_backup(
    state: AppState,
    payload: crate::manager::network::backup::EngineBackup,
) -> Result<(), ApiError> {
    // Local backups are untrusted: `remote_id` and `requires_pro` come from
    // user-controlled JSON, so neither field proves an official Free catalog
    // entry. Only the *already loaded, canonical* Free remote packs are safe
    // to restore without Pro. Unknown IDs and premium references are gated,
    // including references embedded in the restored network blocks.
    let needs_pro = {
        let manager = state.network_manager.read().await;
        let is_known_free_pack = |raw_id: &str| -> bool {
            SitePackId::try_new(raw_id.to_owned())
                .ok()
                .and_then(|id| manager.get_site_pack(&id).ok())
                .is_some_and(|pack| pack.is_remote() && !pack.requires_pro())
        };
        payload
            .custom_site_packs
            .iter()
            .any(|pack| !is_known_free_pack(&pack.id))
            || payload
                .network_blocks
                .iter()
                .flat_map(|block| &block.connected_site_packs)
                .any(|pack_id| !is_known_free_pack(pack_id))
    };
    if needs_pro && !state.is_pro_account_entitled().await {
        return Err(ApiError::Forbidden(
            "Pro required to restore or connect custom or premium site lists".into(),
        ));
    }
    let mut manager = state.policy_write().await?;
    manager.import_backup(payload).await?;
    drop(manager);

    #[cfg(not(debug_assertions))]
    schedule_restored_site_pack_recovery(&state);
    Ok(())
}
