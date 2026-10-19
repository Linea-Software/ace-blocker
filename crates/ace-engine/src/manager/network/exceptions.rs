use crate::core::NetworkBlockId;
use crate::db::models::ExceptionModel;

use super::{NetworkManager, error::NetworkManagerOpErr};

impl NetworkManager {
    pub fn get_exceptions_of(
        &mut self,
        network_block_id: &NetworkBlockId,
    ) -> Result<Vec<String>, NetworkManagerOpErr> {
        let list = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        Ok(list.get_exceptions().patterns().to_vec())
    }

    pub fn add_exceptions(
        &mut self,
        network_block_id: &NetworkBlockId,
        entries: Vec<String>,
    ) -> Result<Vec<ExceptionModel>, NetworkManagerOpErr> {
        let list = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;
        if list.is_locked(&chrono::Local::now()) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }
        let mut prepared = list.get_exceptions().clone();
        prepared.add_patterns(entries.clone()).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("invalid exception rule: {error}"),
            }
        })?;

        let results = self
            .db
            .add_exceptions(network_block_id, &entries)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to add exceptions for list {}: {}",
                    network_block_id, error
                ),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .expect("block cannot disappear under the manager write lock")
            .replace_exceptions(prepared);
        Ok(results)
    }

    pub fn delete_exception(
        &mut self,
        exception_id: uuid::Uuid,
    ) -> Result<(), NetworkManagerOpErr> {
        let block_model = self
            .db
            .get_network_block_of_exception(&exception_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to find network block for exception {}: {}",
                    exception_id, error
                ),
            })?;

        let block_uuid = uuid::Uuid::parse_str(&block_model.id).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: format!("Invalid UUID in DB for network block: {}", error),
            }
        })?;
        let net_list_id = NetworkBlockId::from(block_uuid);
        let now = &chrono::Local::now();

        // Check memory first
        let content_list = self.get_network_block_from(&net_list_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: net_list_id.clone(),
            },
        )?;

        if content_list.is_locked(now) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: net_list_id.clone(),
            });
        }

        // Delete from DB
        let pattern = self.db.delete_exception(&exception_id).map_err(|error| {
            NetworkManagerOpErr::DatabaseError {
                message: format!("failed to delete exception {}: {}", exception_id, error),
            }
        })?;

        // Delete from memory
        if let Some(content_list) = self.get_mut_network_block_from(&net_list_id) {
            content_list.remove_exception(&pattern).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!(
                        "failed to remove exception for list {}: {}",
                        net_list_id, error
                    ),
                }
            })?;
        } else {
            return Err(NetworkManagerOpErr::InternalError {
                message: format!(
                    "Content list with id {} not found in memory, but exists in db. This indicates a synchronization issue.",
                    net_list_id
                ),
            });
        }

        Ok(())
    }

    pub(crate) fn load_exceptions_of(
        db: &crate::db::EngineDB,
        network_block_id: &uuid::Uuid,
    ) -> Result<crate::core::RegexRules, NetworkManagerOpErr> {
        let models = db.get_exceptions_of(network_block_id).map_err(|error| {
            NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            }
        })?;

        let patterns = models.into_iter().map(|m| m.rule).collect();
        let rules = crate::core::RegexRules::new(patterns).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: error.to_string(),
            }
        })?;

        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::EngineDB, settings::Settings};
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
    fn invalid_exception_does_not_reach_sqlite() {
        let mut ctx = setup_test_manager();
        let id = ctx
            .manager
            .create_network_block("Invalid Exception".into())
            .unwrap();
        assert!(ctx.manager.add_exceptions(&id, vec!["(".into()]).is_err());
        assert!(ctx.manager.get_exceptions_of(&id).unwrap().is_empty());
        assert!(
            ctx.manager
                .db
                .get_exceptions_of(id.as_ref())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_add_delete_exceptions() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Exceptions List".into())
            .unwrap();

        let entries = vec!["example.com".to_string()];
        let added = ctx.manager.add_exceptions(&list_id, entries).unwrap();

        let saved = ctx.manager.get_exceptions_of(&list_id).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0], "example.com");

        // Delete
        let ex_id = uuid::Uuid::parse_str(&added[0].id).unwrap();
        ctx.manager.delete_exception(ex_id).unwrap();

        let saved = ctx.manager.get_exceptions_of(&list_id).unwrap();
        assert!(saved.is_empty());
    }
}
