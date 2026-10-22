use super::{NetworkManager, error::NetworkManagerOpErr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupSitePackSource {
    pub source: String,
    pub source_type: String,
    pub format: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupSitePack {
    /// The pack's local row id. For built-in/default and updater-registered
    /// remote packs this can be the canonical catalog id (`pack-adult-free`,
    /// ...) — never assume a UUID — or a UUIDv7 for engine-created rows.
    pub id: String,
    #[serde(default)]
    pub remote_id: Option<String>,
    #[serde(default)]
    pub requires_pro: bool,
    #[serde(default)]
    pub version: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sources: Vec<BackupSitePackSource>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupScheduleEntry {
    pub start_time: String,
    pub end_time: String,
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupNetworkBlock {
    pub id: uuid::Uuid,
    pub name: String,
    pub enabled: bool,
    pub safe_search: bool,
    pub icon: Option<String>,
    pub schedules: Vec<BackupScheduleEntry>,
    pub user_rules: Vec<crate::core::UserDefinedRule>,
    pub connected_site_packs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EngineBackup {
    pub version: String,
    pub custom_site_packs: Vec<BackupSitePack>,
    pub network_blocks: Vec<BackupNetworkBlock>,
    // the scripts ids are normal strings
    pub deactivated_resistance_scripts: Vec<String>,
}

use chrono::Weekday;

impl NetworkManager {
    pub fn export_backup(&mut self) -> Result<EngineBackup, NetworkManagerOpErr> {
        let mut backup_custom_site_packs: Vec<BackupSitePack> = Vec::new();

        /////////////////
        // SITE PACKS //
        /////////////////
        for site_pack in self.site_packs.values() {
            let site_pack =
                site_pack
                    .read()
                    .map_err(|error| NetworkManagerOpErr::InternalError {
                        message: error.to_string(),
                    })?;

            let site_pack_backup = BackupSitePack {
                id: site_pack.get_id().to_string(),
                // Preserve remote linkage and tier so a restore re-creates a
                // remote row (matched by the updater) instead of demoting it
                // to a local pack.
                remote_id: site_pack.get_remote_id().map(|v| v.to_string()),
                requires_pro: site_pack.requires_pro(),
                version: site_pack.get_version().map(|v| v.to_string()),
                name: site_pack.get_name().to_string(),
                description: site_pack.get_description().map(|value| value.to_string()),
                icon: site_pack.get_icon().map(|value| value.to_string()),
                sources: site_pack
                    .get_sources()
                    .iter()
                    .map(|source| BackupSitePackSource {
                        source: source.source.clone(),
                        source_type: source.source_type.to_string(),
                        format: source.format.to_string(),
                    })
                    .collect(),
            };
            backup_custom_site_packs.push(site_pack_backup);
        }

        ////////////////////
        // NETWORK BLOCKS //
        ////////////////////
        let mut backup_network_blocks: Vec<BackupNetworkBlock> = Vec::new();

        for network_block in self.network_blocks.values() {
            let schedules: Vec<_> = network_block
                .get_schedule()
                .get_entries()
                .iter()
                .map(|sched_entry| BackupScheduleEntry {
                    start_time: sched_entry.get_start().to_string(),
                    end_time: sched_entry.get_end().to_string(),
                    monday: sched_entry.has_weekday(Weekday::Mon),
                    tuesday: sched_entry.has_weekday(Weekday::Tue),
                    wednesday: sched_entry.has_weekday(Weekday::Wed),
                    thursday: sched_entry.has_weekday(Weekday::Thu),
                    friday: sched_entry.has_weekday(Weekday::Fri),
                    saturday: sched_entry.has_weekday(Weekday::Sat),
                    sunday: sched_entry.has_weekday(Weekday::Sun),
                })
                .collect();

            let net_block_backup = BackupNetworkBlock {
                id: *network_block.get_id(),
                name: network_block.get_name(),
                enabled: network_block.is_enabled(),
                safe_search: network_block.is_safe_search_enabled(),
                icon: network_block.get_icon(),
                schedules,
                user_rules: network_block.get_user_defined_rules().rules().to_vec(),
                connected_site_packs: network_block
                    .get_site_packs()
                    .keys()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>(),
            };
            backup_network_blocks.push(net_block_backup);
        }

        let deactivated_resistance_scripts: Vec<_> = self
            .db
            .get_all_script_states()?
            .into_iter()
            .filter(|state| !state.enabled)
            .map(|state| state.script_id)
            .collect();

        Ok(EngineBackup {
            version: "1.0".to_string(),
            custom_site_packs: backup_custom_site_packs,
            network_blocks: backup_network_blocks,
            deactivated_resistance_scripts,
        })
    }

    /// Restores each eligible resource atomically. If a later unit fails, prior
    /// commits remain and the live manager is reloaded before returning Err.
    pub async fn import_backup(&mut self, backup: EngineBackup) -> Result<(), NetworkManagerOpErr> {
        if backup.version != "1.0" {
            return Err(NetworkManagerOpErr::InvalidBackupVersion {
                message: "expected version 1.0".to_string(),
            });
        }

        let result = self.persist_backup_units(backup);
        // The database may contain successfully restored units even if a later
        // unit failed. Always reconcile live policy before leaving this method.
        let db = self.get_db();
        let reload = (|| {
            let site_packs = Self::load_site_packs(&db)?;
            let network_blocks = Self::load_net_lists(&db, &site_packs)?;
            let script_tree = Self::try_load_resistance_mode(&db)?;
            self.site_packs = site_packs;
            self.network_blocks = network_blocks;
            self.resistance_mode.script_tree = script_tree;
            Ok::<(), NetworkManagerOpErr>(())
        })();
        match (result, reload) {
            (Err(error), Err(reload_error)) => {
                tracing::error!(%reload_error, "Backup failed and live policy reload also failed");
                Err(error)
            }
            (Err(error), _) => Err(error),
            (Ok(()), reload) => reload,
        }
    }

    fn persist_backup_units(&mut self, backup: EngineBackup) -> Result<(), NetworkManagerOpErr> {
        use crate::core::common::injectable_scripts::ScriptId;
        use crate::core::common::schedule::ScheduleEntry;
        use crate::core::importer::{SitePackFormat, SitePackSourceType};
        use crate::core::{
            NetworkBlockId, RemoteSitePackId, SitePackId, SitePackVersion, UserDefinedRules,
        };
        use crate::db::backup_restore_repo::RestoredNetworkBlock;
        use crate::db::models::{
            NetworkBlockScheduleEntryModel, SitePackModel, SitePackSourceModel,
        };

        for pack in backup.custom_site_packs {
            let id = SitePackId::try_new(pack.id).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("Invalid site-pack ID in backup: {error}"),
                }
            })?;
            if self.db.site_pack_exists(&id)? {
                continue;
            }

            let remote_id = pack
                .remote_id
                .map(RemoteSitePackId::try_new)
                .transpose()
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("Invalid remote site-pack ID in backup: {error}"),
                })?;
            let version = pack
                .version
                .map(SitePackVersion::try_new)
                .transpose()
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("Invalid site-pack version in backup: {error}"),
                })?;
            let source_models = pack
                .sources
                .into_iter()
                .map(|source| {
                    let source_type = SitePackSourceType::try_from(source.source_type.as_str())
                        .map_err(|error| NetworkManagerOpErr::InternalError {
                            message: format!("Invalid site-pack source type in backup: {error}"),
                        })?;
                    let format =
                        SitePackFormat::try_from(source.format.as_str()).map_err(|error| {
                            NetworkManagerOpErr::InternalError {
                                message: format!(
                                    "Invalid site-pack source format in backup: {error}"
                                ),
                            }
                        })?;
                    Ok(SitePackSourceModel {
                        id: uuid::Uuid::now_v7().to_string(),
                        site_pack_id: id.to_string(),
                        source: source.source,
                        source_type: source_type.to_string(),
                        format: format.to_string(),
                    })
                })
                .collect::<Result<Vec<_>, NetworkManagerOpErr>>()?;
            let model = SitePackModel {
                id: id.to_string(),
                remote_id: remote_id.map(|value| value.to_string()),
                name: pack.name,
                description: pack.description,
                requires_pro: pack.requires_pro,
                last_updated_at: None,
                version: version.map(|value| value.to_string()),
                is_ignored: false,
                icon: pack.icon,
            };
            self.db.restore_missing_site_pack(&model, &source_models)?;
        }

        for block in backup.network_blocks {
            let id = block.id;
            if self.is_network_block_locked(&NetworkBlockId::from(id)) {
                tracing::info!(%id, "Skipping locked network block during backup restore");
                continue;
            }

            let schedules = block
                .schedules
                .into_iter()
                .map(|schedule| {
                    let mut days = 0;
                    for (weekday, present) in [
                        schedule.monday,
                        schedule.tuesday,
                        schedule.wednesday,
                        schedule.thursday,
                        schedule.friday,
                        schedule.saturday,
                        schedule.sunday,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if present {
                            days |= 1 << weekday;
                        }
                    }
                    let row = NetworkBlockScheduleEntryModel {
                        id: uuid::Uuid::now_v7().to_string(),
                        network_block_id: id.to_string(),
                        start_time: schedule.start_time,
                        end_time: schedule.end_time,
                        schedule_days_bitmask: days,
                    };
                    ScheduleEntry::from_raw(
                        row.id.clone(),
                        row.network_block_id.clone(),
                        &row.start_time,
                        &row.end_time,
                        row.schedule_days_bitmask,
                    )
                    .map_err(|error| NetworkManagerOpErr::InternalError {
                        message: format!("Invalid schedule in backup for {id}: {error}"),
                    })?;
                    Ok(row)
                })
                .collect::<Result<Vec<_>, NetworkManagerOpErr>>()?;
            let normalized_rules = UserDefinedRules::new(block.user_rules).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("Invalid user rule in backup for {id}: {error}"),
                }
            })?;
            let site_pack_ids = block
                .connected_site_packs
                .into_iter()
                .map(|id| {
                    SitePackId::try_new(id).map_err(|error| NetworkManagerOpErr::InternalError {
                        message: format!("Invalid site-pack reference in backup: {error}"),
                    })
                })
                .collect::<Result<Vec<_>, NetworkManagerOpErr>>()?;

            self.db.restore_network_block(RestoredNetworkBlock {
                id,
                name: &block.name,
                enabled: block.enabled,
                safe_search: block.safe_search,
                icon: block.icon.as_deref(),
                schedules: &schedules,
                user_rules: normalized_rules.rules(),
                site_pack_ids: &site_pack_ids,
            })?;
        }

        // Script state writes do not weaken locked policies: the same lock
        // checks remain authoritative for changes during normal operation.
        for script_id in backup.deactivated_resistance_scripts {
            let script_id = ScriptId::try_new(script_id).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!("Invalid script ID in backup: {error}"),
                }
            })?;
            self.db.set_script_state(script_id.as_ref(), false)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::EngineDB;
    use crate::settings::Settings;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// One process-wide temp dir for test site packs (TESTINFRA-01).
    fn set_shared_sites_dir() {
        Settings::set_test_sites_dir(Settings::shared_test_sites_dir());
    }

    /// One process-wide leaked resistance-scripts dir containing an (empty)
    /// catalog, shared by all tests in this module. The Settings override is
    /// process-global and tests run in parallel, so a per-test dir would race
    /// (TESTINFRA-01); import_backup reloads the script tree from this path.
    fn shared_resistance_dir() -> PathBuf {
        static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        DIR.get_or_init(|| {
            let dir = tempfile::tempdir().expect("failed to create shared resistance dir");
            let path = dir.path().to_path_buf();
            std::fs::write(path.join("scripts-data.json"), r#"{"sites":{}}"#).unwrap();
            std::mem::forget(dir); // keep alive for the whole test process
            path
        })
        .clone()
    }

    fn setup_manager() -> (NetworkManager, tempfile::TempDir) {
        set_shared_sites_dir();
        Settings::set_test_resistance_scripts_dir(shared_resistance_dir());
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");
        let db = Arc::new(EngineDB::from(&db_path).unwrap());
        let account_manager = crate::manager::AccountManager::from(db.clone()).unwrap();
        let mut manager = NetworkManager::from(db, account_manager).unwrap();
        manager.set_debug_pro_policy(crate::manager::network::DebugProPolicy::Pro);
        (manager, temp_dir)
    }

    #[test]
    fn backup_requires_explicit_enabled_state() {
        let incomplete = serde_json::json!({
            "id": uuid::Uuid::now_v7(),
            "name": "Missing Enabled",
            "safeSearch": false,
            "icon": null,
            "schedules": [],
            "userRules": [],
            "connectedSitePacks": []
        });
        assert!(serde_json::from_value::<BackupNetworkBlock>(incomplete).is_err());
    }

    #[tokio::test]
    async fn test_backup_round_trip_preserves_remote_pack_semantics() {
        let (mut manager, _temp) = setup_manager();

        // A remote/server-managed pack (remote_id = catalog-style id) plus its
        // tier/version must come back from an export→import round trip as a
        // remote row, not be demoted to a local Free pack.
        manager
            .add_remote_site_pack_from_updater(
                &crate::core::RemoteSitePackId::try_new("pack-backup-test-remote".to_string())
                    .unwrap(),
                "Test Remote Pack",
                None,
                true,
                &crate::core::SitePackVersion::try_new("2026.09.02".to_string()).unwrap(),
            )
            .await
            .unwrap();

        let backup = manager.export_backup().unwrap();
        let pack_entry = backup
            .custom_site_packs
            .iter()
            .find(|p| p.remote_id.as_deref() == Some("pack-backup-test-remote"))
            .expect("remote pack must be exported with its remote linkage");
        assert!(pack_entry.requires_pro, "tier must be preserved on export");
        assert_eq!(pack_entry.version.as_deref(), Some("2026.09.02"));

        // Simulate a fresh engine: wipe the row, then import. Restoring by
        // remote linkage must reproduce the row as remote (not local).
        let row_before = manager
            .db
            .get_site_pack_by_remote_id("pack-backup-test-remote")
            .unwrap();
        manager.db.delete_site_pack(&row_before.id).unwrap();
        manager.import_backup(backup).await.unwrap();

        let restored = manager
            .db
            .get_site_pack_by_remote_id("pack-backup-test-remote")
            .unwrap();
        assert_eq!(
            restored.remote_id.as_deref(),
            Some("pack-backup-test-remote")
        );
        assert!(restored.requires_pro);
        assert_eq!(restored.version.as_deref(), Some("2026.09.02"));
    }

    #[tokio::test]
    async fn test_backup_round_trip_keeps_block_site_pack_reference() {
        let (mut manager, _temp) = setup_manager();

        // Build: block + connected pack with a NON-UUID catalog-style id (the
        // shape that used to break cloud backups expecting UUIDs).
        let pack = manager
            .add_local_site_pack("Custom List", None, None)
            .await
            .unwrap();
        pack.write().unwrap().reload_db().unwrap();
        let pack_id = pack.read().unwrap().get_id().clone();

        let block_id = manager
            .create_network_block("My Block".to_string())
            .unwrap();
        manager
            .add_network_block_to_site_pack(&block_id, &pack_id)
            .unwrap();

        let backup = manager.export_backup().unwrap();
        let block_entry = backup
            .network_blocks
            .iter()
            .find(|b| b.id.to_string() == block_id.to_string())
            .unwrap();
        assert!(
            block_entry
                .connected_site_packs
                .contains(&pack_id.to_string()),
            "the exported block must reference its site pack"
        );

        // Restore into the same manager: the reference survives (the pack row
        // is kept, not deleted-and-recreated as a different row).
        manager.import_backup(backup).await.unwrap();
        let block = manager
            .get_network_block_from(&block_id)
            .expect("block must still exist");
        assert!(block.has_site_pack(&pack_id));
    }
    fn backup_block(id: uuid::Uuid, name: &str) -> BackupNetworkBlock {
        BackupNetworkBlock {
            id,
            name: name.to_string(),
            enabled: true,
            safe_search: false,
            icon: None,
            schedules: vec![],
            user_rules: vec![],
            connected_site_packs: vec![],
        }
    }

    #[tokio::test]
    async fn backup_block_transaction_rolls_back_after_sql_write_failure() {
        use diesel::connection::SimpleConnection;
        let (mut manager, _temp) = setup_manager();
        let block_id = manager
            .create_network_block("Original".to_string())
            .unwrap();
        let raw_id = *block_id.as_ref();
        let mut backup = EngineBackup {
            version: "1.0".to_string(),
            custom_site_packs: vec![],
            network_blocks: vec![backup_block(raw_id, "Replacement")],
            deactivated_resistance_scripts: vec![],
        };
        backup.network_blocks[0]
            .schedules
            .push(BackupScheduleEntry {
                start_time: "09:00:00".to_string(),
                end_time: "10:00:00".to_string(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            });

        let db = manager.get_db();
        let mut connection = db.get_db_connection().unwrap();
        connection.batch_execute(
            "CREATE TRIGGER reject_restored_schedule BEFORE INSERT ON network_block_schedule_entries
             BEGIN SELECT RAISE(ABORT, 'injected schedule write failure'); END;",
        ).unwrap();
        drop(connection);

        assert!(manager.import_backup(backup).await.is_err());
        let block = db
            .get_all_network_blocks()
            .unwrap()
            .into_iter()
            .find(|row| row.id == raw_id.to_string())
            .unwrap();
        assert_eq!(
            block.name, "Original",
            "delete/insert must roll back as one unit"
        );
        assert!(db.get_schedule_entry_models_of(&raw_id).unwrap().is_empty());
        assert_eq!(
            manager
                .get_network_block_from(&block_id)
                .unwrap()
                .get_name(),
            "Original",
            "live policy must remain consistent after failed transaction"
        );
    }

    #[tokio::test]
    async fn backup_partial_success_reloads_all_committed_units_before_err() {
        let (mut manager, _temp) = setup_manager();
        let first = manager
            .create_network_block("First Original".to_string())
            .unwrap();
        let second = manager
            .create_network_block("Second Original".to_string())
            .unwrap();

        let mut invalid_second = backup_block(*second.as_ref(), "Second Replacement");
        invalid_second
            .connected_site_packs
            .push("not-present-pack".to_string());
        let backup = EngineBackup {
            version: "1.0".to_string(),
            custom_site_packs: vec![],
            network_blocks: vec![
                backup_block(*first.as_ref(), "First Replacement"),
                invalid_second,
            ],
            deactivated_resistance_scripts: vec![],
        };
        assert!(manager.import_backup(backup).await.is_err());
        assert_eq!(
            manager.get_network_block_from(&first).unwrap().get_name(),
            "First Replacement",
        );
        assert_eq!(
            manager.get_network_block_from(&second).unwrap().get_name(),
            "Second Original",
        );
        let rows = manager.get_db().get_all_network_blocks().unwrap();
        assert!(
            rows.iter()
                .any(|row| row.id == first.to_string() && row.name == "First Replacement")
        );
        assert!(
            rows.iter()
                .any(|row| row.id == second.to_string() && row.name == "Second Original")
        );
    }

    #[tokio::test]
    async fn invalid_site_pack_sources_cannot_leave_partial_restored_row() {
        let (mut manager, _temp) = setup_manager();
        let backup = EngineBackup {
            version: "1.0".to_string(),
            custom_site_packs: vec![BackupSitePack {
                id: "test-invalid-backup-source".to_string(),
                remote_id: None,
                requires_pro: false,
                version: None,
                name: "Test".to_string(),
                description: None,
                icon: None,
                sources: vec![BackupSitePackSource {
                    source: "https://example.com/list".to_string(),
                    source_type: "unexpected".to_string(),
                    format: "standard".to_string(),
                }],
            }],
            network_blocks: vec![],
            deactivated_resistance_scripts: vec![],
        };

        assert!(manager.import_backup(backup).await.is_err());
        let id = crate::core::SitePackId::try_new("test-invalid-backup-source").unwrap();
        assert!(!manager.get_db().site_pack_exists(&id).unwrap());
    }
    #[tokio::test]
    async fn backup_site_pack_sources_rollback_inserted_pack_on_sql_failure() {
        use diesel::connection::SimpleConnection;

        let (mut manager, _temp) = setup_manager();
        let db = manager.get_db();
        let mut connection = db.get_db_connection().unwrap();
        connection
            .batch_execute(
                "CREATE TRIGGER reject_backup_source BEFORE INSERT ON site_pack_sources
             BEGIN SELECT RAISE(ABORT, 'injected source write failure'); END;",
            )
            .unwrap();
        drop(connection);

        let backup = EngineBackup {
            version: "1.0".to_string(),
            custom_site_packs: vec![BackupSitePack {
                id: "test-source-rollback".to_string(),
                remote_id: None,
                requires_pro: false,
                version: None,
                name: "Atomic Pack".to_string(),
                description: None,
                icon: None,
                sources: vec![BackupSitePackSource {
                    source: "https://example.com/list".to_string(),
                    source_type: "url".to_string(),
                    format: "standard".to_string(),
                }],
            }],
            network_blocks: vec![],
            deactivated_resistance_scripts: vec![],
        };

        assert!(manager.import_backup(backup).await.is_err());
        let id = crate::core::SitePackId::try_new("test-source-rollback").unwrap();
        assert!(
            !db.site_pack_exists(&id).unwrap(),
            "pack and sources must roll back together"
        );
    }

    #[tokio::test]
    async fn backup_restore_skips_locked_blocks_without_weakening_policy() {
        use crate::core::{Lock, LockState};

        let (mut manager, _temp) = setup_manager();
        let block_id = manager
            .create_network_block("Protected".to_string())
            .unwrap();
        manager
            .set_lock(&block_id, Lock::from(LockState::SoftUnlockable))
            .unwrap();

        let backup = EngineBackup {
            version: "1.0".to_string(),
            custom_site_packs: vec![],
            network_blocks: vec![backup_block(*block_id.as_ref(), "Unwanted replacement")],
            deactivated_resistance_scripts: vec![],
        };

        manager.import_backup(backup).await.unwrap();
        let block = manager.get_network_block_from(&block_id).unwrap();
        assert_eq!(block.get_name(), "Protected");
        assert_eq!(block.get_lock().get_state(), LockState::SoftUnlockable);
        assert!(block.is_locked(&chrono::Local::now()));
    }
}
