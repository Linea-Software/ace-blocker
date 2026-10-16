use crate::core::{NetworkBlock, NetworkBlockId, Schedule, ScheduleEntry, ScheduleEntryId};
use chrono::{Timelike, Weekday, WeekdaySet};

use crate::db::models::NetworkBlockScheduleEntryModel;

use super::{NetworkManager, error::NetworkManagerOpErr};

impl NetworkManager {
    pub fn get_schedule_entries_of(
        &mut self,
        network_block_id: &NetworkBlockId,
    ) -> Result<&Vec<ScheduleEntry>, NetworkManagerOpErr> {
        Ok(self
            .get_network_block_from(network_block_id)
            .ok_or(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            })?
            .get_schedule()
            .get_entries())
    }

    pub fn get_net_list_of_schedule_entry(
        &self,
        schedule_entry_id: ScheduleEntryId,
    ) -> Result<&NetworkBlock, NetworkManagerOpErr> {
        let net_list = self
            .get_all_network_blocks()
            .into_iter()
            .find(|net_list| {
                net_list
                    .get_schedule()
                    .get_entries()
                    .iter()
                    .any(|entry| entry.get_id() == schedule_entry_id)
            })
            .ok_or(NetworkManagerOpErr::ScheduleEntryNotFound {
                id: schedule_entry_id,
            })?;

        Ok(net_list)
    }

    /// Extends a block's covered windows; expansion remains allowed under lock.
    pub fn add_schedule_entries(
        &mut self,
        network_block_id: &NetworkBlockId,
        new_entries: Vec<ScheduleEntry>,
    ) -> Result<(), NetworkManagerOpErr> {
        let net_list = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        let mut existing_entries = net_list.get_schedule().get_entries().clone();
        existing_entries.extend(new_entries);

        // Note: Adding schedule entries is always allowed because it only extends
        // the covered time windows, making the block stricter (more restrictive).

        self.process_and_save_schedule(network_block_id, existing_entries)
    }

    /// When locked, permits edits only if the proposed complete schedule still
    /// covers every previously enforced window.
    pub fn edit_schedule_entry(
        &mut self,
        edited_schedule_entry: ScheduleEntry,
    ) -> Result<(), NetworkManagerOpErr> {
        let net_list = self.get_net_list_of_schedule_entry(edited_schedule_entry.get_id())?;

        let existing_entries = net_list.get_schedule().get_entries();

        let mut entries: Vec<ScheduleEntry> = existing_entries
            .iter()
            .filter(|error| error.get_id() != edited_schedule_entry.get_id())
            .cloned()
            .collect();

        entries.push(edited_schedule_entry);

        if net_list.is_locked(&chrono::Local::now()) {
            let proposed_schedule = Schedule::from(entries.clone());
            if !net_list
                .get_schedule()
                .is_strictly_more_restrictive_than(&proposed_schedule)
            {
                return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                    id: net_list.get_id(),
                });
            }
        }

        self.process_and_save_schedule(&net_list.get_id(), entries)
    }

    pub fn delete_schedule_entry(
        &mut self,
        schedule_entry_id: ScheduleEntryId,
    ) -> Result<(), NetworkManagerOpErr> {
        let net_list = self.get_net_list_of_schedule_entry(schedule_entry_id.clone())?;

        if net_list.is_locked(&chrono::Local::now()) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: net_list.get_id(),
            });
        }

        let existing_entries = net_list.get_schedule().get_entries();

        let entries: Vec<ScheduleEntry> = existing_entries
            .iter()
            .filter(|error| error.get_id() != schedule_entry_id)
            .cloned()
            .collect();

        self.process_and_save_schedule(&net_list.get_id(), entries)
    }

    /// Canonicalizes overlapping/adjacent weekday windows before persistence;
    /// downstream comparisons must not depend on how entries were subdivided.
    fn process_and_save_schedule(
        &mut self,
        network_block_id: &NetworkBlockId,
        all_entries: Vec<ScheduleEntry>,
    ) -> Result<(), NetworkManagerOpErr> {
        // Simplify the schedule by breaking it down into intervals
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        struct Interval {
            day: u8,    // 0=Mon, 6=Sun
            start: u32, // Minutes from midnight
            end: u32,   // Minutes from midnight
        }

        let mut intervals = Vec::new();
        for entry in all_entries {
            let start_minutes = entry.get_start().num_seconds_from_midnight() / 60;
            let end_minutes = entry.get_end().num_seconds_from_midnight() / 60;

            // Handle edge case where end_time is 00:00:00 (next day) -> 24*60
            let end_minutes =
                if end_minutes == 0 && entry.get_end().num_seconds_from_midnight() == 0 {
                    24 * 60
                } else {
                    end_minutes
                };

            for day in WeekdaySet::ALL.iter(Weekday::Mon) {
                if entry.get_weekday_set().contains(day) {
                    intervals.push(Interval {
                        day: day.num_days_from_monday() as u8,
                        start: start_minutes,
                        end: end_minutes,
                    });
                }
            }
        }

        // Sort and merge intervals
        intervals.sort();

        let mut merged_intervals = Vec::new();
        if !intervals.is_empty() {
            let mut current = intervals[0];
            for next in intervals.into_iter().skip(1) {
                if next.day == current.day {
                    // Overlap or Adjacent: current.end >= next.start
                    if current.end >= next.start {
                        current.end = std::cmp::max(current.end, next.end);
                    } else {
                        merged_intervals.push(current);
                        current = next;
                    }
                } else {
                    merged_intervals.push(current);
                    current = next;
                }
            }
            merged_intervals.push(current);
        }

        // Combine identical time windows across weekdays into one persisted entry.
        let mut grouped: std::collections::HashMap<(u32, u32), i32> =
            std::collections::HashMap::new();

        for interval in merged_intervals {
            let mask = grouped.entry((interval.start, interval.end)).or_insert(0);
            *mask |= 1 << interval.day;
        }

        // Convert back to ScheduleEntryModel
        let mut final_models = Vec::new();
        for ((start, end), mask) in grouped {
            let start_h = start / 60;
            let start_m = start % 60;
            let end_h = end / 60;
            let end_m = end % 60;

            let start_str = format!("{:02}:{:02}:00", start_h, start_m);
            // If 24:00, use 00:00
            let end_str = format!("{:02}:{:02}:00", end_h % 24, end_m);

            final_models.push(NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::now_v7().to_string(), // Replaced by the repository during insertion
                network_block_id: (*network_block_id).to_string(),
                start_time: start_str,
                end_time: end_str,
                schedule_days_bitmask: mask,
            });
        }

        self.db
            .replace_schedule_entries(network_block_id, final_models)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to replace schedule entries: {}", error),
            })?;

        // Update In-Memory ContentList
        let reloaded_models = self
            .db
            .get_schedule_entry_models_of(network_block_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to reload schedule entries: {}", error),
            })?;

        let mut schedule_entries_core = Vec::new();
        for model in reloaded_models {
            schedule_entries_core.push(
                ScheduleEntry::from_raw(
                    model.id,
                    model.network_block_id,
                    &model.start_time,
                    &model.end_time,
                    model.schedule_days_bitmask,
                )
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: error.to_string(),
                })?,
            );
        }

        if let Some(content_list) = self.get_mut_network_block_from(network_block_id) {
            content_list.set_schedule(schedule_entries_core);
        }

        Ok(())
    }

    pub(crate) fn load_schedule_entries_of(
        db: &crate::db::EngineDB,
        network_block_id: &uuid::Uuid,
    ) -> Result<Vec<ScheduleEntry>, NetworkManagerOpErr> {
        let models = db
            .get_schedule_entry_models_of(network_block_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?;
        let mut sched_entries = Vec::<ScheduleEntry>::new();

        for model in models.into_iter() {
            let sched_entry = ScheduleEntry::from_raw(
                model.id,
                model.network_block_id,
                &model.start_time,
                &model.end_time,
                model.schedule_days_bitmask,
            )
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: error.to_string(),
            })?;
            sched_entries.push(sched_entry);
        }

        Ok(sched_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Lock, LockState};
    use crate::db::EngineDB;
    use crate::settings::Settings;
    use chrono::{
        Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Weekday,
    };
    use std::sync::Arc;
    use tempfile::tempdir;

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

    fn setup_test_manager() -> (NetworkManager, NetworkBlockId) {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");
        let sites_dir = Settings::shared_test_sites_dir();
        // Configure Settings to use our temp dir
        Settings::set_test_sites_dir(sites_dir.clone());

        let db = EngineDB::from(&db_path).unwrap();
        let db_lock = Arc::new(db);
        let account_manager = crate::manager::AccountManager::from(db_lock.clone()).unwrap();
        let mut manager = NetworkManager::from(db_lock, account_manager).unwrap();

        let list_id = manager.create_network_block("Test List".into()).unwrap();

        (manager, list_id)
    }

    #[test]
    fn test_merge_overlapping_intervals() {
        let (mut manager, list_id) = setup_test_manager();

        let entries = vec![
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "12:00:00".to_string(),
                schedule_days_bitmask: 1, // Mon
            },
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "11:00:00".to_string(),
                end_time: "13:00:00".to_string(),
                schedule_days_bitmask: 1, // Mon
            },
        ];

        let core_entries = entries
            .into_iter()
            .map(|model| {
                ScheduleEntry::from_raw(
                    model.id,
                    model.network_block_id,
                    &model.start_time,
                    &model.end_time,
                    model.schedule_days_bitmask,
                )
                .unwrap()
            })
            .collect();

        manager
            .add_schedule_entries(&list_id, core_entries)
            .unwrap();

        let saved = manager.get_schedule_entries_of(&list_id).unwrap();
        assert_eq!(saved.len(), 1);

        // Verify with iterator
        let times = get_time_range_iterator(10, 0, 13, 0);
        let schedule = manager
            .get_network_block_from(&list_id)
            .unwrap()
            .get_schedule();

        for (hour, minute) in times {
            let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
            let date = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(); // Monday
            let date_time = Local
                .from_local_datetime(&NaiveDateTime::new(date, time))
                .unwrap();

            assert!(
                schedule.is_active_at(&date_time),
                "Schedule should be active at {:02}:{:02} on Monday",
                hour,
                minute
            );
        }
    }

    #[test]
    fn test_merge_bridging_gap() {
        let (mut manager, list_id) = setup_test_manager();

        // Existing: 10-11, 12-13
        let entries1 = vec![
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                schedule_days_bitmask: 1,
            },
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "12:00:00".to_string(),
                end_time: "13:00:00".to_string(),
                schedule_days_bitmask: 1,
            },
        ];
        let core_entries1 = entries1
            .into_iter()
            .map(|m| {
                ScheduleEntry::from_raw(
                    m.id,
                    m.network_block_id,
                    &m.start_time,
                    &m.end_time,
                    m.schedule_days_bitmask,
                )
                .unwrap()
            })
            .collect();

        manager
            .add_schedule_entries(&list_id, core_entries1)
            .unwrap();

        // Bridge: 10:30-12:30
        let entries2 = vec![NetworkBlockScheduleEntryModel {
            id: uuid::Uuid::new_v4().to_string(),
            network_block_id: (*list_id).to_string(),
            start_time: "10:30:00".to_string(),
            end_time: "12:30:00".to_string(),
            schedule_days_bitmask: 1,
        }];
        let core_entries2 = entries2
            .into_iter()
            .map(|m| {
                ScheduleEntry::from_raw(
                    m.id,
                    m.network_block_id,
                    &m.start_time,
                    &m.end_time,
                    m.schedule_days_bitmask,
                )
                .unwrap()
            })
            .collect();

        manager
            .add_schedule_entries(&list_id, core_entries2)
            .unwrap();

        let saved = manager.get_schedule_entries_of(&list_id).unwrap();
        assert_eq!(saved.len(), 1);

        // Verify with iterator
        let times = get_time_range_iterator(10, 0, 13, 0);
        let schedule = manager
            .get_network_block_from(&list_id)
            .unwrap()
            .get_schedule();

        for (hour, minute) in times {
            let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
            let date = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(); // Monday
            let date_time = Local
                .from_local_datetime(&NaiveDateTime::new(date, time))
                .unwrap();

            assert!(
                schedule.is_active_at(&date_time),
                "Schedule should be active at {:02}:{:02} on Monday after bridging gap",
                hour,
                minute
            );
        }
    }

    #[test]
    fn test_merge_adjacent() {
        let (mut manager, list_id) = setup_test_manager();

        let entries = vec![
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                schedule_days_bitmask: 1,
            },
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "11:00:00".to_string(),
                end_time: "12:00:00".to_string(),
                schedule_days_bitmask: 1,
            },
        ];
        let core_entries = entries
            .into_iter()
            .map(|m| {
                ScheduleEntry::from_raw(
                    m.id,
                    m.network_block_id,
                    &m.start_time,
                    &m.end_time,
                    m.schedule_days_bitmask,
                )
                .unwrap()
            })
            .collect();

        manager
            .add_schedule_entries(&list_id, core_entries)
            .unwrap();

        let saved = manager.get_schedule_entries_of(&list_id).unwrap();
        assert_eq!(saved.len(), 1);

        // Verify with iterator
        let times = get_time_range_iterator(10, 0, 12, 0);
        let schedule = manager
            .get_network_block_from(&list_id)
            .unwrap()
            .get_schedule();

        for (hour, minute) in times {
            let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
            let date = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(); // Monday
            let date_time = Local
                .from_local_datetime(&NaiveDateTime::new(date, time))
                .unwrap();

            assert!(
                schedule.is_active_at(&date_time),
                "Schedule should be active at {:02}:{:02} on Monday (adjacent segments)",
                hour,
                minute
            );
        }
    }

    #[test]
    fn test_different_days_same_time() {
        let (mut manager, list_id) = setup_test_manager();

        let entries = vec![
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                schedule_days_bitmask: 1, // Mon
            },
            NetworkBlockScheduleEntryModel {
                id: uuid::Uuid::new_v4().to_string(),
                network_block_id: (*list_id).to_string(),
                start_time: "10:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                schedule_days_bitmask: 2, // Tue
            },
        ];
        let core_entries = entries
            .into_iter()
            .map(|m| {
                ScheduleEntry::from_raw(
                    m.id,
                    m.network_block_id,
                    &m.start_time,
                    &m.end_time,
                    m.schedule_days_bitmask,
                )
                .unwrap()
            })
            .collect();

        manager
            .add_schedule_entries(&list_id, core_entries)
            .unwrap();

        let saved = manager.get_schedule_entries_of(&list_id).unwrap();
        assert_eq!(saved.len(), 1);

        // Verify over a range of days
        let start_date = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(); // Monday
        let end_date = start_date + Duration::days(6); // Until Sunday
        let dates = get_date_range_iterator(start_date, end_date);

        let schedule = manager
            .get_network_block_from(&list_id)
            .unwrap()
            .get_schedule();

        let test_time = NaiveTime::from_hms_opt(10, 30, 0).unwrap();

        for date in dates {
            let date_time = Local
                .from_local_datetime(&NaiveDateTime::new(date, test_time))
                .unwrap();

            let should_be_active = date.weekday() == Weekday::Mon || date.weekday() == Weekday::Tue;

            assert_eq!(
                schedule.is_active_at(&date_time),
                should_be_active,
                "Schedule activity mismatch at {:?} ({})",
                date_time,
                date.weekday()
            );
        }
    }

    #[test]
    fn test_lock_constraints_on_schedule() {
        let (mut manager, list_id) = setup_test_manager();

        // Add an initial entry
        let entry = create_entry_helper(&list_id, "10:00:00", "11:00:00", 1);
        manager
            .add_schedule_entries(&list_id, vec![entry.clone()])
            .unwrap();

        // Lock the network block
        let lock_end = Local::now() + Duration::days(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end });
        manager
            .get_mut_network_block_from(&list_id)
            .unwrap()
            .set_lock(lock);

        // 1. Adding schedule entries should SUCCEED (Expansion/Stricter)
        let new_entry = create_entry_helper(&list_id, "14:00:00", "15:00:00", 1);
        assert!(
            manager
                .add_schedule_entries(&list_id, vec![new_entry])
                .is_ok(),
            "Adding schedule entries should be allowed when locked"
        );

        // 2. Deleting schedule entry should FAIL (Reduction)
        let entries = manager.get_schedule_entries_of(&list_id).unwrap().clone();
        let entry_id_to_delete = entries[0].get_id();
        let result = manager.delete_schedule_entry(entry_id_to_delete);
        assert!(
            matches!(
                result,
                Err(NetworkManagerOpErr::NetworkBlockIsLocked { .. })
            ),
            "Deleting schedule entry should fail when locked"
        );

        // 3. Editing while locked: the documented rule is that the proposed
        // schedule must cover everything the current one covers (keep-or-
        // extend). An identical edit is therefore ALLOWED; a weakening edit
        // (shrinking the window) is REJECTED.
        let entries = manager.get_schedule_entries_of(&list_id).unwrap().clone();
        let original = entries[0].clone();

        let identical = original.clone();
        assert!(
            manager.edit_schedule_entry(identical).is_ok(),
            "An edit that keeps full coverage is allowed while locked"
        );

        // Successful edits rewrite the schedule (and regenerate entry ids),
        // so re-fetch before each subsequent edit.
        let entries = manager.get_schedule_entries_of(&list_id).unwrap().clone();
        let morning = entries
            .iter()
            .find(|entry| entry.get_start() == chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap())
            .expect("the 10:00 entry must still exist")
            .clone();

        // Weaken: shrink 10:00–11:00 to 10:30–11:00.
        let weakened = ScheduleEntry::from(
            morning.get_id(),
            morning.get_parent_list_id(),
            chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            morning.get_end(),
            morning.get_weekday_set(),
        )
        .unwrap();
        let result = manager.edit_schedule_entry(weakened);
        assert!(
            matches!(
                result,
                Err(NetworkManagerOpErr::NetworkBlockIsLocked { .. })
            ),
            "An edit that reduces coverage must fail when locked"
        );

        // Strengthen: extend 10:00–11:00 to 09:00–12:00.
        let strengthened = ScheduleEntry::from(
            morning.get_id(),
            morning.get_parent_list_id(),
            chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            morning.get_weekday_set(),
        )
        .unwrap();
        assert!(
            manager.edit_schedule_entry(strengthened).is_ok(),
            "An edit that extends coverage is allowed while locked"
        );
    }

    fn create_entry_helper(
        list_id: &NetworkBlockId,
        start: &str,
        end: &str,
        bitmask: i32,
    ) -> ScheduleEntry {
        ScheduleEntry::from_raw(
            uuid::Uuid::now_v7().to_string(),
            (*list_id).to_string(),
            start,
            end,
            bitmask,
        )
        .unwrap()
    }
}
