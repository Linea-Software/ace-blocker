pub mod account_repo;
pub mod backup_restore_repo;
pub mod blocked_attempts_repo;
pub mod exceptions_repo;
pub mod lock_repo;
pub mod models;
pub mod network_block_repo;
pub mod network_block_snapshot_repo;
pub mod network_block_to_site_pack_repo;
pub mod proxy_exclusion_repo;
pub mod resistance_mode_repo;
pub mod safe_search_repo;
pub mod schedule_repo;
pub mod script_state_repo;
pub mod settings_repo;
pub mod site_pack_repo;
pub mod site_pack_source_repo;
pub(crate) mod sqlite_pool;
pub mod user_defined_rule_repo;

use crate::schema_main_db::network_block_to_site_pack;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::r2d2::PooledConnection;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use models::*;
use std::{collections::HashMap, path::Path};
// migrations is simply the ace-engine/migrations/ folder from the perspective of cargo
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Legacy naive timestamps were local wall times. Resolve the civil instant
/// in the policy's initial pinned zone, choosing the *later* instant for a
/// repeated DST hour. A nonexistent local hour conservatively advances to a
/// representable local time (rather than shortening the lock).
fn legacy_deadline_as_absolute(
    raw: &str,
    timezone: chrono_tz::Tz,
) -> Result<Option<String>, DBError> {
    use chrono::{LocalResult, TimeZone};
    if chrono::DateTime::parse_from_rfc3339(raw).is_ok() {
        return Ok(None);
    }
    let civil =
        chrono::NaiveDateTime::parse_from_str(raw, crate::core::common::constants::DATETIME_FORMAT)
            .map_err(|error| DBError::MigrationError {
                message: format!("Invalid legacy lock date {raw:?}: {error}"),
            })?;
    let choose_latest = |date| match timezone.from_local_datetime(&date) {
        LocalResult::Single(instant) => Some(instant),
        LocalResult::Ambiguous(first, second) => Some(first.max(second)),
        LocalResult::None => None,
    };
    let instant = choose_latest(civil)
        .or_else(|| choose_latest(civil + chrono::Duration::hours(3)))
        .or_else(|| choose_latest(civil + chrono::Duration::days(2)))
        .ok_or_else(|| DBError::MigrationError {
            message: format!("Unrepresentable legacy lock deadline {raw:?} in {timezone}"),
        })?;
    Ok(Some(instant.to_rfc3339()))
}

pub struct EngineDB {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

pub mod error;
pub use error::DBError;

impl EngineDB {
    pub fn from(db_path: &Path) -> Result<EngineDB, DBError> {
        let pool = sqlite_pool::open_pool(db_path, MIGRATIONS)?;
        let db = EngineDB { db_pool: pool };
        let system_zone = crate::core::common::schedule::system_timezone();
        db.pin_legacy_schedule_timezone(system_zone.name())?;
        db.pin_legacy_timed_lock_deadlines(system_zone)?;
        Ok(db)
    }

    /// Capture the install-time IANA zone for policies written before the
    /// timezone column existed, before a manager can load those rows.
    /// Conditional UPDATE makes the migration idempotent and leaves existing
    /// pinned zones unchanged on later reboots / Windows timezone changes.
    fn pin_legacy_schedule_timezone(&self, tz: &str) -> Result<(), DBError> {
        use crate::schema_main_db::network_blocks::dsl::{network_blocks, schedule_timezone};
        let mut connection = self.get_db_connection()?;
        diesel::update(network_blocks.filter(schedule_timezone.eq("__legacy_system_timezone__")))
            .set(schedule_timezone.eq(tz))
            .execute(&mut connection)
            .map_err(|error| DBError::MigrationError {
                message: format!("Could not pin legacy schedule IANA timezone: {error}"),
            })?;
        Ok(())
    }

    /// Normalize older timezone-less lock timestamps exactly once, while
    /// preserving the original civil zone captured during this upgrade.
    /// All subsequent loads use RFC3339 absolute deadlines, so a Windows
    /// timezone change cannot make an existing legacy lock expire earlier.
    fn pin_legacy_timed_lock_deadlines(&self, fallback_zone: chrono_tz::Tz) -> Result<(), DBError> {
        use crate::schema_main_db::{
            network_block_locks as locks, network_blocks as blocks,
            resistance_mode_settings as resistance,
        };
        let mut connection = self.get_db_connection()?;
        let rows: Vec<(String, Option<String>, String)> = locks::table
            .inner_join(blocks::table)
            .select((
                locks::network_block_id,
                locks::lock_end,
                blocks::schedule_timezone,
            ))
            .load(&mut connection)
            .map_err(|error| DBError::MigrationError {
                message: format!("Could not read legacy lock dates: {error}"),
            })?;
        for (id, raw, name) in rows {
            let Some(raw) = raw else {
                continue;
            };
            let zone = name
                .parse::<chrono_tz::Tz>()
                .map_err(|error| DBError::MigrationError {
                    message: format!("Invalid pinned timezone {name}: {error}"),
                })?;
            if let Some(absolute) = legacy_deadline_as_absolute(&raw, zone)? {
                diesel::update(locks::table.find(&id))
                    .set(locks::lock_end.eq(absolute))
                    .execute(&mut connection)
                    .map_err(|error| DBError::MigrationError {
                        message: format!("Could not normalize legacy network lock {id}: {error}"),
                    })?;
            }
        }

        let mode = resistance::table
            .select(resistance::lock_end)
            .first::<Option<String>>(&mut connection)
            .optional()
            .map_err(|error| DBError::MigrationError {
                message: format!("Could not read legacy Resistance Mode lock: {error}"),
            })?;
        if let Some(Some(raw)) = mode {
            if let Some(absolute) = legacy_deadline_as_absolute(&raw, fallback_zone)? {
                diesel::update(resistance::table)
                    .set(resistance::lock_end.eq(absolute))
                    .execute(&mut connection)
                    .map_err(|error| DBError::MigrationError {
                        message: format!(
                            "Could not normalize legacy Resistance Mode lock: {error}"
                        ),
                    })?;
            }
        }
        Ok(())
    }

    pub fn get_db_connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, DBError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|error| DBError::DatabaseConnectionError {
                message: format!("failed to get connection from pool: {}", error),
            })?;

        Ok(conn)
    }

    pub fn get_network_blocks_to_site_packs(
        &self,
    ) -> Result<HashMap<String, Vec<String>>, DBError> {
        let mut conn = self.get_db_connection()?;

        let join_pairs: Vec<NetworkBlockToSitePackModel> = network_block_to_site_pack::table
            .select((
                network_block_to_site_pack::network_block_id,
                network_block_to_site_pack::site_pack_id,
            ))
            .load::<NetworkBlockToSitePackModel>(&mut conn)
            .map_err(|error| DBError::DataFetchError {
                table: "network_block_to_site_pack".to_string(),
                message: format!(
                    "Failed to load network blocks connections to site packs: {}",
                    error
                ),
            })?;

        let mut mapping = HashMap::<String, Vec<String>>::new();
        for pair in join_pairs {
            mapping
                .entry(pair.network_block_id)
                .or_default()
                .push(pair.site_pack_id);
        }

        Ok(mapping)
    }
}

#[cfg(test)]
mod migration_seed_tests {
    use super::*;
    use crate::core::{NetworkBlockId, SitePackId};
    use crate::manager::{AccountManager, NetworkManager};
    use crate::settings::Settings;
    use std::sync::Arc;

    // The four default blocks seeded by 0021; ids must never change (the app
    // recognises them via ace-app/src/lib/onboarding/default-blocks.ts).
    const ADULT_BLOCK: &str = "01950000-0000-7000-8000-000000000001";
    const SOCIAL_BLOCK: &str = "01950000-0000-7000-8000-000000000002";
    const GAMBLING_BLOCK: &str = "01950000-0000-7000-8000-000000000003";
    const STREAMING_BLOCK: &str = "01950000-0000-7000-8000-000000000004";

    // The FREE site packs the baseline connects the pack-backed blocks to. These are
    // the canonical catalog IDs from ace-app/src/lib/data/site-packs.json,
    // seeded with id == remote_id so the content updater recognises and never
    // duplicates them (get_site_pack_by_remote_id → "already exists, skipping").
    const ADULT_PACK: &str = "pack-adult-free";
    const SOCIAL_PACK: &str = "pack-social-free";
    const GAMBLING_PACK: &str = "pack-gambling-free";
    const STREAMING_PACK: &str = "pack-streaming-free";

    fn establish_connection(db_path: &Path) -> SqliteConnection {
        SqliteConnection::establish(db_path.to_str().unwrap())
            .expect("failed to open the test database")
    }

    fn load_all_rules_for(conn: &mut SqliteConnection, block_id: &str) -> Vec<String> {
        use crate::schema_main_db::user_defined_rules::dsl::*;
        user_defined_rules
            .filter(network_block_id.eq(block_id))
            .select(rule)
            .load::<String>(conn)
            .expect("failed to load user defined rules")
    }

    fn count_pack_connections(conn: &mut SqliteConnection, block_id: &str, pack_id: &str) -> i64 {
        use crate::schema_main_db::network_block_to_site_pack::dsl::*;
        network_block_to_site_pack
            .filter(network_block_id.eq(block_id))
            .filter(site_pack_id.eq(pack_id))
            .count()
            .get_result::<i64>(conn)
            .expect("failed to count pack connections")
    }

    /// Unit tests must never touch the real ProgramData sites dir
    /// (TESTINFRA-01); point the process-global override at the shared test
    /// dir like the other manager unit tests do.
    fn set_test_sites_dir() {
        let sandbox = Settings::shared_test_sites_dir();
        Settings::set_test_sites_dir(sandbox.clone());
        // A manager opened by a migration test also checks the crash-recovery
        // journal. Never read the root-owned live installation on Linux: that
        // would make this hermetic test depend on production permissions.
        Settings::set_test_resistance_scripts_dir(sandbox.join("resistance_scripts"));
    }

    fn to_block_id(block_id: &str) -> NetworkBlockId {
        NetworkBlockId::from(uuid::Uuid::parse_str(block_id).unwrap())
    }

    /// The consolidated fresh baseline seeds Free, category-backed policies.
    #[test]
    fn legacy_lock_deadlines_are_migrated_to_absolute_utc_once() {
        let temp = tempfile::tempdir().unwrap();
        let db = EngineDB::from(&temp.path().join("legacy-locks.sqlite")).unwrap();
        let mut conn = db.get_db_connection().unwrap();
        use crate::schema_main_db::network_block_locks as locks;
        use crate::schema_main_db::network_blocks as blocks;
        use crate::schema_main_db::resistance_mode_settings as mode;
        diesel::update(blocks::table.find(ADULT_BLOCK))
            .set(blocks::schedule_timezone.eq("Europe/Rome"))
            .execute(&mut conn)
            .unwrap();
        diesel::update(locks::table.find(ADULT_BLOCK))
            .set((
                locks::lock_state.eq("active_until_lock_end"),
                locks::lock_end.eq(Some("2026-10-25T02:30:00")),
            ))
            .execute(&mut conn)
            .unwrap();
        diesel::update(mode::table)
            .set((
                mode::enabled.eq(true),
                mode::lock_state.eq("active_until_lock_end"),
                mode::lock_end.eq(Some("2026-03-29T02:30:00")),
            ))
            .execute(&mut conn)
            .unwrap();
        drop(conn);

        db.pin_legacy_timed_lock_deadlines("Europe/Rome".parse().unwrap())
            .unwrap();
        let row = db
            .get_lock_of(&uuid::Uuid::parse_str(ADULT_BLOCK).unwrap())
            .unwrap();
        // Fall back chooses the *later* 02:30 on Rome's DST transition.
        assert_eq!(row.lock_end.as_deref(), Some("2026-10-25T02:30:00+01:00"));
        let resistance = db.get_resistance_mode_settings().unwrap();
        // Spring gap is advanced, never interpreted as an earlier instant.
        assert!(
            resistance
                .lock_end
                .as_ref()
                .is_some_and(|raw| raw.contains("+02:00"))
        );

        // A later Windows timezone change cannot rewrite old absolute dates.
        db.pin_legacy_timed_lock_deadlines("America/New_York".parse().unwrap())
            .unwrap();
        let reloaded = db
            .get_lock_of(&uuid::Uuid::parse_str(ADULT_BLOCK).unwrap())
            .unwrap();
        assert_eq!(reloaded.lock_end, row.lock_end);
    }

    #[test]
    fn legacy_schedule_timezone_is_pinned_once_on_upgrade() {
        let temp = tempfile::tempdir().unwrap();
        let db = EngineDB::from(&temp.path().join("upgrade.sqlite")).unwrap();
        let mut conn = db.get_db_connection().unwrap();
        // Test the conditional upgrade with a specific named zone so the
        // test does not depend on the host running these fixtures.
        diesel::update(crate::schema_main_db::network_blocks::table)
            .set(
                crate::schema_main_db::network_blocks::schedule_timezone
                    .eq("__legacy_system_timezone__"),
            )
            .execute(&mut conn)
            .unwrap();
        drop(conn);

        db.pin_legacy_schedule_timezone("Europe/Rome").unwrap();
        let rows = db.get_all_network_blocks().unwrap();
        assert!(!rows.is_empty());
        assert!(
            rows.iter()
                .all(|block| block.schedule_timezone == "Europe/Rome")
        );

        db.pin_legacy_schedule_timezone("America/New_York").unwrap();
        let reloaded = db.get_all_network_blocks().unwrap();
        assert!(
            reloaded
                .iter()
                .all(|block| block.schedule_timezone == "Europe/Rome")
        );
    }

    #[test]
    fn seeded_default_blocks_use_free_packs_without_custom_rules() {
        set_test_sites_dir();
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");
        let db = Arc::new(EngineDB::from(&db_path).unwrap());
        let mut conn = establish_connection(&db_path);

        let starters = [
            (ADULT_BLOCK, ADULT_PACK),
            (SOCIAL_BLOCK, SOCIAL_PACK),
            (GAMBLING_BLOCK, GAMBLING_PACK),
            (STREAMING_BLOCK, STREAMING_PACK),
        ];
        let packs = db.get_all_site_packs().unwrap();
        for (block_id, pack_id) in starters {
            assert!(load_all_rules_for(&mut conn, block_id).is_empty());
            let pack = packs
                .iter()
                .find(|pack| pack.id == pack_id)
                .expect("Free preset pack must exist");
            assert_eq!(pack.remote_id.as_deref(), Some(pack_id));
            assert!(!pack.requires_pro);
            assert_eq!(count_pack_connections(&mut conn, block_id, pack_id), 1);
        }

        let account_manager = AccountManager::from(db.clone()).unwrap();
        let manager = NetworkManager::from(db, account_manager).unwrap();
        for (block_id, pack_id) in starters {
            let block = manager
                .get_network_block_from(&to_block_id(block_id))
                .unwrap();
            assert!(block.has_site_pack(&SitePackId::try_new(pack_id.to_string()).unwrap()));
        }
    }

    #[test]
    fn starter_blocks_are_disabled_and_always_without_redundant_periods() {
        let temp = tempfile::tempdir().unwrap();
        let db = EngineDB::from(&temp.path().join("fresh.sqlite")).unwrap();
        use crate::schema_main_db::network_block_schedule_entries::dsl as schedule;
        use crate::schema_main_db::network_blocks::dsl as blocks;
        let mut conn = db.get_db_connection().unwrap();
        for starter in [ADULT_BLOCK, SOCIAL_BLOCK, GAMBLING_BLOCK, STREAMING_BLOCK] {
            let (enabled, schedule_mode): (bool, String) = blocks::network_blocks
                .filter(blocks::id.eq(starter))
                .select((blocks::enabled, blocks::schedule_mode))
                .first(&mut conn)
                .unwrap();
            assert!(!enabled);
            assert_eq!(schedule_mode, "always");
            let periods = schedule::network_block_schedule_entries
                .filter(schedule::network_block_id.eq(starter))
                .count()
                .get_result::<i64>(&mut conn)
                .unwrap();
            assert_eq!(periods, 0);
        }
    }
}
