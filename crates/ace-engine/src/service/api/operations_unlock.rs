use chrono::Local;

use crate::core::{
    LockState, NetworkBlockId,
    common::unlock_challenge::{UnlockChallenge, UnlockChallengeId},
};
use crate::service::api::{AppState, error::ApiError, schemas::*};

pub(super) async fn initiate_unlock(
    state: AppState,
    network_block_id: uuid::Uuid,
) -> Result<UnlockChallengeInitiateResponse, ApiError> {
    let net_list_id = NetworkBlockId::from(network_block_id);

    // Verify the block exists and has SoftUnlockable lock state.
    {
        let manager = state.network_manager.read().await;
        let network_block = manager
            .get_network_block_from(&net_list_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Network block {} not found", network_block_id))
            })?;

        if network_block.get_lock().get_state() != LockState::SoftUnlockable {
            return Err(ApiError::Forbidden(
                "This network block is not soft-unlockable. Only blocks with a SoftUnlockable lock can use the reflection unlock flow.".to_string(),
            ));
        }
    }

    // Check for existing challenge or create a new one.
    let mut challenges = state.unlock_challenges.lock().await;

    if let Some(existing) = challenges.get(&net_list_id) {
        // Return existing challenge state (idempotent).
        return Ok(UnlockChallengeInitiateResponse {
            challenge_id: existing.challenge_id.into_inner(),
            reflection_prompt: UnlockChallenge::REFLECTION_PROMPT.to_string(),
            min_words: UnlockChallenge::MIN_WORDS,
            cooldown_seconds: UnlockChallenge::COOLDOWN_SECONDS,
            initiated_at: existing
                .initiated_at
                .format(crate::core::constants::DATETIME_FORMAT)
                .to_string(),
        });
    }

    let challenge = UnlockChallenge::new();
    let response = UnlockChallengeInitiateResponse {
        challenge_id: challenge.challenge_id.into_inner(),
        reflection_prompt: UnlockChallenge::REFLECTION_PROMPT.to_string(),
        min_words: UnlockChallenge::MIN_WORDS,
        cooldown_seconds: UnlockChallenge::COOLDOWN_SECONDS,
        initiated_at: challenge
            .initiated_at
            .format(crate::core::constants::DATETIME_FORMAT)
            .to_string(),
    };

    challenges.insert(net_list_id, challenge);

    Ok(response)
}

pub(super) async fn confirm_unlock(
    state: AppState,
    network_block_id: uuid::Uuid,
    payload: UnlockChallengeConfirmRequest,
) -> Result<super::rpc::UnlockConfirmOutcome, ApiError> {
    let net_list_id = NetworkBlockId::from(network_block_id);

    // Look up the challenge.
    let challenges = state.unlock_challenges.lock().await;

    let challenge = challenges.get(&net_list_id).cloned().ok_or_else(|| {
        ApiError::NotFound(
            "No active unlock challenge for this network block. Call /unlock/initiate first."
                .to_string(),
        )
    })?;

    // Verify challenge ID matches.
    if challenge.challenge_id != UnlockChallengeId::from(payload.challenge_id) {
        return Err(ApiError::BadRequest(
            "Challenge ID does not match the active challenge for this block.".to_string(),
        ));
    }

    // Check server-side cooldown.
    if !challenge.is_cooldown_elapsed() {
        let remaining = challenge.cooldown_remaining_seconds();
        let body = TooEarlyResponse {
            error: format!(
                "Cooldown not elapsed. Please wait {} more seconds.",
                remaining
            ),
            retry_after_seconds: remaining,
        };
        return Ok(super::rpc::UnlockConfirmOutcome::TooEarly(body));
    }

    // All checks passed — unlock the block via the challenge-redemption path
    // (the ONLY path that may weaken a SoftUnlockable lock; see
    // `NetworkManager::redeem_soft_unlock`). The challenge is consumed only
    // AFTER the redemption succeeds, so a failure (error.g. the lock was
    // escalated while the challenge was pending) leaves the challenge intact.
    drop(challenges);

    let mut manager = state.network_manager.write().await;
    let now = Local::now();

    manager
        .redeem_soft_unlock(&net_list_id)
        .map_err(ApiError::from)?;

    let mut challenges = state.unlock_challenges.lock().await;
    challenges.remove(&net_list_id);
    drop(challenges);

    let network_block = manager
        .get_network_block_from(&net_list_id)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Network block {} not found after unlock",
                network_block_id
            ))
        })?;

    Ok(super::rpc::UnlockConfirmOutcome::Unlocked(
        UnlockChallengeConfirmResponse {
            network_block: NetworkBlockSchema::from((network_block, &now)),
        },
    ))
}
