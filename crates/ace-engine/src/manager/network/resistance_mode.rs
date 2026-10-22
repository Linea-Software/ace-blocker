use tracing::{info, warn};

use crate::{
    manager::NetworkManagerOpErr, manager::resistance::PunishmentIntensity, settings::Settings,
};

use super::NetworkManager;

/// Machine-readable enforcement status for Resistance Mode.
///
/// The engine never reports `operational` merely because the master setting
/// is enabled: scripts must be loaded, eligible for injection, and a
/// non-`none` intensity must be configured. Priority order when multiple
/// conditions fail: `disabled` > `not_pro` > `scripts_missing` >
/// `scripts_invalid` > `no_scripts_enabled` > `intensity_none`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementStatus {
    /// Resistance Mode is enforcing on covered allowed sites right now.
    Operational,
    /// The master switch is off.
    Disabled,
    /// The account is not Pro; injection and downloads are gated on Pro.
    NotPro,
    /// No script catalog/tree is installed (never downloaded or deleted).
    ScriptsMissing,
    /// The installed catalog failed to load (corrupt/unreadable).
    ScriptsInvalid,
    /// Scripts are installed but every script is disabled.
    NoScriptsEnabled,
    /// Static intensity is `none` — configured but intentionally not friction.
    IntensityNone,
}

impl NetworkManager {
    /// Returns true if the resistance mode is currently locked.
    pub fn is_resistance_mode_locked(&self) -> bool {
        self.resistance_mode.is_locked()
    }

    /// Returns true if the resistance mode (master switch) is currently enabled.
    pub fn is_resistance_mode_enabled(&self) -> bool {
        self.resistance_mode.enabled
    }

    pub fn has_scripts_of(&self, host: &str) -> bool {
        self.resistance_mode.script_tree.has_scripts_of(host)
    }

    /// Computes the truthful enforcement status of Resistance Mode.
    ///
    /// This is what the UI should render: it distinguishes "configured on"
    /// from "actually enforcing". See [`EnforcementStatus`] for the reason
    /// codes and their priority.
    pub fn resistance_enforcement_status(&self) -> EnforcementStatus {
        let engine = &self.resistance_mode;

        if !engine.enabled {
            return EnforcementStatus::Disabled;
        }

        if !self.is_pro_user_cached() {
            return EnforcementStatus::NotPro;
        }

        let entries = engine.script_tree.get_all_script_entries();
        if entries.is_empty() {
            // The discriminator is the catalog file: no file at all means the
            // scripts were never installed (or were deleted); a file that
            // produced no tree means the installed catalog is unreadable,
            // empty, or failed to load.
            if Settings::get_resistance_scripts_json_path().exists() {
                return EnforcementStatus::ScriptsInvalid;
            }
            return EnforcementStatus::ScriptsMissing;
        }

        if entries.iter().all(|entry| !entry.enabled) {
            return EnforcementStatus::NoScriptsEnabled;
        }

        if engine.get_intensity() == PunishmentIntensity::None {
            return EnforcementStatus::IntensityNone;
        }

        EnforcementStatus::Operational
    }

    pub fn get_resistance_scripts_of(&self, host: &str) -> Vec<String> {
        self.resistance_mode.script_tree.get_scripts(host)
    }

    pub fn set_script_enabled(
        &mut self,
        id: &crate::core::common::injectable_scripts::ScriptId,
        enabled: bool,
    ) -> Result<bool, NetworkManagerOpErr> {
        if self.resistance_mode.is_locked() && !enabled {
            return Err(NetworkManagerOpErr::ResistanceModeIsLocked);
        }
        if self
            .resistance_mode
            .script_tree
            .get_script_by_id(id)
            .is_none()
        {
            return Ok(false);
        }
        self.db.set_script_state((*id).as_ref(), enabled)?;
        Ok(self
            .resistance_mode
            .script_tree
            .set_script_enabled(id, enabled))
    }

    fn persist_resistance_mode_settings(
        &self,
        enabled: bool,
        lock: &crate::core::Lock,
        intensity: PunishmentIntensity,
    ) -> Result<(), NetworkManagerOpErr> {
        // Clean up expired lock before persisting — an ActiveUntilLockEnd lock
        // whose end time has passed should be stored as Inactive.
        let (lock_state, lock_end) = match lock.get_state() {
            crate::core::LockState::ActiveUntilLockEnd { end } if chrono::Local::now() > end => {
                ("inactive".to_string(), None)
            }
            other => (
                other.to_string(),
                lock.get_end_time().map(|end| {
                    end.format(crate::core::constants::DATETIME_FORMAT)
                        .to_string()
                }),
            ),
        };

        self.db
            .set_resistance_mode_settings(crate::db::models::ResistanceModeSettingsModel {
                table_lock: "0".to_string(),
                enabled,
                lock_state,
                lock_end,
                static_intensity: intensity.to_string(),
                scripts_version: self.resistance_mode.scripts_version.clone(),
            })?;
        Ok(())
    }

    pub fn set_resistance_mode_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<(), NetworkManagerOpErr> {
        if self.resistance_mode.is_locked() && !enabled {
            return Err(NetworkManagerOpErr::ResistanceModeIsLocked);
        }
        let next_lock = if enabled {
            self.resistance_mode.lock.clone()
        } else {
            crate::core::Lock::default()
        };
        self.persist_resistance_mode_settings(
            enabled,
            &next_lock,
            self.resistance_mode.get_intensity(),
        )?;
        self.resistance_mode.enabled = enabled;
        self.resistance_mode.lock = next_lock;
        Ok(())
    }

    pub fn set_resistance_mode_lock(
        &mut self,
        lock: crate::core::Lock,
    ) -> Result<(), NetworkManagerOpErr> {
        // v1 beta: Resistance Mode supports exactly two lock states — `inactive`
        // and `active_until_lock_end` (a one-way timed lock). `soft_unlockable`
        // and `active_during_schedule` are not part of the public contract and
        // must be rejected BEFORE any in-memory mutation or DB write, so a
        // rejected request can never diverge memory from the database.
        match lock.get_state() {
            crate::core::LockState::Inactive
            | crate::core::LockState::ActiveUntilLockEnd { .. } => {}
            other => {
                return Err(NetworkManagerOpErr::UnsupportedLockState {
                    state: other.to_string(),
                });
            }
        }

        if !self.resistance_mode.enabled {
            return Err(NetworkManagerOpErr::InternalError {
                message: "Cannot lock resistance engine when disabled".to_string(),
            });
        }

        if self.resistance_mode.is_locked()
            && !lock_is_stricter_than(&lock, &self.resistance_mode.lock)
        {
            return Err(NetworkManagerOpErr::ResistanceModeIsLocked);
        }

        self.persist_resistance_mode_settings(
            self.resistance_mode.enabled,
            &lock,
            self.resistance_mode.get_intensity(),
        )?;
        self.resistance_mode.lock = lock;
        Ok(())
    }

    pub fn set_resistance_mode_type(
        &mut self,
        mode: crate::manager::resistance::punishment::PunishmentMode,
    ) -> Result<(), NetworkManagerOpErr> {
        if self.resistance_mode.is_locked() {
            return Err(NetworkManagerOpErr::ResistanceModeIsLocked);
        }

        // v1 beta: static mode only (MGR-24) — there is no dynamic variant.
        let crate::manager::resistance::punishment::PunishmentMode::Static { intensity } = mode;
        self.persist_resistance_mode_settings(
            self.resistance_mode.enabled,
            &self.resistance_mode.lock,
            intensity,
        )?;
        self.resistance_mode
            .set_punishment(crate::manager::resistance::Punishment::new_static(
                intensity,
            ));
        Ok(())
    }

    /// Commits a staged resistance-scripts bundle under the manager write lock.
    ///
    /// The bundle was downloaded, SHA-256 verified, and extracted into
    /// `staged.staging_dir` by the content updater **outside** this lock. This
    /// method only swaps the scripts directory atomically (with rollback),
    /// reloads the in-memory script tree, and — only after everything
    /// succeeded — persists the new version to memory **and** SQLite, so a
    /// settings save can never clobber the database back to a stale version.
    pub fn commit_resistance_scripts(
        &mut self,
        staged: crate::service::content_updater::StagedScriptBundle,
    ) -> Result<(), NetworkManagerOpErr> {
        let live_dir = Settings::get_resistance_scripts_dir();
        let parent = live_dir
            .parent()
            .ok_or_else(|| NetworkManagerOpErr::InternalError {
                message: "resistance scripts directory has no parent".to_string(),
            })?
            .to_path_buf();
        let unique = uuid::Uuid::new_v4();
        let old_dir = parent.join(format!(".resistance_scripts.old-{unique}"));
        let previous_tree = self.resistance_mode.script_tree.clone();
        let previous_load_error = self.resistance_mode.scripts_load_error.clone();

        // Atomic swap: live dir -> old dir, staged dir -> live dir.
        let live_exists = live_dir.exists();
        if live_exists && let Err(error) = std::fs::rename(&live_dir, &old_dir) {
            return Err(NetworkManagerOpErr::ScriptDownloadFailed {
                message: format!("failed to move the current resistance scripts aside: {error}"),
            });
        }
        if let Err(error) = std::fs::rename(&staged.staging_dir, &live_dir) {
            if live_exists {
                let _ = std::fs::rename(&old_dir, &live_dir);
            }
            return Err(NetworkManagerOpErr::ScriptDownloadFailed {
                message: format!("failed to activate the new resistance scripts: {error}"),
            });
        }

        // Reload the in-memory tree from the new catalog; on failure roll the
        // directory swap back and reload the previous tree.
        if let Err(error) = self.reload_scripts() {
            let _ = std::fs::remove_dir_all(&live_dir);
            if live_exists {
                let _ = std::fs::rename(&old_dir, &live_dir);
            }
            let _ = self.reload_scripts();
            return Err(error);
        }

        // SQLite is not part of the filesystem rename transaction. If it
        // rejects the version write, restore the previous working disk + memory
        // state; never publish the new tree under the old persisted version.
        if let Err(db_error) = self.db.update_scripts_version(staged.version.clone()) {
            let failed_dir = parent.join(format!(".resistance_scripts.failed-{unique}"));
            std::fs::rename(&live_dir, &failed_dir).map_err(|error| {
                NetworkManagerOpErr::InternalError {
                    message: format!(
                        "script version persistence failed ({db_error}); unable to quarantine activated scripts for rollback: {error}; old scripts retained at {}",
                        old_dir.display()
                    ),
                }
            })?;

            if live_exists {
                if let Err(error) = std::fs::rename(&old_dir, &live_dir) {
                    // Keep the previous bundle on disk for explicit recovery.
                    let _ = std::fs::rename(&failed_dir, &live_dir);
                    return Err(NetworkManagerOpErr::InternalError {
                        message: format!(
                            "script version persistence failed ({db_error}); rollback failed ({error}); previous scripts preserved at {}",
                            old_dir.display()
                        ),
                    });
                }
            }

            self.resistance_mode.script_tree = previous_tree;
            self.resistance_mode.scripts_load_error = previous_load_error;
            if let Err(error) = crate::core::utils::remove_superseded_dir(&failed_dir) {
                warn!(%error, path = %failed_dir.display(),
                    "failed to clean up uncommitted resistance scripts");
            }
            return Err(db_error.into());
        }
        self.resistance_mode.scripts_version = Some(staged.version.clone());

        // Best-effort cleanup of the superseded directory. `NotFound`
        // (os error 2) is an expected outcome of the idempotent swap — the
        // `.old-*` dir only exists when a live dir was renamed aside — so it
        // is handled silently; genuine failures still warn below.
        if let Err(error) = crate::core::utils::remove_superseded_dir(&old_dir) {
            warn!(
                "failed to delete superseded resistance scripts at {}: {}",
                old_dir.display(),
                error
            );
        }

        info!("Resistance scripts committed and reloaded successfully.");
        Ok(())
    }

    /// Reloads the script tree from the catalog on disk.
    ///
    /// On failure the previous working in-memory tree is preserved and the
    /// failure reason is recorded for the enforcement-status computation.
    pub fn reload_scripts(&mut self) -> Result<(), NetworkManagerOpErr> {
        // Keep a backup of the old tree during the transition
        let old_tree = self.resistance_mode.script_tree.clone();

        // Completely remove the old tree from memory to ensure a fresh start
        self.resistance_mode.script_tree = super::InjectableScriptTree::default();

        // Attempt to reload from disk/DB
        match Self::try_load_resistance_mode(&self.db) {
            Ok(new_tree) => {
                info!("Resistance engine reloaded successfully.");
                self.resistance_mode.script_tree = new_tree;
                self.resistance_mode.scripts_load_error = None;
                Ok(())
            }
            Err(error) => {
                warn!(
                    "Failed to reload resistance engine: {}. Falling back to previous version.",
                    error
                );
                self.resistance_mode.script_tree = old_tree;
                self.resistance_mode.scripts_load_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    /// Loads the resistance mode (InjectableScriptTree) from the catalog JSON.
    ///
    /// After loading, stored per-script enabled/disabled states from the
    /// `script_states` table are applied by script id, overriding the serde
    /// default (enabled). The script id is the permanent key: name,
    /// description, or any other metadata changing never affects a stored
    /// state, and a script id with no stored row stays enabled.
    ///
    /// Orphan rows (ids no longer present in the catalog) are intentionally
    /// kept in the DB — a script may return in a future catalog update and
    /// the user's choice should still be there when it does. Tradeoff: rows
    /// accumulate across catalog changes, but the table stores one bool per
    /// script id, so the growth is negligible.
    pub(crate) fn try_load_resistance_mode(
        db: &crate::db::EngineDB,
    ) -> Result<super::InjectableScriptTree, NetworkManagerOpErr> {
        let mut tree = super::InjectableScriptTree::load_from_json(
            &Settings::get_resistance_scripts_json_path(),
        )
        .map_err(|error| NetworkManagerOpErr::InternalError {
            message: format!("Failed to load resistance mode: {}", error),
        })?;

        let states = db.get_all_script_states()?;
        for state in states {
            tree.set_script_enabled(
                &crate::core::common::injectable_scripts::ScriptId::try_new(state.script_id)
                    .map_err(|error| NetworkManagerOpErr::InternalError {
                        message: format!("Invalid persisted script identifier: {error}"),
                    })?,
                state.enabled,
            );
        }

        Ok(tree)
    }
}

/// Helper function to check if a new lock is stricter than the current one.
fn lock_is_stricter_than(new_lock: &crate::core::Lock, current_lock: &crate::core::Lock) -> bool {
    use crate::core::LockState;

    match (current_lock.get_state(), new_lock.get_state()) {
        // Inactive is the least strict
        (LockState::Inactive, _) => true,
        (_, LockState::Inactive) => false,

        // SoftUnlockable sits between Inactive and ActiveDuringSchedule
        (LockState::SoftUnlockable, LockState::SoftUnlockable) => false,
        (
            LockState::SoftUnlockable,
            LockState::ActiveDuringSchedule | LockState::ActiveUntilLockEnd { .. },
        ) => true,
        (
            LockState::ActiveDuringSchedule | LockState::ActiveUntilLockEnd { .. },
            LockState::SoftUnlockable,
        ) => false,

        // ActiveUntilLockEnd is stricter than ActiveDuringSchedule if it's still active
        (LockState::ActiveDuringSchedule, LockState::ActiveUntilLockEnd { .. }) => true,
        (LockState::ActiveUntilLockEnd { .. }, LockState::ActiveDuringSchedule) => false,

        // Same types: compare end times
        (
            LockState::ActiveUntilLockEnd { end: current_end },
            LockState::ActiveUntilLockEnd { end: new_end },
        ) => new_end > current_end,

        // Same type, no change
        (LockState::ActiveDuringSchedule, LockState::ActiveDuringSchedule) => false,
    }
}

#[cfg(test)]
mod tests {
    // Test-serialization locks (TEST_DIR_LOCK) are held across awaits by
    // design: they protect process-global test dirs for the whole test.
    #![expect(clippy::await_holding_lock)]

    use super::*;
    use crate::core::common::injectable_scripts::ScriptId;
    use crate::db::EngineDB;
    use crate::manager::AccountManager;
    use crate::manager::NetworkManager;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Serializes tests that touch the global test resistance-scripts dir.
    static TEST_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const SCRIPT_GRAYSCALE: &str = "console.log('grayscale');";
    const SCRIPT_CAPTCHA: &str = "console.log('captcha');";
    const SCRIPT_SHORTS: &str = "console.log('shorts');";

    fn catalog_with_scripts(scripts: &[(&str, &str, &str)]) -> String {
        // (id, name, path)
        let entries: Vec<String> = scripts
            .iter()
            .map(|(id, name, path)| {
                format!(
                    r#"{{"id": "{}", "path": "{}", "name": "{}", "description": "test", "version": "1.0"}}"#,
                    id, path, name
                )
            })
            .collect();
        format!(
            r#"{{"sites": {{"com": {{"scripts": [], "children": {{"example": {{"scripts": [{}], "children": {{}}}}}}}}}}}}"#,
            entries.join(",")
        )
    }

    fn write_catalog(dir: &std::path::Path, catalog: &str) {
        std::fs::write(dir.join("scripts-data.json"), catalog).unwrap();
    }

    fn write_script_file(dir: &std::path::Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    /// Creates a temp dir with a catalog + script files, a fresh DB, and an
    /// empty NetworkManager wired to both. Callers must hold TEST_DIR_LOCK.
    fn setup(v1_scripts: &[(&str, &str, &str)]) -> (TempDir, Arc<EngineDB>, NetworkManager) {
        let temp = tempfile::tempdir().unwrap();
        let scripts_dir = temp.path().join("resistance_scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        Settings::set_test_resistance_scripts_dir(scripts_dir.clone());

        for (id, _, path) in v1_scripts {
            let content = match *id {
                "grayscale" => SCRIPT_GRAYSCALE,
                "captcha" => SCRIPT_CAPTCHA,
                "shorts" => SCRIPT_SHORTS,
                _ => "console.log('other');",
            };
            write_script_file(&scripts_dir, path, content);
        }
        write_catalog(&scripts_dir, &catalog_with_scripts(v1_scripts));

        let db = Arc::new(
            EngineDB::from(&temp.path().join("test.sqlite")).expect("Failed to open test DB"),
        );
        let account_manager = AccountManager::from(db.clone()).unwrap();
        let manager = NetworkManager::new_empty(db.clone(), account_manager);
        (temp, db, manager)
    }

    fn all_enabled(manager: &NetworkManager) -> bool {
        manager
            .get_resistance_mode()
            .script_tree
            .get_all_script_entries()
            .iter()
            .all(|entry| entry.enabled)
    }

    fn staged_bundle(
        parent: &std::path::Path,
        name: &str,
        version: &str,
        scripts: &[(&str, &str, &str)],
    ) -> crate::service::content_updater::StagedScriptBundle {
        let staging_dir = parent.join(format!(".{name}.staging-test"));
        std::fs::create_dir_all(&staging_dir).unwrap();
        for (id, _, path) in scripts {
            let content = match *id {
                "grayscale" => SCRIPT_GRAYSCALE,
                "captcha" => SCRIPT_CAPTCHA,
                "shorts" => SCRIPT_SHORTS,
                _ => "console.log('other');",
            };
            write_script_file(&staging_dir, path, content);
        }
        write_catalog(&staging_dir, &catalog_with_scripts(scripts));
        crate::service::content_updater::StagedScriptBundle {
            version: version.to_string(),
            staging_dir,
        }
    }

    #[test]
    fn commit_resistance_scripts_swaps_directory_and_persists_version() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);
        manager.reload_scripts().unwrap();
        assert!(
            manager
                .get_resistance_mode()
                .script_tree
                .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .is_some()
        );

        let bundle = staged_bundle(
            temp.path(),
            "resistance_scripts",
            "2.0",
            &[
                ("grayscale", "Grayscale v2", "grayscale.js"),
                ("shorts", "Shorts", "shorts.js"),
            ],
        );
        manager.commit_resistance_scripts(bundle).unwrap();

        // The new tree is live.
        let tree = &manager.get_resistance_mode().script_tree;
        assert!(
            tree.get_script_by_id(&ScriptId::try_new("shorts").unwrap())
                .is_some(),
            "new scripts must be live after the commit"
        );
        assert_eq!(
            tree.get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .unwrap()
                .name,
            "Grayscale v2"
        );
        // Version persisted in memory AND in SQLite.
        assert_eq!(
            manager.get_resistance_mode().scripts_version.as_deref(),
            Some("2.0")
        );
        assert_eq!(
            db.get_resistance_mode_settings()
                .unwrap()
                .scripts_version
                .as_deref(),
            Some("2.0")
        );
        // The staging directory is consumed (renamed into place).
        assert!(
            !temp
                .path()
                .join(".resistance_scripts.staging-test")
                .exists(),
            "staging dir must be consumed by the commit"
        );
        // The live dir holds the new catalog.
        let live_catalog = std::fs::read_to_string(
            temp.path()
                .join("resistance_scripts")
                .join("scripts-data.json"),
        )
        .unwrap();
        assert!(live_catalog.contains("Shorts"));
    }

    #[test]
    fn commit_resistance_scripts_sqlite_failure_rolls_back_disk_and_memory() {
        use diesel::RunQueryDsl;

        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);
        manager.reload_scripts().unwrap();

        // Failure injection: updating the version fails after activating the catalog.
        let mut conn = db.get_db_connection().unwrap();
        diesel::sql_query("DROP TABLE resistance_mode_settings")
            .execute(&mut conn)
            .unwrap();
        drop(conn);

        let bundle = staged_bundle(
            temp.path(),
            "resistance_scripts",
            "2.0",
            &[("shorts", "Shorts", "shorts.js")],
        );
        assert!(manager.commit_resistance_scripts(bundle).is_err());
        assert!(manager.get_resistance_mode().scripts_version.is_none());
        let tree = &manager.get_resistance_mode().script_tree;
        assert!(
            tree.get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .is_some()
        );
        assert!(
            tree.get_script_by_id(&ScriptId::try_new("shorts").unwrap())
                .is_none()
        );
        let live_catalog =
            std::fs::read_to_string(temp.path().join("resistance_scripts/scripts-data.json"))
                .unwrap();
        assert!(live_catalog.contains("Grayscale"));
        assert!(!live_catalog.contains("Shorts"));
    }

    #[test]
    fn commit_resistance_scripts_failure_preserves_the_previous_bundle() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, _db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);
        manager.reload_scripts().unwrap();

        // A staged bundle whose catalog is invalid JSON must fail the commit
        // and leave the previous tree + directory intact.
        let staging_dir = temp.path().join(".resistance_scripts.staging-test");
        std::fs::create_dir_all(&staging_dir).unwrap();
        write_catalog(&staging_dir, "{ not valid json !!!");
        let bundle = crate::service::content_updater::StagedScriptBundle {
            version: "2.0".to_string(),
            staging_dir,
        };

        let result = manager.commit_resistance_scripts(bundle);
        assert!(result.is_err(), "a corrupt staged catalog must fail");
        assert!(
            manager
                .get_resistance_mode()
                .script_tree
                .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .is_some(),
            "the previous tree must survive"
        );
        assert_eq!(
            manager.get_resistance_mode().scripts_version.as_deref(),
            None,
            "the failed version must not be persisted"
        );
        // The live directory still has the v1 catalog.
        let live_catalog = std::fs::read_to_string(
            temp.path()
                .join("resistance_scripts")
                .join("scripts-data.json"),
        )
        .unwrap();
        assert!(
            live_catalog.contains("Grayscale"),
            "the previous on-disk bundle must survive"
        );
    }

    #[test]
    fn failed_settings_writes_leave_resistance_policy_unchanged() {
        use diesel::connection::SimpleConnection;
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (_temp, db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);
        manager.reload_scripts().unwrap();
        db.get_db_connection()
            .unwrap()
            .batch_execute("DROP TABLE script_states")
            .unwrap();
        let script_id = ScriptId::try_new("grayscale").unwrap();
        assert!(manager.set_script_enabled(&script_id, false).is_err());
        assert!(
            manager
                .resistance_mode
                .script_tree
                .get_script_by_id(&script_id)
                .unwrap()
                .enabled
        );

        manager.set_resistance_mode_enabled(true).unwrap();
        db.get_db_connection()
            .unwrap()
            .batch_execute("DROP TABLE resistance_mode_settings")
            .unwrap();

        assert!(manager.set_resistance_mode_enabled(false).is_err());
        assert!(manager.is_resistance_mode_enabled());
        let future = chrono::Local::now() + chrono::Duration::hours(1);
        let lock =
            crate::core::Lock::from(crate::core::LockState::ActiveUntilLockEnd { end: future });
        assert!(manager.set_resistance_mode_lock(lock).is_err());
        assert!(!manager.is_resistance_mode_locked());

        assert!(
            manager
                .set_resistance_mode_type(crate::manager::resistance::PunishmentMode::Static {
                    intensity: PunishmentIntensity::Medium,
                },)
                .is_err()
        );
        assert_eq!(
            manager.resistance_mode.get_intensity(),
            PunishmentIntensity::None
        );
    }

    #[test]
    fn test_script_states_survive_catalog_update() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, _db, mut manager) = setup(&[
            ("grayscale", "Grayscale", "grayscale.js"),
            ("captcha", "Captcha", "captcha.js"),
        ]);
        // Initial load: everything enabled (serde default).
        manager.reload_scripts().unwrap();
        assert!(
            all_enabled(&manager),
            "fresh catalog should load all enabled"
        );

        // Disable two scripts by id.
        manager
            .set_script_enabled(&ScriptId::try_new("grayscale").unwrap(), false)
            .unwrap();
        manager
            .set_script_enabled(&ScriptId::try_new("captcha").unwrap(), false)
            .unwrap();

        // Catalog update: grayscale renamed, captcha removed, shorts added.
        let scripts_dir = temp.path().join("resistance_scripts");
        write_script_file(&scripts_dir, "shorts.js", SCRIPT_SHORTS);
        write_catalog(
            &scripts_dir,
            &catalog_with_scripts(&[
                ("grayscale", "Grayscale v2", "grayscale.js"),
                ("shorts", "Shorts", "shorts.js"),
            ]),
        );
        manager.reload_scripts().unwrap();

        let tree = &manager.get_resistance_mode().script_tree;

        // Renamed script keeps its disabled state — the id is the key.
        let grayscale = tree
            .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
            .expect("grayscale should still be in the catalog");
        assert_eq!(grayscale.name, "Grayscale v2", "metadata should refresh");
        assert!(!grayscale.enabled, "disabled state must survive a rename");

        // Removed script is gone from the tree.
        assert!(
            tree.get_script_by_id(&ScriptId::try_new("captcha").unwrap())
                .is_none()
        );

        // New script id arrives enabled.
        let shorts = tree
            .get_script_by_id(&ScriptId::try_new("shorts").unwrap())
            .expect("shorts should be in the catalog");
        assert!(shorts.enabled, "new script id should default to enabled");
    }

    #[test]
    fn test_removed_script_state_restored_on_reappearance() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, _db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);

        manager.reload_scripts().unwrap();
        manager
            .set_script_enabled(&ScriptId::try_new("grayscale").unwrap(), false)
            .unwrap();

        // v2: grayscale removed from the catalog.
        let scripts_dir = temp.path().join("resistance_scripts");
        write_catalog(&scripts_dir, &catalog_with_scripts(&[]));
        manager.reload_scripts().unwrap();
        assert!(
            manager
                .get_resistance_mode()
                .script_tree
                .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .is_none()
        );

        // v3: grayscale returns with a new name — the stored disabled state
        // must still be there.
        write_catalog(
            &scripts_dir,
            &catalog_with_scripts(&[("grayscale", "Grayscale v3", "grayscale.js")]),
        );
        manager.reload_scripts().unwrap();
        let grayscale = manager
            .get_resistance_mode()
            .script_tree
            .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
            .expect("grayscale should be back");
        assert!(
            !grayscale.enabled,
            "state must be restored for a returning id"
        );
    }

    #[tokio::test]
    async fn test_backup_import_round_trips_script_states() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (_temp, _db, mut manager) = setup(&[
            ("grayscale", "Grayscale", "grayscale.js"),
            ("captcha", "Captcha", "captcha.js"),
        ]);

        manager.reload_scripts().unwrap();
        manager
            .set_script_enabled(&ScriptId::try_new("grayscale").unwrap(), false)
            .unwrap();
        manager
            .set_script_enabled(&ScriptId::try_new("captcha").unwrap(), false)
            .unwrap();

        // Export captures the deactivated ids.
        let backup = manager.export_backup().unwrap();
        assert_eq!(backup.deactivated_resistance_scripts.len(), 2);
        assert!(
            backup
                .deactivated_resistance_scripts
                .contains(&"grayscale".to_string())
        );
        assert!(
            backup
                .deactivated_resistance_scripts
                .contains(&"captcha".to_string())
        );

        // Flip both back on, then import the backup: states must round-trip
        // and be applied after the reload-from-catalog.
        manager
            .set_script_enabled(&ScriptId::try_new("grayscale").unwrap(), true)
            .unwrap();
        manager
            .set_script_enabled(&ScriptId::try_new("captcha").unwrap(), true)
            .unwrap();
        assert!(all_enabled(&manager));

        manager.import_backup(backup).await.unwrap();

        let tree = &manager.get_resistance_mode().script_tree;
        assert!(
            !tree
                .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .unwrap()
                .enabled
        );
        assert!(
            !tree
                .get_script_by_id(&ScriptId::try_new("captcha").unwrap())
                .unwrap()
                .enabled
        );
    }

    // ── v1 beta regression tests ────────────────────────────────────

    #[test]
    fn test_reload_failure_preserves_tree_and_records_error() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, _db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);

        manager.reload_scripts().unwrap();
        assert!(
            manager
                .get_resistance_mode()
                .script_tree
                .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .is_some()
        );
        assert!(manager.get_resistance_mode().scripts_load_error.is_none());

        // Corrupt the catalog: reload must fail, keep the old tree, and
        // record the load error for diagnostics.
        let scripts_dir = temp.path().join("resistance_scripts");
        write_catalog(&scripts_dir, "{ not valid json !!!");

        assert!(
            manager.reload_scripts().is_err(),
            "corrupt catalog must fail"
        );
        assert!(
            manager
                .get_resistance_mode()
                .script_tree
                .get_script_by_id(&ScriptId::try_new("grayscale").unwrap())
                .is_some(),
            "previous working tree must be preserved on failure"
        );
        assert!(
            manager.get_resistance_mode().scripts_load_error.is_some(),
            "load failure must be recorded"
        );

        // A good catalog clears the error flag.
        write_catalog(
            &scripts_dir,
            &catalog_with_scripts(&[("grayscale", "Grayscale", "grayscale.js")]),
        );
        manager.reload_scripts().unwrap();
        assert!(manager.get_resistance_mode().scripts_load_error.is_none());
    }

    #[test]
    fn test_enforcement_status_matrix() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (temp, _db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);
        let scripts_dir = temp.path().join("resistance_scripts");

        // Disabled wins over everything.
        manager.reload_scripts().unwrap();
        assert_eq!(
            manager.resistance_enforcement_status(),
            EnforcementStatus::Disabled
        );

        // Enabled but not Pro → not_pro.
        manager.set_resistance_mode_enabled(true).unwrap();
        assert_eq!(
            manager.resistance_enforcement_status(),
            EnforcementStatus::NotPro
        );

        // Pro + enabled → intensity_none (static None is not friction, so
        // not operational).
        manager.set_debug_pro_policy(super::super::DebugProPolicy::Pro);
        assert_eq!(
            manager.resistance_enforcement_status(),
            EnforcementStatus::IntensityNone
        );

        // All scripts disabled → no_scripts_enabled.
        manager
            .set_script_enabled(&ScriptId::try_new("grayscale").unwrap(), false)
            .unwrap();
        assert_eq!(
            manager.resistance_enforcement_status(),
            EnforcementStatus::NoScriptsEnabled
        );
        manager
            .set_script_enabled(&ScriptId::try_new("grayscale").unwrap(), true)
            .unwrap();

        // Non-None static intensity + enabled script + Pro → operational.
        manager
            .set_resistance_mode_type(
                crate::manager::resistance::punishment::PunishmentMode::Static {
                    intensity: PunishmentIntensity::Medium,
                },
            )
            .unwrap();
        assert_eq!(
            manager.resistance_enforcement_status(),
            EnforcementStatus::Operational
        );

        // Startup-style scenarios: a fresh manager whose tree never loaded.
        // (a) No catalog file on disk → scripts_missing.
        let missing_dir = tempfile::tempdir().unwrap();
        Settings::set_test_resistance_scripts_dir(missing_dir.path().to_path_buf());
        let db_missing =
            Arc::new(EngineDB::from(&temp.path().join("missing.sqlite")).expect("test DB"));
        let mut manager_missing = NetworkManager::new_empty(
            db_missing.clone(),
            AccountManager::from(db_missing).unwrap(),
        );
        manager_missing.set_resistance_mode_enabled(true).unwrap();
        manager_missing.set_debug_pro_policy(super::super::DebugProPolicy::Pro);
        assert_eq!(
            manager_missing.resistance_enforcement_status(),
            EnforcementStatus::ScriptsMissing
        );

        // (b) Catalog file exists but nothing loaded → scripts_invalid.
        Settings::set_test_resistance_scripts_dir(scripts_dir.clone());
        write_catalog(&scripts_dir, "{ corrupt json");
        let db_invalid =
            Arc::new(EngineDB::from(&temp.path().join("invalid.sqlite")).expect("test DB"));
        let mut manager_invalid = NetworkManager::new_empty(
            db_invalid.clone(),
            AccountManager::from(db_invalid).unwrap(),
        );
        manager_invalid.set_resistance_mode_enabled(true).unwrap();
        manager_invalid.set_debug_pro_policy(super::super::DebugProPolicy::Pro);
        assert_eq!(
            manager_invalid.resistance_enforcement_status(),
            EnforcementStatus::ScriptsInvalid
        );

        // Restore a valid catalog for the remaining tests in this module.
        write_catalog(
            &scripts_dir,
            &catalog_with_scripts(&[("grayscale", "Grayscale", "grayscale.js")]),
        );
    }

    #[test]
    fn test_unsupported_lock_states_rejected_before_mutation() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let (_temp, _db, mut manager) = setup(&[("grayscale", "Grayscale", "grayscale.js")]);
        manager.set_resistance_mode_enabled(true).unwrap();

        for state in ["soft_unlockable", "active_during_schedule"] {
            let lock = crate::core::Lock::from_raw(state.to_string(), None).unwrap();
            let result = manager.set_resistance_mode_lock(lock);
            assert!(result.is_err(), "{} must be rejected", state);
            assert!(
                !manager.is_resistance_mode_locked(),
                "rejected lock must not mutate in-memory state"
            );
            // No partial DB write: the stored lock must still be inactive.
            let stored = manager.db.get_resistance_mode_settings().unwrap();
            assert_eq!(stored.lock_state, "inactive");
        }

        // A valid timed lock still works after the rejections.
        let end = (chrono::Local::now() + chrono::Duration::hours(1))
            .format(crate::core::constants::DATETIME_FORMAT)
            .to_string();
        let lock =
            crate::core::Lock::from_raw("active_until_lock_end".to_string(), Some(end)).unwrap();
        manager.set_resistance_mode_lock(lock).unwrap();
        assert!(manager.is_resistance_mode_locked());
    }
}
