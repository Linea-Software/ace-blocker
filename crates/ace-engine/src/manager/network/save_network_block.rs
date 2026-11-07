//! Transactional Network Block save. Validate first; publish live policy only
//! after the database has committed metadata, rules, schedules and connections.
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use super::{NetworkManager, error::NetworkManagerOpErr};
use crate::core::common::schedule::weekday_set_to_bitmask;
use crate::core::{
    NetworkBlock, NetworkBlockId, Schedule, ScheduleEntry, ScheduleEntryId, ScheduleMode, SitePack,
    SitePackId, UserDefinedRule, UserDefinedRules,
};
use crate::db::models::{
    NetworkBlockModel, NetworkBlockScheduleEntryModel, NetworkBlockToSitePackModel,
    UserDefinedRuleModel,
};

pub struct NetworkBlockDraft {
    pub id: Option<NetworkBlockId>,
    pub name: String,
    pub icon: Option<String>,
    pub safe_search: bool,
    pub rules: Vec<UserDefinedRule>,
    pub schedule: Vec<ScheduleEntry>,
    pub schedule_mode: ScheduleMode,
    pub site_pack_ids: Vec<SitePackId>,
}

impl NetworkManager {
    pub fn save_network_block(
        &mut self,
        draft: NetworkBlockDraft,
        is_pro_entitled: bool,
    ) -> Result<NetworkBlockId, NetworkManagerOpErr> {
        if draft.name.trim().is_empty() {
            return Err(NetworkManagerOpErr::InternalError {
                message: "Network Block name cannot be empty".into(),
            });
        }

        let is_new = draft.id.is_none();
        let id = draft
            .id
            .unwrap_or_else(|| NetworkBlockId::from(uuid::Uuid::now_v7()));
        let current = if is_new {
            None
        } else {
            Some(
                self.get_network_block_from(&id)
                    .ok_or_else(|| NetworkManagerOpErr::NetworkBlockNotFound { id: id.clone() })?,
            )
        };

        // Compile the complete matcher before any persistence.
        let rules = UserDefinedRules::new(draft.rules).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Invalid user-defined rules: {error}"),
            }
        })?;
        let timezone = current
            .map_or_else(crate::core::common::schedule::system_timezone, |block| {
                block.get_schedule().timezone()
            });
        let schedule = Schedule::from_in_zone(draft.schedule, timezone);
        if draft.schedule_mode == ScheduleMode::Scheduled && schedule.get_entries().is_empty() {
            return Err(NetworkManagerOpErr::InternalError {
                message: "Scheduled blocks require at least one time period".into(),
            });
        }
        let locked =
            current.is_some_and(|block| block.is_locked(&crate::trusted_time::locking_now()));

        if let Some(block) = current {
            if locked {
                if draft.name != block.get_name()
                    || draft.safe_search != block.is_safe_search_enabled()
                {
                    return Err(NetworkManagerOpErr::NetworkBlockIsLocked { id });
                }
                // Locked blocks can gain rules but not lose any old occurrence.
                let mut remaining = rules.rules().to_vec();
                for old in block.get_user_defined_rules().rules() {
                    let Some(index) = remaining.iter().position(|candidate| candidate == old)
                    else {
                        return Err(NetworkManagerOpErr::NetworkBlockIsLocked { id });
                    };
                    remaining.remove(index);
                }
                // Always covers every minute. A locked Always block cannot
                // be reduced to Scheduled; Scheduled -> Always is stronger.
                let allowed = match (block.schedule_mode(), draft.schedule_mode) {
                    (ScheduleMode::Always, ScheduleMode::Scheduled) => false,
                    (ScheduleMode::Always, ScheduleMode::Always) => true,
                    (ScheduleMode::Scheduled, ScheduleMode::Always) => true,
                    (ScheduleMode::Scheduled, ScheduleMode::Scheduled) => block
                        .get_schedule()
                        .is_strictly_more_restrictive_than(&schedule),
                };
                if !allowed {
                    return Err(NetworkManagerOpErr::NetworkBlockIsLocked { id });
                }
            }
        }

        let mut selected_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>> = HashMap::new();
        for pack_id in draft.site_pack_ids {
            let pack_arc = self.site_packs.get(&pack_id).ok_or_else(|| {
                NetworkManagerOpErr::SitePackNotFound {
                    id: pack_id.clone(),
                }
            })?;
            let was_connected =
                current.is_some_and(|block| block.get_site_packs().contains_key(&pack_id));
            let pack = pack_arc
                .read()
                .map_err(|_| NetworkManagerOpErr::RwLockPoison)?;
            if !was_connected {
                if !pack.is_synced() {
                    return Err(NetworkManagerOpErr::SitePackNotSynced { id: pack_id });
                }
                if (!pack.is_remote() || pack.requires_pro()) && !is_pro_entitled {
                    return Err(NetworkManagerOpErr::ProPlanRequired);
                }
            }
            drop(pack);
            selected_packs.insert(pack_id, pack_arc.clone());
        }

        if locked
            && current.is_some_and(|block| {
                block
                    .get_site_packs()
                    .keys()
                    .any(|pack_id| !selected_packs.contains_key(pack_id))
            })
        {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked { id });
        }

        // Keep stable row IDs for unchanged entries (especially important
        // when the editor sends an otherwise identical complete snapshot).
        let old_rules = if is_new {
            Vec::new()
        } else {
            self.db.get_user_defined_rules_of(&id)?
        };
        let mut claimed_rule_ids = HashSet::new();
        let rule_models = rules
            .rules()
            .iter()
            .map(|rule| {
                let matching = old_rules.iter().find(|old| {
                    old.rule == rule.rule
                        && old.rule_type == rule.rule_type.to_string()
                        && !claimed_rule_ids.contains(&old.id)
                });
                let record_id = matching
                    .map(|old| old.id.clone())
                    .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
                claimed_rule_ids.insert(record_id.clone());
                UserDefinedRuleModel {
                    id: record_id,
                    network_block_id: id.to_string(),
                    rule: rule.rule.clone(),
                    rule_type: rule.rule_type.to_string(),
                }
            })
            .collect::<Vec<_>>();

        let mut claimed_schedule_ids = HashSet::new();
        let old_schedule = current
            .map(|block| block.get_schedule().get_entries().as_slice())
            .unwrap_or(&[]);
        let mut committed_schedule = Vec::new();
        let mut schedule_models = Vec::new();
        for entry in schedule.get_entries() {
            let same = old_schedule.iter().find(|previous| {
                previous.get_start() == entry.get_start()
                    && previous.get_end() == entry.get_end()
                    && previous.get_weekday_set() == entry.get_weekday_set()
                    && !claimed_schedule_ids.contains(&previous.get_id())
            });
            let entry_id = same
                .map(|previous| previous.get_id())
                .unwrap_or_else(|| ScheduleEntryId::from(uuid::Uuid::now_v7()));
            claimed_schedule_ids.insert(entry_id.clone());
            let prepared = ScheduleEntry::from(
                entry_id,
                id.clone(),
                entry.get_start(),
                entry.get_end(),
                entry.get_weekday_set(),
            )
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: format!("Invalid schedule entry: {error}"),
            })?;
            schedule_models.push(NetworkBlockScheduleEntryModel {
                id: prepared.get_id().to_string(),
                network_block_id: id.to_string(),
                start_time: prepared.get_start().format("%H:%M:%S").to_string(),
                end_time: prepared.get_end().format("%H:%M:%S").to_string(),
                schedule_days_bitmask: weekday_set_to_bitmask(&prepared.get_weekday_set()),
            });
            committed_schedule.push(prepared);
        }

        let pack_models = selected_packs
            .keys()
            .map(|pack_id| NetworkBlockToSitePackModel {
                network_block_id: id.to_string(),
                site_pack_id: pack_id.to_string(),
            })
            .collect::<Vec<_>>();

        let original_enabled = current.map_or(true, NetworkBlock::is_enabled);
        let original_lock = current
            .map(|block| block.get_lock().clone())
            .unwrap_or_default();
        let original_exceptions = current
            .map(|block| block.get_exceptions().clone())
            .unwrap_or_default();
        let model = NetworkBlockModel {
            id: id.to_string(),
            name: draft.name.clone(),
            enabled: original_enabled,
            safe_search: draft.safe_search,
            icon: draft.icon.clone(),
            schedule_timezone: timezone.name().to_string(),
            schedule_mode: draft.schedule_mode.as_str().into(),
        };
        let prepared_block = NetworkBlock::from_with_icon(
            id.clone(),
            draft.name,
            original_enabled,
            Schedule::from_in_zone(committed_schedule, timezone),
            original_lock,
            selected_packs,
            rules,
            original_exceptions,
            draft.safe_search,
            draft.icon,
        );

        let mut prepared_block = prepared_block;
        prepared_block.set_schedule_mode(draft.schedule_mode);

        self.db.save_network_block_snapshot(
            &model,
            &rule_models,
            &schedule_models,
            &pack_models,
            is_new,
        )?;
        self.network_blocks.insert(id.clone(), prepared_block);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Lock, LockState, UserDefinedRuleType};
    use crate::db::EngineDB;
    use crate::manager::AccountManager;
    use crate::settings::Settings;
    use chrono::{Duration, Local, NaiveTime, WeekdaySet};
    use diesel::RunQueryDsl;
    use std::sync::Arc;

    fn context() -> (NetworkManager, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        Settings::set_test_sites_dir(Settings::shared_test_sites_dir());
        let db = Arc::new(EngineDB::from(&temp.path().join("atomic.sqlite")).unwrap());
        let account = AccountManager::from(db.clone()).unwrap();
        (NetworkManager::from(db, account).unwrap(), temp)
    }

    fn rule(text: &str) -> UserDefinedRule {
        UserDefinedRule::new(text.into(), UserDefinedRuleType::Domain)
    }

    fn draft(id: Option<NetworkBlockId>, name: &str) -> NetworkBlockDraft {
        NetworkBlockDraft {
            id,
            name: name.into(),
            icon: Some("shield".into()),
            safe_search: true,
            rules: vec![rule("new.example")],
            schedule_mode: ScheduleMode::Scheduled,
            schedule: vec![
                ScheduleEntry::from(
                    ScheduleEntryId::from(uuid::Uuid::now_v7()),
                    NetworkBlockId::from(uuid::Uuid::nil()),
                    NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                    WeekdaySet::ALL,
                )
                .unwrap(),
            ],
            site_pack_ids: vec![],
        }
    }

    #[test]
    fn creates_complete_block_and_reloads_same_policy() {
        let (mut manager, _temp) = context();
        let id = manager
            .save_network_block(draft(None, "New"), false)
            .unwrap();
        let current = manager.get_network_block_from(&id).unwrap();
        assert_eq!(current.get_name(), "New");
        assert_eq!(current.get_icon().as_deref(), Some("shield"));
        assert!(current.is_safe_search_enabled());
        assert_eq!(current.get_schedule().get_entries().len(), 1);
        assert_eq!(current.get_user_defined_rules().rules().len(), 1);

        let db = manager.db.clone();
        let account = AccountManager::from(db.clone()).unwrap();
        let reloaded = NetworkManager::from(db, account).unwrap();
        let persisted = reloaded.get_network_block_from(&id).unwrap();
        assert_eq!(persisted.get_name(), "New");
        assert!(persisted.is_safe_search_enabled());
        assert_eq!(persisted.get_schedule().get_entries().len(), 1);
        assert_eq!(persisted.get_user_defined_rules().rules().len(), 1);
    }

    #[test]
    fn failing_late_sql_statement_rolls_back_all_settings_and_live_policy() {
        let (mut manager, _temp) = context();
        let id = manager.create_network_block("Original".into()).unwrap();
        manager
            .add_user_defined_entries(&id, vec![rule("old.example")])
            .unwrap();

        let mut conn = manager.db.get_db_connection().unwrap();
        diesel::sql_query(
            "CREATE TRIGGER reject_schedule_insert BEFORE INSERT ON network_block_schedule_entries BEGIN SELECT RAISE(ABORT, 'test late failure'); END"
        ).execute(&mut conn).unwrap();

        let failed = manager.save_network_block(draft(Some(id.clone()), "Changed"), false);
        assert!(failed.is_err());

        let current = manager.get_network_block_from(&id).unwrap();
        assert_eq!(current.get_name(), "Original");
        assert_eq!(current.get_icon(), None);
        assert!(!current.is_safe_search_enabled());
        assert!(current.get_schedule().get_entries().is_empty());
        assert_eq!(
            current.get_user_defined_rules().rules(),
            &[rule("old.example")]
        );

        let db = manager.db.clone();
        let account = AccountManager::from(db.clone()).unwrap();
        let reloaded = NetworkManager::from(db, account).unwrap();
        let persisted = reloaded.get_network_block_from(&id).unwrap();
        assert_eq!(persisted.get_name(), "Original");
        assert_eq!(persisted.get_icon(), None);
        assert!(!persisted.is_safe_search_enabled());
        assert!(persisted.get_schedule().get_entries().is_empty());
        assert_eq!(
            persisted.get_user_defined_rules().rules(),
            &[rule("old.example")]
        );
    }

    #[test]
    fn locked_block_rejects_rule_removal_without_writing() {
        let (mut manager, _temp) = context();
        let id = manager.create_network_block("Protected".into()).unwrap();
        manager
            .add_user_defined_entries(&id, vec![rule("old.example")])
            .unwrap();
        manager
            .set_lock(
                &id,
                Lock::from(LockState::ActiveUntilLockEnd {
                    end: Local::now() + Duration::hours(1),
                }),
            )
            .unwrap();

        let result = manager.save_network_block(
            NetworkBlockDraft {
                id: Some(id.clone()),
                name: "Protected".into(),
                icon: None,
                safe_search: false,
                rules: Vec::new(),
                schedule: Vec::new(),
                schedule_mode: ScheduleMode::Always,
                site_pack_ids: Vec::new(),
            },
            false,
        );
        assert!(matches!(
            result,
            Err(NetworkManagerOpErr::NetworkBlockIsLocked { .. })
        ));
        assert_eq!(
            manager
                .get_network_block_from(&id)
                .unwrap()
                .get_user_defined_rules()
                .rules(),
            &[rule("old.example")]
        );
    }

    #[test]
    fn always_mode_enforces_without_schedule_and_survives_reload() {
        let (mut manager, _temp) = context();
        let id = manager.create_network_block("Always".into()).unwrap();
        let block = manager.get_network_block_from(&id).unwrap();
        assert_eq!(block.schedule_mode(), ScheduleMode::Always);
        assert!(block.get_schedule().get_entries().is_empty());
        assert!(block.is_active(&Local::now()));

        manager.set_network_block_enabled(&id, false).unwrap();
        assert!(
            !manager
                .get_network_block_from(&id)
                .unwrap()
                .is_active(&Local::now())
        );

        let db = manager.db.clone();
        let account = AccountManager::from(db.clone()).unwrap();
        let loaded = NetworkManager::from(db, account).unwrap();
        let block = loaded.get_network_block_from(&id).unwrap();
        assert_eq!(block.schedule_mode(), ScheduleMode::Always);
        assert!(!block.is_active(&Local::now()));
    }

    #[test]
    fn scheduled_mode_needs_a_period_and_rejects_partial_new_block() {
        let (mut manager, _temp) = context();
        let initial = manager.get_all_network_blocks().len();
        let mut empty = draft(None, "No period");
        empty.schedule_mode = ScheduleMode::Scheduled;
        empty.schedule.clear();
        assert!(manager.save_network_block(empty, false).is_err());
        assert_eq!(manager.get_all_network_blocks().len(), initial);

        let id = manager
            .save_network_block(draft(None, "Scheduled"), false)
            .unwrap();
        assert_eq!(
            manager.get_network_block_from(&id).unwrap().schedule_mode(),
            ScheduleMode::Scheduled
        );
        let db = manager.db.clone();
        let account = AccountManager::from(db.clone()).unwrap();
        let loaded = NetworkManager::from(db, account).unwrap();
        assert_eq!(
            loaded.get_network_block_from(&id).unwrap().schedule_mode(),
            ScheduleMode::Scheduled
        );
    }

    #[test]
    fn locked_always_cannot_be_weakened_to_scheduled() {
        let (mut manager, _temp) = context();
        let id = manager.create_network_block("Protected".into()).unwrap();
        manager
            .set_lock(
                &id,
                Lock::from(LockState::ActiveUntilLockEnd {
                    end: Local::now() + Duration::hours(1),
                }),
            )
            .unwrap();
        let mut change = draft(Some(id.clone()), "Protected");
        change.safe_search = false;
        change.icon = None;
        assert!(matches!(
            manager.save_network_block(change, false),
            Err(NetworkManagerOpErr::NetworkBlockIsLocked { .. })
        ));
        assert_eq!(
            manager.get_network_block_from(&id).unwrap().schedule_mode(),
            ScheduleMode::Always
        );
    }

    #[test]
    fn invalid_rule_or_missing_pack_cannot_create_a_partial_block() {
        let (mut manager, _temp) = context();
        let before = manager.get_all_network_blocks().len();
        let mut invalid = draft(None, "Invalid");
        invalid.rules = vec![UserDefinedRule::new("(".into(), UserDefinedRuleType::Regex)];
        assert!(manager.save_network_block(invalid, false).is_err());
        let mut missing = draft(None, "Missing pack");
        missing.site_pack_ids = vec![SitePackId::try_new("not-installed").unwrap()];
        assert!(manager.save_network_block(missing, false).is_err());
        assert_eq!(manager.get_all_network_blocks().len(), before);
    }
}
