use crate::core::{NetworkBlockId, UserDefinedRule, UserDefinedRuleType, UserDefinedRules};
use crate::db::models::UserDefinedRuleModel;

use super::{NetworkManager, error::NetworkManagerOpErr};

impl NetworkManager {
    pub fn get_user_defined_entries_of(
        &mut self,
        network_block_id: &NetworkBlockId,
    ) -> Result<Vec<UserDefinedRuleModel>, NetworkManagerOpErr> {
        let rules = self
            .db
            .get_user_defined_rules_of(network_block_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?;

        Ok(rules)
    }

    pub fn add_user_defined_entries(
        &mut self,
        net_list_id: &NetworkBlockId,
        entries: Vec<UserDefinedRule>,
    ) -> Result<Vec<UserDefinedRuleModel>, NetworkManagerOpErr> {
        // Build the complete replacement matcher before writing to SQLite.
        // A batch may be individually valid while exceeding the combined
        // compiled-regex limit of the existing policy.
        let list = self.get_network_block_from(net_list_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: net_list_id.clone(),
            },
        )?;
        let existing_len = list.get_user_defined_rules().rules().len();
        let mut prepared = list.get_user_defined_rules().clone();
        prepared
            .add_rules(entries)
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: format!("invalid user defined rule: {error}"),
            })?;
        let normalized_entries = prepared.rules()[existing_len..].to_vec();

        let results = self
            .db
            .add_user_defined_rules(net_list_id, &normalized_entries)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to add user defined entries for list {}: {}",
                    net_list_id, error
                ),
            })?;

        // The matcher is already compiled and cannot fail after persistence.
        self.get_mut_network_block_from(net_list_id)
            .expect("block cannot disappear under the manager write lock")
            .replace_user_defined_rules(prepared);
        Ok(results)
    }

    pub fn delete_user_defined_entry(
        &mut self,
        rule_id: uuid::Uuid,
    ) -> Result<(), NetworkManagerOpErr> {
        let block_model = self
            .db
            .get_network_block_of_user_defined_rule(&rule_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to find network block for rule {}: {}",
                    rule_id, error
                ),
            })?;

        let block_uuid = uuid::Uuid::parse_str(&block_model.id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!(
                    "Invalid network-block UUID '{}' for user-defined rule: {error}",
                    block_model.id
                ),
            }
        })?;
        let network_block_id = NetworkBlockId::from(block_uuid);
        let now = &crate::trusted_time::locking_now();

        // Check memory first
        let network_block = self.get_network_block_from(&network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        if network_block.is_locked(now) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }

        // Delete from DB
        let rule_model = self
            .db
            .delete_user_defined_rule(&rule_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to delete user defined entry {}: {}", rule_id, error),
            })?;
        let rule_type = rule_model
            .rule_type
            .parse::<UserDefinedRuleType>()
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: error.to_string(),
            })?;
        let rule = UserDefinedRule::new(rule_model.rule, rule_type);

        let network_block = self.get_mut_network_block_from(&network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        network_block
            .remove_user_defined_entry(&rule)
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: format!(
                    "failed to remove user defined entry for list {}: {}",
                    network_block_id, error
                ),
            })?;

        Ok(())
    }

    pub(crate) fn load_user_defined_rules_of(
        db: &crate::db::EngineDB,
        network_block_id: &uuid::Uuid,
    ) -> Result<UserDefinedRules, NetworkManagerOpErr> {
        let rules = db
            .get_user_defined_rules_of(network_block_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            })?
            .into_iter()
            .map(|model| {
                let rule_type =
                    model
                        .rule_type
                        .parse::<UserDefinedRuleType>()
                        .map_err(|error| NetworkManagerOpErr::InternalError {
                            message: error.to_string(),
                        })?;
                Ok(UserDefinedRule::new(model.rule, rule_type))
            })
            .collect::<Result<Vec<_>, NetworkManagerOpErr>>()?;

        UserDefinedRules::new(rules).map_err(|error| NetworkManagerOpErr::InternalError {
            message: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Lock, LockState, ScheduleEntry, ScheduleEntryId};
    use crate::db::EngineDB;
    use crate::settings::Settings;
    use chrono::Local;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct TestContext {
        manager: NetworkManager,
        _temp_dir: tempfile::TempDir,
    }

    fn domain_rule(rule: &str) -> UserDefinedRule {
        UserDefinedRule::new(rule.to_string(), UserDefinedRuleType::Domain)
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
    #[tokio::test]
    async fn test_add_user_defined_rules() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("User Def List".into())
            .unwrap();

        let entries = vec![domain_rule("example.com"), domain_rule("test.org")];
        let _added = ctx
            .manager
            .add_user_defined_entries(&list_id, entries.clone())
            .unwrap();

        // Verify in Memory (via new API)
        let saved = ctx.manager.get_user_defined_entries_of(&list_id).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(saved.iter().any(|error| error.rule == "example.com"));
        assert!(saved.iter().any(|error| error.rule == "test.org"));

        let uri = "http://example.com".parse::<http::Uri>().unwrap();

        let schedule_entries = vec![
            ScheduleEntry::from(
                ScheduleEntryId::from(uuid::Uuid::nil()),
                list_id.clone(),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(), // midnight end = full day
                chrono::WeekdaySet::ALL,
            )
            .unwrap(),
        ];

        ctx.manager
            .add_schedule_entries(&list_id, schedule_entries)
            .unwrap();

        assert!(!matches!(
            ctx.manager.is_uri_blocked(&uri),
            crate::core::BlockingResult::NotBlocked
        ));
    }

    #[test]
    fn test_delete_user_defined_rule() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Delete Entry List".into())
            .unwrap();

        let entries = vec![domain_rule("delete.me")];
        let added = ctx
            .manager
            .add_user_defined_entries(&list_id, entries)
            .unwrap();

        let rule_id = added[0].id.parse().unwrap_or_default();

        ctx.manager.delete_user_defined_entry(rule_id).unwrap();

        let saved = ctx.manager.get_user_defined_entries_of(&list_id).unwrap();
        assert!(saved.is_empty());
    }

    #[test]
    fn test_delete_user_defined_rule_locked() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Locked Entry List".into())
            .unwrap();

        let entries = vec![domain_rule("locked.me")];
        let added = ctx
            .manager
            .add_user_defined_entries(&list_id, entries)
            .unwrap();

        let rule_id = added[0].id.parse().unwrap_or_default();

        // Lock the list
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock).unwrap();

        let result = ctx.manager.delete_user_defined_entry(rule_id);

        match result {
            Err(NetworkManagerOpErr::NetworkBlockIsLocked { id }) => assert_eq!(id, list_id),
            _ => panic!("Expected ListIsLocked error. Result was: {:?}", result),
        }
    }

    #[test]
    fn test_add_user_defined_rule_locked() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Locked List For Add".into())
            .unwrap();

        // Lock the list
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock).unwrap();

        // Adding rules should SUCCEED even when locked (it does not weaken the block)
        let entries = vec![domain_rule("new.rule")];
        let result = ctx.manager.add_user_defined_entries(&list_id, entries);

        assert!(result.is_ok());
        let saved = ctx.manager.get_user_defined_entries_of(&list_id).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].rule, "new.rule");
    }
}
