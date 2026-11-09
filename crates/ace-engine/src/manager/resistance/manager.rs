use crate::core::Lock;
use crate::core::common::injectable_scripts::InjectableScriptTree;

/// Owns enabled interventions and their independent script settings.
/// Lock state protects weakening or disabling any configured intervention.
#[derive(Debug, Clone)]
pub struct ResistanceModeManager {
    /// The script tree for resistance scripts
    pub script_tree: InjectableScriptTree,
    /// Whether resistance mode is enabled
    pub enabled: bool,
    /// Lock state for resistance mode
    pub lock: Lock,
    /// Store scripts version
    pub scripts_version: Option<String>,
    /// Set when the last catalog load failed (missing/corrupt `scripts-data.json`).
    /// Used by the enforcement-status computation so the UI can distinguish
    /// "no scripts installed" from "scripts installed but unreadable".
    pub scripts_load_error: Option<String>,
}

impl ResistanceModeManager {
    pub fn new() -> Self {
        Self {
            script_tree: InjectableScriptTree::default(),
            enabled: false,
            lock: Lock::default(),
            scripts_version: None,
            scripts_load_error: None,
        }
    }

    /// Checks if the resistance mode is locked.
    ///
    /// For `ActiveUntilLockEnd` locks, this properly checks the end time against
    /// the current time. Expired locks return `false` but the stale state remains
    /// in memory until the next save (see `save_resistance_mode_settings`).
    pub fn is_locked(&self) -> bool {
        match self.lock.get_state() {
            crate::core::LockState::Inactive => false,
            crate::core::LockState::SoftUnlockable => true,
            crate::core::LockState::ActiveDuringSchedule => true,
            crate::core::LockState::ActiveUntilLockEnd { end } => {
                #[cfg(test)]
                if crate::trusted_time::current_interval().is_none() {
                    return chrono::Local::now() <= end;
                }
                crate::trusted_time::current_interval()
                    .is_none_or(|range| range.earliest <= end.with_timezone(&chrono::Utc))
            }
        }
    }

    /// Sets the lock state. Returns an error if the new lock is not stricter.
    pub fn set_lock(&mut self, lock: Lock) -> Result<(), &'static str> {
        if self.is_locked() && !lock_is_stricter_than(&lock, &self.lock) {
            return Err("Cannot weaken an active lock");
        }
        self.lock = lock;
        Ok(())
    }

}

impl Default for ResistanceModeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to check if a new lock is stricter than the current one.
fn lock_is_stricter_than(new_lock: &Lock, current_lock: &Lock) -> bool {
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
    use super::*;
    #[test]
    fn default_has_no_interventions_enabled_globally() {
        let manager = ResistanceModeManager::default();
        assert!(!manager.enabled);
        assert!(!manager.is_locked());
    }
    #[test]
    fn active_lock_cannot_be_weakened() {
        let mut manager = ResistanceModeManager::default();
        let end = chrono::Local::now() + chrono::Duration::hours(2);
        manager.set_lock(Lock::from(crate::core::LockState::ActiveUntilLockEnd { end })).unwrap();
        assert!(manager.set_lock(Lock::default()).is_err());
    }
}
