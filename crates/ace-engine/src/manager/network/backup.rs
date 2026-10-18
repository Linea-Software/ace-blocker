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

fn default_enabled() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupNetworkBlock {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default = "default_enabled")]
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

    pub async fn import_backup(&mut self, backup: EngineBackup) -> Result<(), NetworkManagerOpErr> {
        if backup.version != "1.0" {
            return Err(NetworkManagerOpErr::InvalidBackupVersion {
                message: "expected version 1.0".to_string(),
            });
        }

        use crate::core::NetworkBlockId;
        use crate::db::models::NetworkBlockScheduleEntryModel;

        /////////////////
        // SITE PACKS //
        /////////////////
        for pack in backup.custom_site_packs {
            let id = pack.id;

            // Never delete-and-recreate an existing pack row: default packs
            // (id == remote_id, seeded by migration 0022), updater-registered
            // remote packs, and previously restored custom packs must keep
            // their row (remote linkage, version, data directory). The backup
            // only *adds* packs that are genuinely missing locally.
            if self.db.get_site_pack(&id).is_ok() {
                continue;
            }

            // Restore a missing pack with its original tier and remote
            // linkage (Issue 5: a backup taken from a Pro install must not
            // come back as a Free local pack, and a server-managed pack must
            // stay server-managed so the content updater keeps matching it).
            match pack.remote_id {
                Some(remote_id) => {
                    self.db
                        .add_remote_site_pack_with_optional_version(
                            &id,
                            &remote_id,
                            &pack.name,
                            pack.description.clone(),
                            pack.requires_pro,
                            pack.version,
                        )
                        .map_err(|error| NetworkManagerOpErr::DatabaseError {
                            message: error.to_string(),
                        })?;
                }
                None => {
                    self.db
                        .add_site_pack(
                            &id,
                            &pack.name,
                            pack.description.clone(),
                            pack.requires_pro,
                            pack.icon,
                        )
                        .map_err(|error| NetworkManagerOpErr::DatabaseError {
                            message: error.to_string(),
                        })?;
                }
            }

            for source in pack.sources {
                self.db
                    .add_site_pack_source(
                        &uuid::Uuid::now_v7(),
                        &id,
                        &source.source,
                        &source.source_type,
                        &source.format,
                    )
                    .map_err(|error| NetworkManagerOpErr::DatabaseError {
                        message: error.to_string(),
                    })?;
            }
        }

        ////////////////////
        // NETWORK BLOCKS //
        ////////////////////
        for block in backup.network_blocks {
            let id = block.id;
            if self.is_network_block_locked(&NetworkBlockId::from(id)) {
                continue;
            }

            if self
                .db
                .get_all_network_blocks()
                .unwrap_or_default()
                .iter()
                .any(|b| b.id == id.to_string())
            {
                let _ = self.db.delete_network_block(&id);
            }

            self.db
                .add_network_block_with_id(&id, block.name)
                .map_err(|error| NetworkManagerOpErr::DatabaseError {
                    message: error.to_string(),
                })?;

            self.db
                .set_network_block_enabled(&id, block.enabled)
                .map_err(|error| NetworkManagerOpErr::DatabaseError {
                    message: error.to_string(),
                })?;

            self.db
                .set_safe_search(&id, block.safe_search)
                .map_err(|error| NetworkManagerOpErr::DatabaseError {
                    message: error.to_string(),
                })?;

            if let Some(ref icon) = block.icon {
                self.db
                    .set_network_block_icon(&id, Some(icon.clone()))
                    .map_err(|error| NetworkManagerOpErr::DatabaseError {
                        message: error.to_string(),
                    })?;
            }

            let mut schedule_models = Vec::new();
            for sched in block.schedules {
                let mut bitmask = 0;
                if sched.monday {
                    bitmask |= 1 << 0;
                }
                if sched.tuesday {
                    bitmask |= 1 << 1;
                }
                if sched.wednesday {
                    bitmask |= 1 << 2;
                }
                if sched.thursday {
                    bitmask |= 1 << 3;
                }
                if sched.friday {
                    bitmask |= 1 << 4;
                }
                if sched.saturday {
                    bitmask |= 1 << 5;
                }
                if sched.sunday {
                    bitmask |= 1 << 6;
                }

                schedule_models.push(NetworkBlockScheduleEntryModel {
                    id: "".to_string(),
                    network_block_id: id.to_string(),
                    start_time: sched.start_time,
                    end_time: sched.end_time,
                    schedule_days_bitmask: bitmask,
                });
            }

            self.db
                .replace_schedule_entries(&id, schedule_models)
                .map_err(|error| NetworkManagerOpErr::DatabaseError {
                    message: error.to_string(),
                })?;

            let normalized_user_rules = crate::core::UserDefinedRules::new(block.user_rules)
                .map_err(|error| NetworkManagerOpErr::InternalError {
                    message: format!("invalid user rule in backup: {error}"),
                })?
                .rules()
                .to_vec();

            if !normalized_user_rules.is_empty() {
                self.db
                    .add_user_defined_rules(&id, &normalized_user_rules)
                    .map_err(|error| NetworkManagerOpErr::DatabaseError {
                        message: error.to_string(),
                    })?;
            }

            for site_pack_id in block.connected_site_packs {
                let _ = self.db.add_network_block_to_site_pack(&id, &site_pack_id);
            }
        }

        /////////////////////
        // RESISTANCE SCRIPTS //
        /////////////////////
        for script_id in backup.deactivated_resistance_scripts {
            let _ = self.db.set_script_state(&script_id.to_string(), false);
        }

        let db_arc = self.get_db();
        self.site_packs = Self::load_site_packs(&db_arc)?;
        self.network_blocks = Self::load_net_lists(&db_arc, &self.site_packs)?;
        self.resistance_mode.script_tree = Self::try_load_resistance_mode(&db_arc)?;

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
    fn test_legacy_backup_without_enabled_defaults_to_enabled() {
        let legacy = serde_json::json!({
            "id": uuid::Uuid::now_v7(),
            "name": "Legacy Block",
            "safeSearch": false,
            "icon": null,
            "schedules": [],
            "userRules": [],
            "connectedSitePacks": []
        });

        let block: BackupNetworkBlock = serde_json::from_value(legacy).unwrap();
        assert!(block.enabled);
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
}
