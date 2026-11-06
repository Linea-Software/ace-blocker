use chrono::Local;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use strum_macros::{AsRefStr, Display, EnumString};
use thiserror::Error;

use crate::core::Schedule;

/// Strictness ladder (least → most restrictive):
/// `Inactive < SoftUnlockable < ActiveDuringSchedule < ActiveUntilLockEnd`
///
/// A lock can only be *upgraded* (moved to a stricter state), never downgraded.
/// `SoftUnlockable` is the only state that allows the client to initiate an
/// unlock flow via the soft-unlock challenge endpoints.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    PartialOrd,
    EnumString,
    Display,
    JsonSchema,
    AsRefStr,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "snake_case")]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LockState {
    #[default]
    Inactive,
    /// Lock is active but can be removed via reflection-challenge flow.
    SoftUnlockable,
    /// Lock is **enforced only while the block's schedule is active** —
    /// "Active During Schedule" (see [`Lock::is_active`]). Outside the
    /// schedule window the lock is not active, so the block/lock can be
    /// edited again. This is the product's intended, name-documented
    /// semantic (the app label is "Active During Schedule"), and it is the
    /// second-strictest *state* on the monotonic ladder: it can never be
    /// downgraded to `SoftUnlockable` or `Inactive` while it is active.
    ///
    /// Note the deliberate divergence of the installer's uninstall gate
    /// (`--check-locks`, `src/lock_check.rs`): it cannot evaluate schedules
    /// and treats this state as *always* active, so a schedule lock blocks
    /// uninstall until the user removes it in the app.
    ActiveDuringSchedule,
    ActiveUntilLockEnd {
        end: chrono::DateTime<chrono::Local>,
    },
}

/// A block's configured lock; enforcement depends on both state and schedule.
#[derive(Debug, Clone, Default)]
pub struct Lock {
    state: LockState,
}

#[derive(Debug, Error)]
pub enum LockParseError {
    #[error(
        "Invalid lock type provided: expected one of 'inactive', 'soft_unlockable', 'active_during_schedule' or 'active_until_lock_end', got {invalid_value} instead"
    )]
    InvalidLockType { invalid_value: String },
    #[error("Invalid date time value provided: {message} ")]
    InvalidDateFormat { message: String },
    #[error(
        "Lock end time is too far in the future: the maximum lock duration is 90 days from now"
    )]
    LockEndTooFar,
    #[error("Lock end must be strictly after the authenticated UTC interval")]
    LockEndNotFuture,
}

impl Lock {
    /// Whether the lock currently constrains the block.
    ///
    /// `ActiveDuringSchedule` is window-bound: it is active only while the
    /// block's schedule matches `now` (the state's name — "Active During
    /// Schedule" — is its spec). `ActiveUntilLockEnd` is active until its
    /// end; `SoftUnlockable` is always active until redeemed; `Inactive` is
    /// never active. Every engine guard (`is_locked`-based) uses this
    /// evaluator, so schedule locks do not constrain edits outside their
    /// window — only the uninstall gate (`lock_check.rs`) is deliberately
    /// stricter.
    pub fn is_active(&self, schedule: &Schedule, now: &chrono::DateTime<chrono::Local>) -> bool {
        match self.state {
            LockState::Inactive => false,
            LockState::SoftUnlockable => true,
            LockState::ActiveDuringSchedule => schedule.is_active_at(now),
            LockState::ActiveUntilLockEnd { end } => now <= &end,
        }
    }

    pub fn get_state(&self) -> LockState {
        self.state
    }

    pub fn get_end_time(&self) -> Option<chrono::DateTime<Local>> {
        match self.state {
            LockState::ActiveUntilLockEnd { end } => Some(end),
            _ => None,
        }
    }

    pub fn from(lock_state: LockState) -> Lock {
        let state = lock_state;

        Lock { state }
    }

    /// Parses a persisted/API lock state, enforcing the time-lock duration cap.
    /// A timed lock requires a local-time end timestamp.
    pub fn from_raw(
        lock_state_str: String,
        lock_end: Option<String>,
    ) -> Result<Lock, LockParseError> {
        let state: LockState = match lock_state_str.as_str() {
            "inactive" => LockState::Inactive,
            "soft_unlockable" => LockState::SoftUnlockable,
            "active_during_schedule" => LockState::ActiveDuringSchedule,
            "active_until_lock_end" => {
                if let Some(lock_end_str) = lock_end {
                    // An offset-bearing stored deadline is an absolute
                    // instant, unaffected by future Windows timezone changes.
                    let end_local =
                        if let Ok(end) = chrono::DateTime::parse_from_rfc3339(&lock_end_str) {
                            end.with_timezone(&Local)
                        } else {
                            let end = chrono::NaiveDateTime::parse_from_str(
                                &lock_end_str,
                                crate::core::common::constants::DATETIME_FORMAT,
                            )
                            .map_err(|error| {
                                LockParseError::InvalidDateFormat {
                                    message: error.to_string(),
                                }
                            })?;
                            end.and_local_timezone(Local).single().ok_or_else(|| {
                                LockParseError::InvalidDateFormat {
                                    message: "ambiguous or nonexistent local lock deadline".into(),
                                }
                            })?
                        };

                    LockState::ActiveUntilLockEnd { end: end_local }
                } else {
                    Err(LockParseError::InvalidDateFormat {
                        message: "No value has been provided".to_string(),
                    })?
                }
            }

            _ => Err(LockParseError::InvalidLockType {
                invalid_value: lock_state_str,
            })?,
        };

        Ok(Lock { state })
    }

    pub fn from_user_raw(
        lock_state_str: String,
        lock_end: Option<String>,
    ) -> Result<Lock, LockParseError> {
        // User-supplied new deadlines must be unambiguous absolute instants.
        // Only the legacy database loader accepts offset-less dates.
        if lock_state_str == "active_until_lock_end"
            && !lock_end
                .as_ref()
                .is_some_and(|end| chrono::DateTime::parse_from_rfc3339(end).is_ok())
        {
            return Err(LockParseError::InvalidDateFormat {
                message: "a timed lock requires an RFC3339 timestamp with UTC offset".into(),
            });
        }
        let lock = Self::from_raw(lock_state_str, lock_end)?;
        if let LockState::ActiveUntilLockEnd { end } = lock.state {
            let trusted = crate::trusted_time::current_interval()
                .filter(|interval| interval.quality == crate::trusted_time::TimeQuality::Verified)
                .ok_or_else(|| LockParseError::InvalidDateFormat {
                    message: "fresh authenticated UTC unavailable".into(),
                })?;
            Self::validate_new_timed_deadline(end.with_timezone(&chrono::Utc), trusted)?;
        }
        Ok(lock)
    }

    fn validate_new_timed_deadline(
        end: chrono::DateTime<chrono::Utc>,
        trusted: crate::trusted_time::UtcInterval,
    ) -> Result<(), LockParseError> {
        // The latest possible current instant must still precede the deadline.
        // A past deadline could otherwise upgrade SoftUnlockable to an
        // already-expired timed lock and avoid its reflection cooldown.
        if end <= trusted.latest {
            return Err(LockParseError::LockEndNotFuture);
        }
        // Use earliest to ensure the 90-day bound isn't relaxed by ambiguity.
        if end > trusted.earliest + chrono::Duration::days(90) {
            return Err(LockParseError::LockEndTooFar);
        }
        Ok(())
    }

    /// Whether this lock is a *strict* upgrade over `other`.
    /// Timed locks must extend the previous deadline; equal states are not
    /// upgrades even though an inactive lock may be replaced by the manager.
    pub fn is_stricter_than(&self, other: &Lock) -> bool {
        match (self.state, other.state) {
            // SoftUnlockable is stricter than Inactive
            (LockState::SoftUnlockable, LockState::Inactive) => true,
            (LockState::ActiveDuringSchedule, LockState::Inactive)
            | (LockState::ActiveDuringSchedule, LockState::SoftUnlockable) => true,
            (
                LockState::ActiveUntilLockEnd { end: self_end },
                LockState::ActiveUntilLockEnd { end: other_end },
            ) => self_end > other_end,
            // ActiveUntilLockEnd is stricter than everything below it
            (LockState::ActiveUntilLockEnd { .. }, LockState::Inactive)
            | (LockState::ActiveUntilLockEnd { .. }, LockState::SoftUnlockable)
            | (LockState::ActiveUntilLockEnd { .. }, LockState::ActiveDuringSchedule) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, WeekdaySet};
    use uuid::Uuid;

    use crate::core::{NetworkBlockId, ScheduleEntry, ScheduleEntryId};

    use super::*;

    fn get_time_range_iterator(
        start_hour: u32,
        start_minute: u32,
        end_hour: u32,
        end_minute: u32,
    ) -> Vec<(u32, u32)> {
        let mut result = Vec::new();
        let mut current_hour = start_hour;
        let mut current_minute = start_minute;

        while current_hour < end_hour || (current_hour == end_hour && current_minute <= end_minute)
        {
            result.push((current_hour, current_minute));
            current_minute += 1;
            if current_minute == 60 {
                current_minute = 0;
                current_hour += 1;
            }
            if current_hour == 24 {
                break;
            }
        }

        result
    }

    fn get_date_range_iterator(
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Vec<chrono::NaiveDate> {
        let mut result = Vec::new();
        let mut current = start;
        while current <= end {
            result.push(current);
            if let Some(next) = current.succ_opt() {
                current = next;
            } else {
                break;
            }
        }
        result
    }

    #[test]
    fn user_timed_lock_requires_explicit_utc_offset_even_when_offline() {
        let result = Lock::from_user_raw(
            "active_until_lock_end".into(),
            Some("2026-11-06T12:00:00".into()),
        );
        assert!(matches!(
            result,
            Err(LockParseError::InvalidDateFormat { .. })
        ));
    }

    #[test]
    fn rejects_expired_or_ambiguous_new_timed_deadline() {
        let earliest = chrono::Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap();
        let latest = earliest + chrono::Duration::seconds(5);
        let trusted = crate::trusted_time::UtcInterval {
            earliest,
            latest,
            quality: crate::trusted_time::TimeQuality::Verified,
        };
        assert!(matches!(
            Lock::validate_new_timed_deadline(earliest, trusted),
            Err(LockParseError::LockEndNotFuture)
        ));
        assert!(matches!(
            Lock::validate_new_timed_deadline(latest, trusted),
            Err(LockParseError::LockEndNotFuture)
        ));
        assert!(matches!(
            Lock::validate_new_timed_deadline(earliest + chrono::Duration::days(91), trusted),
            Err(LockParseError::LockEndTooFar)
        ));
        assert!(
            Lock::validate_new_timed_deadline(latest + chrono::Duration::minutes(1), trusted)
                .is_ok()
        );
    }

    #[test]
    fn test_inactive_lock() {
        let lock = Lock::from(LockState::Inactive);
        let now = chrono::Local::now();
        let schedule = Schedule::from(vec![]);

        assert!(
            !lock.is_active(&schedule, &now),
            "The lock should be inactive"
        );
        assert!(
            lock.get_state() == LockState::Inactive,
            "Lock state mismatch, it should be LockState::Inactive"
        )
    }

    #[test]
    fn test_active_along_schedule_lock() {
        let lock = Lock::from(LockState::ActiveDuringSchedule);

        let schedule = Schedule::from(vec![
            ScheduleEntry::from(
                ScheduleEntryId::from(Uuid::now_v7()),
                NetworkBlockId::from(Uuid::now_v7()),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                chrono::NaiveTime::from_hms_opt(11, 30, 0).unwrap(),
                WeekdaySet::ALL,
            )
            .unwrap(),
            ScheduleEntry::from(
                ScheduleEntryId::from(Uuid::now_v7()),
                NetworkBlockId::from(Uuid::now_v7()),
                chrono::NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                chrono::NaiveTime::from_hms_opt(22, 30, 0).unwrap(),
                WeekdaySet::ALL,
            )
            .unwrap(),
        ]);
        // Checking all of the
        let iter = get_time_range_iterator(0, 0, 11, 30);
        for (hour, minute) in iter {
            let active_datetime = Local
                .from_local_datetime(&chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
                ))
                .unwrap();

            assert!(
                lock.is_active(&schedule, &active_datetime),
                "The lock should be active at this time"
            );
        }

        let iter = get_time_range_iterator(15, 0, 22, 30);
        for (hour, minute) in iter {
            let active_datetime = Local
                .from_local_datetime(&chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
                ))
                .unwrap();

            assert!(
                lock.is_active(&schedule, &active_datetime),
                "The lock should be active at this time"
            );
        }

        let iter = get_time_range_iterator(11, 31, 14, 59);

        for (hour, minute) in iter {
            let inactive_datetime = Local
                .from_local_datetime(&chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
                ))
                .unwrap();

            assert!(
                !lock.is_active(&schedule, &inactive_datetime),
                "The lock should be inactive at this time"
            );
        }
    }

    #[test]
    fn test_active_until_lock_end_large_range() {
        let now = chrono::Local::now();
        let lock_end = now + Duration::days(365);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end });
        let schedule = Schedule::from(vec![]);

        let dates = get_date_range_iterator(now.date_naive(), lock_end.date_naive());

        for date in dates {
            // Test at the same time as 'now'
            let test_time = date
                .and_time(now.time())
                .and_local_timezone(chrono::Local)
                .unwrap();

            if test_time <= lock_end {
                assert!(
                    lock.is_active(&schedule, &test_time),
                    "Lock should be active at {}",
                    test_time
                );
            } else {
                assert!(
                    !lock.is_active(&schedule, &test_time),
                    "Lock should be inactive at {}",
                    test_time
                );
            }
        }

        // Also check one day after
        let day_after = lock_end + Duration::days(1);
        assert!(
            !lock.is_active(&schedule, &day_after),
            "Lock should be inactive after the end date"
        );
    }
}
