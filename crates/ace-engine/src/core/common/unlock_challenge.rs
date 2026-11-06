use chrono::{DateTime, Local};
use nutype::nutype;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[nutype(derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    From,
    Into,
    Display,
    Serialize,
    Deserialize
))]
pub struct UnlockChallengeId(Uuid);

/// A five-minute, in-memory soft-unlock challenge. Only the monotonic
/// elapsed duration authorizes redemption; local wall-clock changes have
/// no effect. Restart discards all challenges.
#[derive(Debug, Clone)]
pub struct UnlockChallenge {
    pub challenge_id: UnlockChallengeId,
    /// Display-only initiation timestamp, not used for policy decisions.
    pub initiated_at: DateTime<Local>,
    started_monotonic: Instant,
}

impl UnlockChallenge {
    pub const COOLDOWN_SECONDS: i64 = 5 * 60;
    pub const MIN_WORDS: u32 = 20;
    pub const REFLECTION_PROMPT: &'static str = "What's making you want to stop right now?";

    pub fn new() -> Self {
        Self {
            challenge_id: UnlockChallengeId::from(Uuid::now_v7()),
            initiated_at: Local::now(),
            started_monotonic: Instant::now(),
        }
    }

    pub fn is_cooldown_elapsed(&self) -> bool {
        self.started_monotonic.elapsed() >= Duration::from_secs(Self::COOLDOWN_SECONDS as u64)
    }

    pub fn cooldown_remaining_seconds(&self) -> i64 {
        let remaining = Duration::from_secs(Self::COOLDOWN_SECONDS as u64)
            .saturating_sub(self.started_monotonic.elapsed());
        // Round up so the UI never reports zero before redemption is legal.
        i64::try_from(remaining.as_secs()).unwrap_or(i64::MAX)
            + i64::from(remaining.subsec_nanos() > 0)
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

    fn challenge_started(seconds_ago: u64) -> UnlockChallenge {
        UnlockChallenge {
            challenge_id: UnlockChallengeId::from(Uuid::now_v7()),
            initiated_at: Local::now() + chrono::Duration::days(100),
            started_monotonic: Instant::now() - Duration::from_secs(seconds_ago),
        }
    }

    #[test]
    fn soft_unlock_cooldown_is_exactly_five_minutes() {
        assert_eq!(UnlockChallenge::COOLDOWN_SECONDS, 300);
    }

    #[test]
    fn wall_clock_does_not_complete_cooldown() {
        let challenge = challenge_started(299);
        assert!(!challenge.is_cooldown_elapsed());
        assert!(challenge.cooldown_remaining_seconds() >= 1);
    }

    #[test]
    fn monotonic_elapsed_completes_cooldown() {
        let challenge = challenge_started(300);
        assert!(challenge.is_cooldown_elapsed());
        assert_eq!(challenge.cooldown_remaining_seconds(), 0);
    }
}
