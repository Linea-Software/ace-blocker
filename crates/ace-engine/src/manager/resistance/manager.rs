use super::punishment::{Punishment, PunishmentIntensity, PunishmentMode};
use crate::core::Lock;
use crate::core::common::injectable_scripts::InjectableScriptTree;

/// Manages resistance mode state including scripts, settings, and punishment
/// configuration.
///
/// v1 beta ships **static** mode only (MGR-24): intensity is a fixed,
/// user-configured level. The dynamic escalation/de-escalation ladder and the
/// tracking pipeline that fed it have been removed.
#[derive(Debug, Clone)]
pub struct ResistanceModeManager {
    punishment: Punishment,

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
    /// Creates a new resistance mode manager in static mode with the given intensity.
    pub fn new_static(intensity: PunishmentIntensity) -> Self {
        Self {
            punishment: Punishment::new_static(intensity),
            script_tree: InjectableScriptTree::default(),
            enabled: false,
            lock: Lock::default(),
            scripts_version: None,
            scripts_load_error: None,
        }
    }

    /// Returns the current intensity.
    pub fn get_intensity(&self) -> PunishmentIntensity {
        self.punishment.get_intensity()
    }

    /// Sets the punishment state.
    pub fn set_punishment(&mut self, punishment: Punishment) {
        self.punishment = punishment;
    }

    /// Returns the punishment structure.
    pub fn get_punishment(&self) -> &Punishment {
        &self.punishment
    }

    /// Returns the punishment mode.
    pub fn get_mode(&self) -> PunishmentMode {
        self.punishment.mode
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
            crate::core::LockState::ActiveUntilLockEnd { end } => chrono::Local::now() <= end,
        }
    }

    /// Returns true if in static mode.
    pub fn is_static(&self) -> bool {
        self.punishment.is_static()
    }

    /// Switches to static mode with the specified intensity. Returns an error if locked.
    pub fn switch_to_static(&mut self, intensity: PunishmentIntensity) -> Result<(), &'static str> {
        if self.is_locked() {
            return Err("Cannot switch mode while locked");
        }
        self.punishment.mode = PunishmentMode::Static { intensity };
        self.punishment.static_intensity = intensity;
        Ok(())
    }

    /// Sets the lock state. Returns an error if the new lock is not stricter.
    pub fn set_lock(&mut self, lock: Lock) -> Result<(), &'static str> {
        if self.is_locked() && !lock_is_stricter_than(&lock, &self.lock) {
            return Err("Cannot weaken an active lock");
        }
        self.lock = lock;
        Ok(())
    }

    /// Sets the intensity directly. Should only be used for manual overrides or testing.
    pub fn set_intensity(&mut self, intensity: PunishmentIntensity) {
        self.punishment.set_intensity(intensity);
    }
}

impl Default for ResistanceModeManager {
    fn default() -> Self {
        Self::new_static(PunishmentIntensity::None)
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
    fn test_static_mode_no_escalation() {
        let mut manager = ResistanceModeManager::new_static(PunishmentIntensity::Medium);
        manager.enabled = true;

        // Static mode intensity is fixed and never changes.
        manager.set_intensity(PunishmentIntensity::Low);
        assert_eq!(manager.get_intensity(), PunishmentIntensity::Low);
    }

    #[test]
    fn test_switch_to_static_sets_intensity() {
        let mut manager = ResistanceModeManager::new_static(PunishmentIntensity::None);
        manager.enabled = true;
        manager
            .switch_to_static(PunishmentIntensity::High)
            .expect("switch should succeed");
        assert_eq!(manager.get_intensity(), PunishmentIntensity::High);
        assert!(manager.is_static());
    }

    #[test]
    fn test_switch_mode_rejected_when_locked() {
        let mut manager = ResistanceModeManager::new_static(PunishmentIntensity::None);
        manager.enabled = true;
        let end = chrono::Local::now() + chrono::Duration::hours(1);
        manager
            .set_lock(Lock::from(crate::core::LockState::ActiveUntilLockEnd {
                end,
            }))
            .expect("lock should be accepted");
        assert!(
            manager.switch_to_static(PunishmentIntensity::High).is_err(),
            "switching mode while locked must be rejected"
        );
    }

    #[test]
    fn test_default_mode_is_static_none() {
        let manager = ResistanceModeManager::default();
        assert!(manager.is_static());
        assert_eq!(manager.get_intensity(), PunishmentIntensity::None);
    }
}
