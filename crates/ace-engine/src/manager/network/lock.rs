use crate::core::{Lock, LockState, NetworkBlockId};

use crate::db::models::NetworkBlockLockModel;

use super::{NetworkManager, error::NetworkManagerOpErr};

fn timed_upgrade_shortens_soft_cooldown(
    current: LockState,
    proposed: LockState,
    latest_trusted_now: chrono::DateTime<chrono::Utc>,
) -> bool {
    matches!(
        (current, proposed),
        (LockState::SoftUnlockable, LockState::ActiveUntilLockEnd { end })
            if end.with_timezone(&chrono::Utc) <
                latest_trusted_now + chrono::Duration::seconds(
                    crate::core::common::unlock_challenge::UnlockChallenge::COOLDOWN_SECONDS
                )
    )
}

impl NetworkManager {
    pub fn get_lock_of(
        &self,
        network_block_id: &NetworkBlockId,
    ) -> Result<&Lock, NetworkManagerOpErr> {
        Ok(self
            .get_network_block_from(network_block_id)
            .ok_or(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            })?
            .get_lock())
    }

    /// Rejects non-stricter changes while the current lock is active.
    /// Redeeming a soft unlock uses a separate challenge-verified path.
    pub fn set_lock(
        &mut self,
        network_block_id: &NetworkBlockId,
        new_lock: Lock,
    ) -> Result<(), NetworkManagerOpErr> {
        let now = crate::trusted_time::locking_now();
        let network_block = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        let current_lock = network_block.get_lock();

        // A timed upgrade that expires before the soft-unlock reflection
        // cooldown is over is not actually stricter: it would bypass the
        // mandatory five minutes without redeeming the challenge.
        if let Some(interval) = crate::trusted_time::current_interval()
            && timed_upgrade_shortens_soft_cooldown(
                current_lock.get_state(),
                new_lock.get_state(),
                interval.latest,
            )
        {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }
        if network_block.is_locked(&now) && !new_lock.is_stricter_than(current_lock) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }

        self.set_lock_unchecked(network_block_id, new_lock)
    }

    /// Redeems a verified soft-unlock challenge: removes a `SoftUnlockable`
    /// lock (API-001).
    ///
    /// This is the **only** path that may weaken a soft lock, and it exists
    /// precisely because the reflection-challenge flow
    /// (`/network-blocks/{id}/unlock/initiate` → cooldown → confirm) has
    /// verified the user's intent — the strictness guard in [`Self::set_lock`]
    /// would otherwise reject the redemption forever, leaving the app's
    /// default lock option un-removable.
    ///
    /// The lock state is re-checked **under the manager write lock**, so a
    /// lock that was escalated (error.g. to `ActiveUntilLockEnd`) while the
    /// challenge was pending is never redeemed.
    pub fn redeem_soft_unlock(
        &mut self,
        network_block_id: &NetworkBlockId,
    ) -> Result<(), NetworkManagerOpErr> {
        let network_block = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        if network_block.get_lock().get_state() != LockState::SoftUnlockable {
            // Not a soft lock (or already unlocked): nothing to redeem, and
            // an escalated lock must never be weakened by a stale challenge.
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }

        self.set_lock_unchecked(network_block_id, Lock::from(LockState::Inactive))
    }

    /// Persists a lock and updates memory without the strictness guard.
    /// Only [`Self::set_lock`] (guard) and [`Self::redeem_soft_unlock`]
    /// (challenge-verified bypass) may call this.
    fn set_lock_unchecked(
        &mut self,
        network_block_id: &NetworkBlockId,
        new_lock: Lock,
    ) -> Result<(), NetworkManagerOpErr> {
        let new_lock_model = NetworkBlockLockModel {
            network_block_id: (*network_block_id).to_string(),
            lock_state: new_lock.get_state().to_string(),
            lock_end: new_lock.get_end_time().map(|end| end.to_rfc3339()),
        };

        self.db
            .set_lock(new_lock_model)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!("failed to set content list lock: {}", error),
            })?;

        let network_block = self.get_mut_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        network_block.set_lock(new_lock);

        Ok(())
    }

    pub(crate) fn load_lock_of(
        db: &crate::db::EngineDB,
        network_block_id: &uuid::Uuid,
    ) -> Result<Lock, NetworkManagerOpErr> {
        let model = db.get_lock_of(network_block_id).map_err(|error| {
            NetworkManagerOpErr::DatabaseError {
                message: error.to_string(),
            }
        })?;

        let cl_lock = Lock::from_raw(model.lock_state, model.lock_end).map_err(|error| {
            NetworkManagerOpErr::InternalError {
                message: error.to_string(),
            }
        })?;

        Ok(cl_lock)
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
    fn a_short_timed_upgrade_cannot_bypass_soft_unlock_delay() {
        let now = chrono::Utc::now();
        let short = LockState::ActiveUntilLockEnd {
            end: (now + chrono::Duration::seconds(1)).with_timezone(&Local),
        };
        let long = LockState::ActiveUntilLockEnd {
            end: (now + chrono::Duration::hours(1)).with_timezone(&Local),
        };
        assert!(timed_upgrade_shortens_soft_cooldown(
            LockState::SoftUnlockable,
            short,
            now
        ));
        assert!(!timed_upgrade_shortens_soft_cooldown(
            LockState::SoftUnlockable,
            long,
            now
        ));
        assert!(!timed_upgrade_shortens_soft_cooldown(
            LockState::Inactive,
            short,
            now
        ));
    }

    #[test]
    fn test_set_lock() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("My Block List".into())
            .unwrap();

        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, lock).unwrap();

        let lock_result = ctx.manager.get_lock_of(&list_id).unwrap();
        assert!(
            lock_result.is_active(
                ctx.manager
                    .get_network_block_from(&list_id)
                    .unwrap()
                    .get_schedule(),
                &Local::now()
            )
        );
        assert_eq!(
            lock_result.get_state(),
            LockState::ActiveUntilLockEnd { end: lock_end_time }
        );
        assert!(lock_result.get_end_time().is_some());
    }

    #[test]
    fn test_update_lock() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Update Lock List".into())
            .unwrap();

        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let initial_lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, initial_lock).unwrap();

        let lock_result = ctx.manager.get_lock_of(&list_id).unwrap();
        assert_eq!(
            lock_result.get_state(),
            LockState::ActiveUntilLockEnd { end: lock_end_time }
        );

        // Update lock: UntilLockEnd (stricter)
        let lock_end_time_new = Local::now() + chrono::Duration::days(1);
        let new_lock = Lock::from(LockState::ActiveUntilLockEnd {
            end: lock_end_time_new,
        });
        ctx.manager.set_lock(&list_id, new_lock).unwrap();

        let lock_result = ctx.manager.get_lock_of(&list_id).unwrap();
        assert_eq!(
            lock_result.get_state(),
            LockState::ActiveUntilLockEnd {
                end: lock_end_time_new
            }
        );
        assert!(lock_result.get_end_time().is_some());
    }

    #[test]
    fn test_prevent_lock_downgrade() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Downgrade Lock List".into())
            .unwrap();

        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        let initial_lock = Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time });
        ctx.manager.set_lock(&list_id, initial_lock).unwrap();

        let downgrade_lock = Lock::from(LockState::ActiveDuringSchedule);

        // This should fail
        let result = ctx.manager.set_lock(&list_id, downgrade_lock);
        assert!(result.is_err());

        // Should still be UntilLockEnd
        let lock_result = ctx.manager.get_lock_of(&list_id).unwrap();
        assert_eq!(
            lock_result.get_state(),
            LockState::ActiveUntilLockEnd { end: lock_end_time }
        );
        assert!(lock_result.get_end_time().is_some());
    }

    #[test]
    fn test_set_lock_list_not_found() {
        let mut ctx = setup_test_manager();
        let lock = Lock::from(LockState::ActiveDuringSchedule);

        let result = ctx
            .manager
            .set_lock(&NetworkBlockId::from(uuid::Uuid::now_v7()), lock);
        assert!(result.is_err());
    }

    #[test]
    fn test_redeem_soft_unlock_removes_a_soft_lock() {
        // API-001 regression: the app's default lock option must be
        // removable through the verified challenge flow.
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Soft Unlock List".into())
            .unwrap();
        ctx.manager
            .set_lock(&list_id, Lock::from(LockState::SoftUnlockable))
            .unwrap();

        // A plain set_lock(Inactive) must still be refused (guard intact)…
        let result = ctx
            .manager
            .set_lock(&list_id, Lock::from(LockState::Inactive));
        assert!(result.is_err(), "the strictness guard must stay intact");

        // …but the challenge-verified redemption succeeds and persists.
        ctx.manager.redeem_soft_unlock(&list_id).unwrap();
        assert_eq!(
            ctx.manager.get_lock_of(&list_id).unwrap().get_state(),
            LockState::Inactive
        );
        let model = ctx.manager.db.get_lock_of(&list_id).unwrap();
        assert_eq!(model.lock_state, "inactive");
    }

    #[test]
    fn test_redeem_soft_unlock_never_weakens_an_escalated_lock() {
        let mut ctx = setup_test_manager();

        let list_id = ctx
            .manager
            .create_network_block("Escalated List".into())
            .unwrap();
        ctx.manager
            .set_lock(&list_id, Lock::from(LockState::SoftUnlockable))
            .unwrap();

        // The user escalates while a challenge is pending.
        let lock_end_time = Local::now() + chrono::Duration::hours(1);
        ctx.manager
            .set_lock(
                &list_id,
                Lock::from(LockState::ActiveUntilLockEnd { end: lock_end_time }),
            )
            .unwrap();

        // The stale challenge must NOT redeem the escalated lock.
        let result = ctx.manager.redeem_soft_unlock(&list_id);
        assert!(result.is_err(), "an escalated lock must never be redeemed");
        assert_eq!(
            ctx.manager.get_lock_of(&list_id).unwrap().get_state(),
            LockState::ActiveUntilLockEnd { end: lock_end_time }
        );

        // Redeeming a block with no soft lock (default inactive) is refused.
        let unlocked_id = ctx.manager.create_network_block("Unlocked".into()).unwrap();
        assert!(ctx.manager.redeem_soft_unlock(&unlocked_id).is_err());
    }
}
