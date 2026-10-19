use std::collections::HashMap;

use super::{NetworkManager, error::NetworkManagerOpErr};
use crate::core::{Lock, NetworkBlock, NetworkBlockId, RegexRules, Schedule, UserDefinedRules};

impl NetworkManager {
    pub fn get_network_block_from(
        &self,
        network_block_id: &NetworkBlockId,
    ) -> Option<&NetworkBlock> {
        let list = self.network_blocks.get(network_block_id);
        if list.is_some() {
            return list;
        }

        None
    }

    pub fn get_mut_network_block_from(
        &mut self,
        network_block_id: &NetworkBlockId,
    ) -> Option<&mut NetworkBlock> {
        let list = self.network_blocks.get_mut(network_block_id);
        if list.is_some() {
            return list;
        }
        None
    }

    pub fn get_all_network_blocks(&self) -> Vec<&NetworkBlock> {
        self.network_blocks.values().collect()
    }

    pub fn create_network_block(
        &mut self,
        network_block_name: String,
    ) -> Result<NetworkBlockId, NetworkManagerOpErr> {
        let network_block_model = self
            .db
            .add_network_block(network_block_name.clone())
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to create net list: {}", error),
            })?;

        let block_uuid = uuid::Uuid::parse_str(&network_block_model.id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Invalid UUID in DB for network block: {}", error),
            }
        })?;
        let net_list_id = NetworkBlockId::from(block_uuid);

        let net_list = NetworkBlock::from(
            net_list_id.clone(),
            network_block_name,
            true,
            Schedule::default(),
            Lock::default(),
            HashMap::new(), // equals ::new()
            UserDefinedRules::default(),
            RegexRules::default(),
            false,
        );

        self.network_blocks.insert(net_list_id.clone(), net_list);

        Ok(net_list_id)
    }

    /// Toggles enforcement independently of schedule membership.
    /// Disabling an actively locked block is forbidden; enabling it is allowed.
    pub fn set_network_block_enabled(
        &mut self,
        network_block_id: &NetworkBlockId,
        enabled: bool,
    ) -> Result<(), NetworkManagerOpErr> {
        let now = &chrono::Local::now();

        let network_block = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        if network_block.is_enabled() && !enabled && network_block.is_locked(now) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }

        self.db
            .set_network_block_enabled(network_block_id, enabled)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .unwrap()
            .set_enabled(enabled);
        Ok(())
    }

    pub fn set_network_block_icon(
        &mut self,
        network_block_id: &NetworkBlockId,
        icon: Option<String>,
    ) -> Result<(), NetworkManagerOpErr> {
        if self.get_network_block_from(network_block_id).is_none() {
            return Err(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            });
        }
        self.db
            .set_network_block_icon(network_block_id, icon.clone())
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to set icon: {}", error),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .unwrap()
            .set_icon(icon);
        Ok(())
    }

    /// Refuses removal while the block's lock is active, regardless of whether
    /// the block's enabled switch is on.
    pub fn delete_network_block(
        &mut self,
        network_block_id: &NetworkBlockId,
    ) -> Result<(), NetworkManagerOpErr> {
        let now = &chrono::Local::now();

        if let Some(block) = self.network_blocks.get(network_block_id) {
            if block.is_locked(now) {
                return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                    id: network_block_id.clone(),
                });
            }
        } else {
            return Err(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            });
        }

        self.db
            .delete_network_block(network_block_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?;

        self.network_blocks.remove(network_block_id);
        Ok(())
    }

    pub fn is_network_block_locked(&self, net_list_id: &NetworkBlockId) -> bool {
        let now = &chrono::Local::now();

        if let Some(block) = self.network_blocks.get(net_list_id) {
            return block.is_locked(now);
        }

        false
    }

    /// Returns true if *any* network block currently has an active lock.
    pub fn any_network_block_locked(&self) -> bool {
        let now = &chrono::Local::now();
        self.network_blocks
            .values()
            .any(|block| block.is_locked(now))
    }

    pub fn set_network_block_safe_search(
        &mut self,
        network_block_id: &NetworkBlockId,
        enabled: bool,
    ) -> Result<(), NetworkManagerOpErr> {
        let now = &chrono::Local::now();

        if let Some(block) = self.network_blocks.get(network_block_id) {
            if block.is_locked(now) {
                return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                    id: network_block_id.clone(),
                });
            }
        } else {
            return Err(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            });
        }

        self.db
            .set_safe_search(network_block_id, enabled)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .unwrap()
            .set_safe_search(enabled);
        Ok(())
    }

    pub fn rename_network_block(
        &mut self,
        network_block_id: &NetworkBlockId,
        new_name: String,
    ) -> Result<(), NetworkManagerOpErr> {
        let now = &chrono::Local::now();

        if let Some(block) = self.network_blocks.get(network_block_id) {
            if block.is_locked(now) {
                return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                    id: network_block_id.clone(),
                });
            }
        } else {
            return Err(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            });
        }

        self.db
            .rename_network_block(network_block_id, new_name.clone())
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .unwrap()
            .set_name(new_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::LockState;
    use crate::db::EngineDB;
    use crate::settings::Settings;
    use chrono::Local;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct TestContext {
        manager: NetworkManager,
        _temp_dir: tempfile::TempDir,
    }

    fn setup_test_manager() -> TestContext {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");
        let sites_dir = Settings::shared_test_sites_dir();
        // Configure Settings to use our temp dir
        Settings::set_test_sites_dir(sites_dir.clone());

        let db = EngineDB::from(&db_path).unwrap();
        let db_lock = Arc::new(db);
        let account_manager = crate::manager::AccountManager::from(db_lock.clone()).unwrap();
        let manager = NetworkManager::from(db_lock, account_manager).unwrap();

        TestContext {
            manager,
            _temp_dir: temp_dir,
        }
    }

    #[test]
    fn failed_sqlite_mutations_do_not_change_live_policy() {
        use diesel::RunQueryDsl;
        let mut ctx = setup_test_manager();
        let id = ctx
            .manager
            .create_network_block("Unchanged".into())
            .unwrap();
        let mut conn = ctx.manager.db.get_db_connection().unwrap();
        diesel::sql_query(
            "CREATE TRIGGER reject_policy_updates BEFORE UPDATE ON network_blocks BEGIN SELECT RAISE(ABORT, 'test rejection'); END"
        ).execute(&mut conn).unwrap();

        assert!(ctx.manager.set_network_block_enabled(&id, false).is_err());
        assert!(
            ctx.manager
                .get_network_block_from(&id)
                .unwrap()
                .is_enabled()
        );
        assert!(
            ctx.manager
                .set_network_block_safe_search(&id, true)
                .is_err()
        );
        assert!(
            !ctx.manager
                .get_network_block_from(&id)
                .unwrap()
                .is_safe_search_enabled()
        );
        assert!(
            ctx.manager
                .rename_network_block(&id, "Renamed".into())
                .is_err()
        );
        assert_eq!(
            ctx.manager.get_network_block_from(&id).unwrap().get_name(),
            "Unchanged"
        );

        diesel::sql_query("DROP TRIGGER reject_policy_updates")
            .execute(&mut conn)
            .unwrap();
        diesel::sql_query(
            "CREATE TRIGGER reject_policy_deletes BEFORE DELETE ON network_blocks BEGIN SELECT RAISE(ABORT, 'test rejection'); END"
        ).execute(&mut conn).unwrap();
        assert!(ctx.manager.delete_network_block(&id).is_err());
        assert!(ctx.manager.get_network_block_from(&id).is_some());
    }

    #[test]
    fn test_create_net_list() {
        let mut ctx = setup_test_manager();

        // Migration 0021 seeds default blocks, so assertions must be
        // relative to the seeded baseline, not absolute counts.
        let baseline_count = ctx.manager.get_all_network_blocks().len();

        let list_id = ctx
            .manager
            .create_network_block("Test Block List".into())
            .unwrap();

        // Verify it's in the manager's map
        assert!(ctx.manager.network_blocks.contains_key(&list_id));

        // Verify DB persistence
        let all_lists = ctx.manager.get_all_network_blocks();
        assert_eq!(all_lists.len(), baseline_count + 1);
        assert!(all_lists.iter().any(|list| list.get_id() == list_id));
    }

    #[test]
    fn test_delete_net_list() {
        let mut ctx = setup_test_manager();

        // Migration 0021 seeds default blocks, so assertions must be
        // relative to the seeded baseline, not absolute counts.
        let baseline_count = ctx.manager.get_all_network_blocks().len();

        let list_id = ctx
            .manager
            .create_network_block("Delete Me".into())
            .unwrap();

        ctx.manager.delete_network_block(&list_id).unwrap();

        assert!(!ctx.manager.network_blocks.contains_key(&list_id));

        let all_lists = ctx.manager.get_all_network_blocks();
        assert_eq!(all_lists.len(), baseline_count);
        assert!(!all_lists.iter().any(|list| list.get_id() == list_id));
    }

    #[test]
    fn test_delete_locked_net_list() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Locked List".into())
            .unwrap();

        // Lock the list
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock).unwrap();

        assert!(ctx.manager.is_network_block_locked(&list_id));

        let result = ctx.manager.delete_network_block(&list_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_enabled_state_persists_across_manager_reload() {
        let mut ctx = setup_test_manager();
        let list_id = ctx
            .manager
            .create_network_block("Enabled Persistence".into())
            .unwrap();

        assert!(
            ctx.manager
                .get_network_block_from(&list_id)
                .unwrap()
                .is_enabled()
        );

        ctx.manager
            .set_network_block_enabled(&list_id, false)
            .unwrap();

        let db = ctx.manager.db.clone();
        let account_manager = crate::manager::AccountManager::from(db.clone()).unwrap();
        let reloaded = NetworkManager::from(db, account_manager).unwrap();

        assert!(
            !reloaded
                .get_network_block_from(&list_id)
                .unwrap()
                .is_enabled()
        );
    }

    #[test]
    fn test_effective_activity_requires_enabled_and_active_schedule() {
        let mut ctx = setup_test_manager();
        let list_id = ctx
            .manager
            .create_network_block("Effective Activity".into())
            .unwrap();

        let schedule_entry = crate::core::ScheduleEntry::from(
            crate::core::ScheduleEntryId::from(uuid::Uuid::now_v7()),
            list_id.clone(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            chrono::WeekdaySet::ALL,
        )
        .unwrap();

        ctx.manager
            .add_schedule_entries(&list_id, vec![schedule_entry])
            .unwrap();

        let now = Local::now();
        assert!(
            ctx.manager
                .get_network_block_from(&list_id)
                .unwrap()
                .is_active(&now)
        );

        ctx.manager
            .set_network_block_enabled(&list_id, false)
            .unwrap();

        assert!(
            !ctx.manager
                .get_network_block_from(&list_id)
                .unwrap()
                .is_active(&now)
        );
    }

    #[test]
    fn test_disabling_locked_block_is_rejected_but_enabling_is_allowed() {
        let mut ctx = setup_test_manager();
        let list_id = ctx
            .manager
            .create_network_block("Enabled Lock Rules".into())
            .unwrap();

        ctx.manager
            .set_network_block_enabled(&list_id, false)
            .unwrap();

        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock).unwrap();

        ctx.manager
            .set_network_block_enabled(&list_id, true)
            .expect("enabling is a strengthening operation");

        let result = ctx.manager.set_network_block_enabled(&list_id, false);
        assert!(matches!(
            result,
            Err(NetworkManagerOpErr::NetworkBlockIsLocked { .. })
        ));
        assert!(
            ctx.manager
                .get_network_block_from(&list_id)
                .unwrap()
                .is_enabled()
        );
    }
}
