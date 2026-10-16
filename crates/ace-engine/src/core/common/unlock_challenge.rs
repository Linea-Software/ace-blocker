use chrono::{DateTime, Local};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Display, Serialize, Deserialize))]
pub struct UnlockChallengeId(Uuid);

/// An active soft-unlock reflection challenge for a network block.
///
/// Created when the client calls `/network-blocks/{id}/unlock/initiate`
/// on a block whose lock is `SoftUnlockable`.
///
/// The unlock is granted only when the 5-minute cooldown since
/// `initiated_at` has elapsed and the challenge id is presented at confirm.
///
/// Challenges are stored in-memory only and do **not** survive engine restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockChallenge {
    /// Unique identifier for this challenge.
    pub challenge_id: UnlockChallengeId,
    /// When the challenge was initiated.
    pub initiated_at: DateTime<Local>,
}

impl UnlockChallenge {
    /// Minimum cooldown in seconds before the unlock can be applied.
    pub const COOLDOWN_SECONDS: i64 = 5 * 60;

    /// Minimum word count for the reflection text.
    pub const MIN_WORDS: u32 = 20;

    /// The placeholder prompt sent to the client for localization.
    pub const REFLECTION_PROMPT: &'static str = "What's making you want to stop right now?";

    /// Creates a new challenge with the current timestamp and a fresh UUID.
    pub fn new() -> Self {
        Self {
            challenge_id: UnlockChallengeId::from(Uuid::now_v7()),
            initiated_at: Local::now(),
        }
    }

    /// Returns true if at least `COOLDOWN_SECONDS` have elapsed since `initiated_at`.
    pub fn is_cooldown_elapsed(&self) -> bool {
        let elapsed = Local::now() - self.initiated_at;
        elapsed.num_seconds() >= Self::COOLDOWN_SECONDS
    }

    /// How many seconds remain until the cooldown elapses.
    /// Returns 0 if already elapsed.
    pub fn cooldown_remaining_seconds(&self) -> i64 {
        let elapsed = (Local::now() - self.initiated_at).num_seconds();
        (Self::COOLDOWN_SECONDS - elapsed).max(0)
    }
}

impl Default for UnlockChallenge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn challenge_started(seconds_ago: i64) -> UnlockChallenge {
        UnlockChallenge {
            challenge_id: UnlockChallengeId::from(Uuid::now_v7()),
            initiated_at: Local::now() - Duration::seconds(seconds_ago),
        }
    }

    #[test]
    fn soft_unlock_cooldown_is_exactly_five_minutes() {
        assert_eq!(UnlockChallenge::COOLDOWN_SECONDS, 5 * 60);
    }

    #[test]
    fn soft_unlock_cannot_redeem_before_five_minutes() {
        let challenge = challenge_started(UnlockChallenge::COOLDOWN_SECONDS - 1);
        assert!(!challenge.is_cooldown_elapsed());
        assert!(challenge.cooldown_remaining_seconds() >= 1);
    }

    #[test]
    fn soft_unlock_can_redeem_after_five_minutes() {
        let challenge = challenge_started(UnlockChallenge::COOLDOWN_SECONDS);
        assert!(challenge.is_cooldown_elapsed());
        assert_eq!(challenge.cooldown_remaining_seconds(), 0);
    }
}
