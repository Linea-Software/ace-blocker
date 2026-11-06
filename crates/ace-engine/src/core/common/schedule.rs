//! Local-time schedule windows used to activate network blocks and evaluate
//! schedule-bound locks. Weekdays use Monday-first bitmasks in persistence.
//!
//! An end time of midnight is treated as the end of the selected day, not
//! as an ordinary zero-length interval.

use super::constants;
use crate::core::NetworkBlockId;
use chrono::{self, Datelike, Timelike, Weekday, WeekdaySet};
use nutype::nutype;
use thiserror::Error;
use uuid::Uuid;

#[nutype(derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Display, Deref, AsRef
))]
pub struct ScheduleEntryId(Uuid);

#[derive(Error, Debug)]
pub enum ScheduleEntryError {
    #[error(
        "invalid time string provided: {invalid_value}. Make sure that the time format is HH-MM-SS"
    )]
    TimeFormat {
        invalid_value: String,
        #[source]
        source: chrono::ParseError,
    },

    #[error("invalid UUID value provided: {invalid_value}")]
    UuidParseError {
        invalid_value: String,
        #[source]
        source: uuid::Error,
    },

    #[error("schedule times must have zero seconds and subseconds: {invalid_value}")]
    SubMinutePrecision { invalid_value: chrono::NaiveTime },

    #[error("start time ({start}) must be before or equal to end time ({end})")]
    StartTimeAfterEndTime {
        start: chrono::NaiveTime,
        end: chrono::NaiveTime,
    },
}

/// A weekday-filtered time window belonging to one network block.
/// Times are evaluated against the caller's local wall clock.
#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    id: ScheduleEntryId,
    parent_list_id: NetworkBlockId,
    start: chrono::NaiveTime,
    end: chrono::NaiveTime,
    weekday_set: WeekdaySet,
}

pub fn bitmask_to_weekday_set(schedule_bitmask: i32) -> WeekdaySet {
    let mut weekday_set = WeekdaySet::EMPTY;
    for day in WeekdaySet::ALL.iter(Weekday::Mon) {
        if schedule_bitmask & (1 << day.num_days_from_monday()) != 0 {
            weekday_set.insert(day);
        }
    }
    weekday_set
}

pub fn weekday_set_to_bitmask(weekday_set: &WeekdaySet) -> i32 {
    let mut bitmask = 0;
    for day in weekday_set.iter(Weekday::Mon) {
        bitmask |= 1 << day.num_days_from_monday();
    }
    bitmask
}

impl ScheduleEntry {
    fn validate_minute_precision(time: chrono::NaiveTime) -> Result<(), ScheduleEntryError> {
        if time.second() != 0 || time.nanosecond() != 0 {
            return Err(ScheduleEntryError::SubMinutePrecision {
                invalid_value: time,
            });
        }
        Ok(())
    }

    pub fn from_raw(
        schedule_entry_id: String,
        parent_list_id: String,
        start_time: &str,
        end_time: &str,
        schedule_bitmask: i32,
    ) -> Result<ScheduleEntry, ScheduleEntryError> {
        let weekday_set = bitmask_to_weekday_set(schedule_bitmask);

        let start = chrono::NaiveTime::parse_from_str(start_time, constants::TIME_FORMAT).map_err(
            |error| ScheduleEntryError::TimeFormat {
                invalid_value: start_time.to_string(),
                source: error,
            },
        )?;
        let end = chrono::NaiveTime::parse_from_str(end_time, constants::TIME_FORMAT).map_err(
            |error| ScheduleEntryError::TimeFormat {
                invalid_value: end_time.to_string(),
                source: error,
            },
        )?;

        Self::validate_minute_precision(start)?;
        Self::validate_minute_precision(end)?;

        if start > end && end != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return Err(ScheduleEntryError::StartTimeAfterEndTime { start, end });
        }

        Ok(ScheduleEntry {
            id: ScheduleEntryId::from(Uuid::parse_str(&schedule_entry_id).map_err(|error| {
                ScheduleEntryError::UuidParseError {
                    invalid_value: schedule_entry_id,
                    source: error,
                }
            })?),
            parent_list_id: NetworkBlockId::from(Uuid::parse_str(&parent_list_id).map_err(
                |error| ScheduleEntryError::UuidParseError {
                    invalid_value: parent_list_id,
                    source: error,
                },
            )?),
            start,
            end,
            weekday_set,
        })
    }

    pub fn from(
        schedule_entry_id: ScheduleEntryId,
        parent_list_id: NetworkBlockId,
        start_time: chrono::NaiveTime,
        end_time: chrono::NaiveTime,
        weekday_set: WeekdaySet,
    ) -> Result<ScheduleEntry, ScheduleEntryError> {
        Self::validate_minute_precision(start_time)?;
        Self::validate_minute_precision(end_time)?;

        if start_time > end_time && end_time != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return Err(ScheduleEntryError::StartTimeAfterEndTime {
                start: start_time,
                end: end_time,
            });
        }

        Ok(ScheduleEntry {
            id: schedule_entry_id,
            parent_list_id,
            start: start_time,
            end: end_time,
            weekday_set,
        })
    }

    pub fn get_id(&self) -> ScheduleEntryId {
        self.id.clone()
    }

    pub fn get_parent_list_id(&self) -> NetworkBlockId {
        self.parent_list_id.clone()
    }

    /// Tests the time of day only; callers must check the weekday separately.
    /// An end bound of midnight means end of day; both bounds at midnight
    /// represent a full-day window.
    pub fn contains(&self, time: chrono::NaiveTime) -> bool {
        if self.start <= self.end && self.end != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            self.start <= time && time <= self.end
        } else {
            // Case where end is 00:00:00 (meaning end of day) or wraps around
            // If start == end == 00:00:00, it also falls here and returns true for all times
            time >= self.start || (self.end != self.start && time <= self.end)
        }
    }

    pub fn has_weekday(&self, weekday: chrono::Weekday) -> bool {
        self.weekday_set.contains(weekday)
    }

    pub fn get_start(&self) -> chrono::NaiveTime {
        self.start
    }

    pub fn get_end(&self) -> chrono::NaiveTime {
        self.end
    }

    pub fn get_weekday_set(&self) -> WeekdaySet {
        self.weekday_set
    }

    pub fn is_strictier_than(&self, other: &ScheduleEntry) -> bool {
        if self.start <= other.start && self.end >= other.end {
            return other
                .get_weekday_set()
                .iter(Weekday::Mon)
                .all(|weekday| self.get_weekday_set().contains(weekday));
        }
        false
    }
}

/// A block is scheduled whenever at least one entry matches local time.
#[derive(Debug, Clone)]
pub struct Schedule {
    entries: Vec<ScheduleEntry>,
    // Pinned when the block is created; changing Windows timezone does not
    // silently move configured weekday/time windows.
    timezone: chrono_tz::Tz,
}

pub fn system_timezone() -> chrono_tz::Tz {
    iana_time_zone::get_timezone()
        .ok()
        .and_then(|name| name.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

impl Default for Schedule {
    fn default() -> Self {
        Self::from_in_zone(Vec::new(), chrono_tz::UTC)
    }
}

impl Schedule {
    pub fn from(schedule_entries: Vec<ScheduleEntry>) -> Schedule {
        Self::from_in_zone(schedule_entries, system_timezone())
    }

    pub fn from_in_zone(entries: Vec<ScheduleEntry>, timezone: chrono_tz::Tz) -> Self {
        Self { entries, timezone }
    }

    pub fn timezone(&self) -> chrono_tz::Tz {
        self.timezone
    }

    /// Evaluation of an authenticated UTC instant in the policy's fixed
    /// IANA zone. DST transitions are resolved by chrono-tz.
    pub fn is_active_at_utc(&self, utc: chrono::DateTime<chrono::Utc>) -> bool {
        let wall = utc.with_timezone(&self.timezone);
        self.entries
            .iter()
            .any(|entry| entry.has_weekday(wall.weekday()) && entry.contains(wall.time()))
    }

    /// Returns true if an authenticated uncertainty interval could overlap
    /// any configured civil-time window. Checking only the interval endpoints
    /// is unsound: a one-minute window can lie wholly between them. Schedule
    /// boundaries have minute precision, so inspect all included UTC minute
    /// transitions, which also covers DST gaps and repeated civil hours.
    pub fn overlaps_utc_interval(
        &self,
        earliest: chrono::DateTime<chrono::Utc>,
        latest: chrono::DateTime<chrono::Utc>,
    ) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let duration = latest.signed_duration_since(earliest);
        // A malformed or excessively uncertain interval is not enough to
        // prove we're outside a lock/active schedule. No unbounded loops.
        if duration < chrono::Duration::zero() || duration >= chrono::Duration::minutes(10) {
            return true;
        }
        if self.is_active_at_utc(earliest) || self.is_active_at_utc(latest) {
            return true;
        }
        let mut minute = (earliest.timestamp().div_euclid(60) + 1) * 60;
        while let Some(instant) = chrono::DateTime::<chrono::Utc>::from_timestamp(minute, 0) {
            if instant >= latest {
                break;
            }
            if self.is_active_at_utc(instant) {
                return true;
            }
            minute += 60;
        }
        false
    }

    /// Local-time form retained for pure call-site fixtures. The live
    /// engine calls is_active_at_utc with authenticated time instead.
    pub fn is_active_at(&self, time: &chrono::DateTime<chrono::Local>) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.has_weekday(time.weekday()) && entry.contains(time.time()))
    }

    pub fn get_entries(&self) -> &Vec<ScheduleEntry> {
        &self.entries
    }

    pub fn is_strictier_than(&self, other: &ScheduleEntry) -> bool {
        // End at midnight means 24:00 of the selected day, not 00:00 at
        // its beginning. Compare numeric offsets to prevent a locked
        // 22:00–00:00 window from being shortened unnoticed.
        let end_of_day = |time: chrono::NaiveTime| {
            if time == chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
                24 * 60 * 60
            } else {
                time.num_seconds_from_midnight()
            }
        };

        for weekday in other.get_weekday_set().iter(Weekday::Mon) {
            let mut intervals: Vec<_> = self
                .entries
                .iter()
                .filter(|entry| entry.has_weekday(weekday))
                .map(|entry| {
                    (
                        entry.get_start().num_seconds_from_midnight(),
                        end_of_day(entry.get_end()),
                    )
                })
                .collect();

            intervals.sort_by_key(|interval| interval.0);

            let mut current_pos = other.get_start().num_seconds_from_midnight();
            let required_end = end_of_day(other.get_end());
            for (start, end) in intervals {
                if start <= current_pos {
                    current_pos = current_pos.max(end);
                } else {
                    break;
                }
                if current_pos >= required_end {
                    break;
                }
            }

            if current_pos < required_end {
                return false;
            }
        }
        true
    }

    /// Whether `other` covers every window in `self`.
    ///
    /// Despite the method name, identical coverage also returns `true`.
    /// Callers must not interpret the result as a strict superset check.
    pub fn is_strictly_more_restrictive_than(&self, other: &Schedule) -> bool {
        self.entries
            .iter()
            .all(|entry| other.is_strictier_than(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

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

    fn get_date_range_iterator(start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
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

    fn create_entry(start: &str, end: &str, bitmask: i32) -> ScheduleEntry {
        let id = Uuid::now_v7().to_string();
        let parent_id = Uuid::now_v7().to_string();
        ScheduleEntry::from_raw(id, parent_id, start, end, bitmask).unwrap()
    }

    #[test]
    fn rejects_subminute_times_at_all_domain_entry_points() {
        let id = Uuid::now_v7();
        let block_id = Uuid::now_v7();
        for (start, end) in [("08:30:45", "09:30:00"), ("08:30:00", "09:30:01")] {
            assert!(matches!(
                ScheduleEntry::from_raw(id.to_string(), block_id.to_string(), start, end, 1),
                Err(ScheduleEntryError::SubMinutePrecision { .. })
            ));
        }
        let weekday_set = bitmask_to_weekday_set(1);
        let fractional = NaiveTime::from_hms_nano_opt(8, 30, 0, 1).unwrap();
        assert!(matches!(
            ScheduleEntry::from(
                ScheduleEntryId::from(id),
                NetworkBlockId::from(block_id),
                fractional,
                NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                weekday_set
            ),
            Err(ScheduleEntryError::SubMinutePrecision { .. })
        ));
        let valid = ScheduleEntry::from_raw(
            id.to_string(),
            block_id.to_string(),
            "08:30:00",
            "00:00:00",
            1,
        );
        assert!(valid.is_ok());
    }

    #[test]
    fn test_entry_is_stricter_basic() {
        let entry1 = create_entry("09:00:00", "12:00:00", 1); // Mon 9-12
        let entry2 = create_entry("10:00:00", "11:00:00", 1); // Mon 10-11

        assert!(entry1.is_strictier_than(&entry2));
        assert!(!entry2.is_strictier_than(&entry1));
    }

    #[test]
    fn test_entry_is_stricter_same() {
        let entry1 = create_entry("09:00:00", "12:00:00", 1);
        assert!(entry1.is_strictier_than(&entry1));
    }

    #[test]
    fn test_entry_is_stricter_weekdays() {
        let entry1 = create_entry("09:00:00", "12:00:00", 3); // Mon+Tue 9-12
        let entry2 = create_entry("09:00:00", "12:00:00", 1); // Mon 9-12

        assert!(entry1.is_strictier_than(&entry2));
        assert!(!entry2.is_strictier_than(&entry1));
    }

    #[test]
    fn test_schedule_is_stricter_combined() {
        let entry1 = create_entry("09:00:00", "11:00:00", 1); // Mon 9-11
        let entry2 = create_entry("11:00:00", "13:00:00", 1); // Mon 11-13
        let schedule = Schedule::from(vec![entry1, entry2]);

        let other = create_entry("10:00:00", "12:00:00", 1); // Mon 10-12
        assert!(schedule.is_strictier_than(&other));

        let other_overlap = create_entry("08:00:00", "12:00:00", 1); // Mon 8-12
        assert!(!schedule.is_strictier_than(&other_overlap));
    }

    #[test]
    fn test_schedule_is_stricter_multi_day() {
        let entry1 = create_entry("09:00:00", "13:00:00", 1); // Mon 9-13
        let entry2 = create_entry("09:00:00", "13:00:00", 2); // Tue 9-13
        let schedule = Schedule::from(vec![entry1, entry2]);

        let other_mon = create_entry("10:00:00", "12:00:00", 1);
        let other_tue = create_entry("10:00:00", "12:00:00", 2);
        let other_both = create_entry("10:00:00", "12:00:00", 3);

        assert!(schedule.is_strictier_than(&other_mon));
        assert!(schedule.is_strictier_than(&other_tue));
        assert!(schedule.is_strictier_than(&other_both));

        let other_wed = create_entry("10:00:00", "12:00:00", 4); // Wed
        assert!(!schedule.is_strictier_than(&other_wed));
    }

    #[test]
    fn test_is_active_at_comprehensive() {
        // Mon-Fri, 9:00 - 17:00
        // Mon is bit 0, Tue is bit 1, ..., Fri is bit 4.
        // 0b11111 = 31
        let entry = create_entry("09:00:00", "17:00:00", 31);
        let schedule = Schedule::from(vec![entry]);

        // Test over a full week starting from a Monday
        let start_date = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(); // 2026-02-09 is Monday
        let end_date = start_date + Duration::days(6); // Until Sunday

        let dates = get_date_range_iterator(start_date, end_date);
        let times = get_time_range_iterator(0, 0, 23, 59);

        for date in dates {
            let is_covered_weekday = date.weekday().number_from_monday() <= 5; // 1 to 5 are Mon-Fri

            for (hour, minute) in &times {
                let time = NaiveTime::from_hms_opt(*hour, *minute, 0).unwrap();
                let date_time = Local
                    .from_local_datetime(&NaiveDateTime::new(date, time))
                    .unwrap();

                let should_be_active = is_covered_weekday
                    && time >= NaiveTime::from_hms_opt(9, 0, 0).unwrap()
                    && time <= NaiveTime::from_hms_opt(17, 0, 0).unwrap();

                assert_eq!(
                    schedule.is_active_at(&date_time),
                    should_be_active,
                    "Schedule activity mismatch at {:?} (weekday: {})",
                    date_time,
                    date.weekday()
                );
            }
        }
    }

    #[test]
    fn midnight_end_preserves_existing_day_boundary_semantics() {
        let entry = create_entry("22:00:00", "00:00:00", 1);
        assert!(entry.contains(NaiveTime::from_hms_opt(22, 0, 0).unwrap()));
        assert!(entry.contains(NaiveTime::from_hms_opt(23, 59, 59).unwrap()));
        assert!(entry.contains(NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
        assert!(!entry.contains(NaiveTime::from_hms_opt(21, 59, 59).unwrap()));
        assert!(!entry.contains(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
    }

    #[test]
    fn locked_midnight_windows_cannot_be_shortened() {
        let protected = create_entry("22:00:00", "00:00:00", 1);
        let shortened = Schedule::from(vec![create_entry("22:00:00", "23:00:00", 1)]);
        let unchanged = Schedule::from(vec![create_entry("22:00:00", "00:00:00", 1)]);
        let extended = Schedule::from(vec![create_entry("21:00:00", "00:00:00", 1)]);
        assert!(!shortened.is_strictier_than(&protected));
        assert!(unchanged.is_strictier_than(&protected));
        assert!(extended.is_strictier_than(&protected));

        let full_day = create_entry("00:00:00", "00:00:00", 1);
        assert!(!shortened.is_strictier_than(&full_day));
        assert!(Schedule::from(vec![full_day.clone()]).is_strictier_than(&full_day));
    }

    #[test]
    fn test_invalid_range() {
        let id = uuid::Uuid::now_v7().to_string();
        let parent_id = uuid::Uuid::now_v7().to_string();
        let result = ScheduleEntry::from_raw(id, parent_id, "22:00:00", "02:00:00", 1);

        assert!(result.is_err());
        match result {
            Err(ScheduleEntryError::StartTimeAfterEndTime { start, end }) => {
                assert_eq!(start, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
                assert_eq!(end, NaiveTime::from_hms_opt(2, 0, 0).unwrap());
            }
            _ => panic!("Expected StartTimeAfterEndTime error"),
        }
    }
}

#[cfg(test)]
mod pinned_timezone_tests {
    use super::*;
    use chrono::{TimeZone, Utc, WeekdaySet};

    fn sunday_window(start: (u32, u32), end: (u32, u32)) -> ScheduleEntry {
        ScheduleEntry::from(
            ScheduleEntryId::from(Uuid::now_v7()),
            NetworkBlockId::from(Uuid::now_v7()),
            chrono::NaiveTime::from_hms_opt(start.0, start.1, 0).unwrap(),
            chrono::NaiveTime::from_hms_opt(end.0, end.1, 0).unwrap(),
            {
                let mut days = WeekdaySet::EMPTY;
                days.insert(chrono::Weekday::Sun);
                days
            },
        )
        .unwrap()
    }

    #[test]
    fn a_short_window_inside_roughtime_uncertainty_is_not_missed() {
        use chrono::{NaiveTime, WeekdaySet};
        let id = NetworkBlockId::from(Uuid::now_v7());
        let start = NaiveTime::from_hms_opt(12, 1, 0).unwrap();
        let end = NaiveTime::from_hms_opt(12, 1, 0).unwrap();
        let mut days = WeekdaySet::EMPTY;
        days.insert(chrono::Weekday::Mon);
        let entry =
            ScheduleEntry::from(ScheduleEntryId::from(Uuid::now_v7()), id, start, end, days)
                .unwrap();
        let zone = "Etc/UTC".parse().unwrap();
        let schedule = Schedule::from_in_zone(vec![entry], zone);
        let earliest = Utc.with_ymd_and_hms(2026, 11, 2, 12, 0, 30).unwrap();
        let latest = Utc.with_ymd_and_hms(2026, 11, 2, 12, 2, 0).unwrap();
        assert!(!schedule.is_active_at_utc(earliest));
        assert!(!schedule.is_active_at_utc(latest));
        assert!(schedule.overlaps_utc_interval(earliest, latest));
        assert!(!schedule.overlaps_utc_interval(
            Utc.with_ymd_and_hms(2026, 11, 2, 12, 2, 1).unwrap(),
            Utc.with_ymd_and_hms(2026, 11, 2, 12, 3, 0).unwrap(),
        ));
    }

    #[test]
    fn spring_forward_skips_missing_civil_hour() {
        let zone = "Europe/Rome".parse().unwrap();
        let schedule = Schedule::from_in_zone(vec![sunday_window((3, 0), (3, 45))], zone);
        assert!(!schedule.is_active_at_utc(Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap()));
        assert!(schedule.is_active_at_utc(Utc.with_ymd_and_hms(2026, 3, 29, 1, 30, 0).unwrap()));
    }

    #[test]
    fn fall_back_repeated_civil_hour_is_enforced_twice() {
        let zone = "Europe/Rome".parse().unwrap();
        let schedule = Schedule::from_in_zone(vec![sunday_window((2, 0), (2, 45))], zone);
        assert!(schedule.is_active_at_utc(Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap()));
        assert!(schedule.is_active_at_utc(Utc.with_ymd_and_hms(2026, 10, 25, 1, 30, 0).unwrap()));
    }

    #[test]
    fn pinned_iana_zone_does_not_follow_system_timezone() {
        let entry = sunday_window((2, 0), (2, 45));
        let rome = Schedule::from_in_zone(vec![entry.clone()], "Europe/Rome".parse().unwrap());
        let new_york = Schedule::from_in_zone(vec![entry], "America/New_York".parse().unwrap());
        let instant = Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap();
        assert!(rome.is_active_at_utc(instant));
        assert!(!new_york.is_active_at_utc(instant));
        assert_eq!(rome.timezone().name(), "Europe/Rome");
    }
}
