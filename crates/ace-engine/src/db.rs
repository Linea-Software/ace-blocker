pub mod account_repo;
pub mod blocked_attempts_repo;
pub mod exceptions_repo;
pub mod lock_repo;
pub mod models;
pub mod network_block_repo;
pub mod network_block_to_site_pack_repo;
pub mod proxy_exclusion_repo;
pub mod resistance_mode_repo;
pub mod safe_search_repo;
pub mod schedule_repo;
pub mod script_state_repo;
pub mod settings_repo;
pub mod site_pack_repo;
pub mod site_pack_source_repo;
pub mod user_defined_rule_repo;

use crate::schema_main_db::network_block_to_site_pack;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::r2d2::PooledConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use models::*;
use std::{collections::HashMap, path::Path};
// migrations is simply the ace-engine/migrations/ folder from the perspective of cargo
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub struct EngineDB {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

pub mod error;
pub use error::DBError;

impl EngineDB {
    pub fn from(db_path: &Path) -> Result<EngineDB, DBError> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| DBError::DatabaseConnectionError {
                message: "provided database path is not a valid string".to_string(),
            })?;

        let db_manager = ConnectionManager::<SqliteConnection>::new(db_path_str);
        let pool = Pool::builder().build(db_manager).map_err(|error| {
            DBError::DatabaseConnectionError {
                message: format!("failed to establish connection pool: {}", error),
            }
        })?;

        let mut db_connection = pool
            .get()
            .map_err(|error| DBError::DatabaseConnectionError {
                message: format!("failed to get connection from pool: {}", error),
            })?;

        db_connection
            .run_pending_migrations(MIGRATIONS)
            .map_err(|error| DBError::MigrationError {
                message: error.to_string(),
            })?;

        Ok(EngineDB { db_pool: pool })
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
    use crate::core::{BlockingResult, NetworkBlockId, SitePackId};
    use crate::manager::{AccountManager, NetworkManager};
    use crate::settings::Settings;
    use diesel::connection::SimpleConnection;
    use diesel_migrations::MigrationHarness;
    use std::sync::Arc;

    // The four default blocks seeded by 0021; ids must never change (the app
    // recognises them via ace-app/src/lib/onboarding/default-blocks.ts).
    const ADULT_BLOCK: &str = "01950000-0000-7000-8000-000000000001";
    const SOCIAL_BLOCK: &str = "01950000-0000-7000-8000-000000000002";
    const GAMBLING_BLOCK: &str = "01950000-0000-7000-8000-000000000003";
    const STREAMING_BLOCK: &str = "01950000-0000-7000-8000-000000000004";

    // The FREE site packs 0022 connects the pack-backed blocks to. These are
    // the canonical catalog IDs from ace-app/src/lib/data/site-packs.json,
    // seeded with id == remote_id so the content updater recognises and never
    // duplicates them (get_site_pack_by_remote_id → "already exists, skipping").
    const ADULT_PACK: &str = "pack-adult-free";
    const SOCIAL_PACK: &str = "pack-social-free";

    // Number of curated domains 0021 seeds per block.
    const SOCIAL_RULE_COUNT: usize = 12;
    const GAMBLING_RULE_COUNT: usize = 14;
    const STREAMING_RULE_COUNT: usize = 9;

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

    fn load_all_rule_types_for(conn: &mut SqliteConnection, block_id: &str) -> Vec<String> {
        use crate::schema_main_db::user_defined_rules::dsl::*;
        user_defined_rules
            .filter(network_block_id.eq(block_id))
            .select(rule_type)
            .load::<String>(conn)
            .expect("failed to load user defined rule types")
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

    fn count_site_packs(conn: &mut SqliteConnection, pack_id: &str) -> i64 {
        use crate::schema_main_db::site_packs::dsl::*;
        site_packs
            .filter(id.eq(pack_id))
            .count()
            .get_result::<i64>(conn)
            .expect("failed to count site packs")
    }

    /// Applies migrations up to and including 0021 — i.e. the state of a DB
    /// that has "already run 0021" when 0022 arrives.
    fn apply_through_0021(conn: &mut SqliteConnection) {
        loop {
            let applied = conn
                .applied_migrations()
                .expect("failed to read applied migrations");
            if applied.iter().any(|version| version.to_string() == "0021") {
                return;
            }
            conn.run_next_migration(MIGRATIONS)
                .expect("failed to run the next migration");
        }
    }

    fn apply_through_0022(conn: &mut SqliteConnection) {
        loop {
            let applied = conn
                .applied_migrations()
                .expect("failed to read applied migrations");
            if applied.iter().any(|version| version.to_string() == "0022") {
                return;
            }
            conn.run_next_migration(MIGRATIONS)
                .expect("failed to run the next migration");
        }
    }

    /// Unit tests must never touch the real ProgramData sites dir
    /// (TESTINFRA-01); point the process-global override at the shared test
    /// dir like the other manager unit tests do.
    fn set_test_sites_dir() {
        Settings::set_test_sites_dir(Settings::shared_test_sites_dir());
    }

    fn block_id(block_id: &str) -> NetworkBlockId {
        NetworkBlockId::from(uuid::Uuid::parse_str(block_id).unwrap())
    }

    /// Fresh-DB outcome of migration 0022: pack-backed blocks have no local
    /// rules, curated blocks keep the same sites as typed domain rules, and the
    /// free packs exist under the canonical catalog IDs
    /// (pack-adult-free / pack-social-free), and a manager loaded from this DB
    /// resolves everything (no dangling pack references — MGR-01).
    #[test]
    fn seeded_default_blocks_use_packs_and_domain_rules() {
        set_test_sites_dir();
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");

        // A fresh DB runs every migration, including 0022.
        let db = Arc::new(EngineDB::from(&db_path).unwrap());
        let mut conn = establish_connection(&db_path);

        // 1. Pack-backed blocks lost their regex lists entirely.
        assert!(load_all_rules_for(&mut conn, ADULT_BLOCK).is_empty());
        assert!(load_all_rules_for(&mut conn, SOCIAL_BLOCK).is_empty());

        // 2. The curated blocks kept the SAME domain set as typed domain rules.
        // These can be enforced from CONNECT hostname metadata without MITM.
        let gambling_rules = load_all_rules_for(&mut conn, GAMBLING_BLOCK);
        let streaming_rules = load_all_rules_for(&mut conn, STREAMING_BLOCK);
        assert_eq!(gambling_rules.len(), GAMBLING_RULE_COUNT);
        assert_eq!(streaming_rules.len(), STREAMING_RULE_COUNT);
        assert!(gambling_rules.iter().any(|rule| rule == "stake.com"));
        assert!(streaming_rules.iter().any(|rule| rule == "youtube.com"));
        assert!(
            load_all_rule_types_for(&mut conn, GAMBLING_BLOCK)
                .iter()
                .all(|rule_type| rule_type == "domain")
        );
        assert!(
            load_all_rule_types_for(&mut conn, STREAMING_BLOCK)
                .iter()
                .all(|rule_type| rule_type == "domain")
        );

        // 3. The two free packs exist under the canonical IDs (id == remote_id).
        let packs = db.get_all_site_packs().unwrap();
        let adult_pack = packs
            .iter()
            .find(|pack| pack.id == ADULT_PACK)
            .expect("adult content pack must be seeded");
        assert_eq!(adult_pack.remote_id.as_deref(), Some(ADULT_PACK));
        assert!(!adult_pack.requires_pro);
        let social_pack = packs
            .iter()
            .find(|pack| pack.id == SOCIAL_PACK)
            .expect("social media pack must be seeded");
        assert_eq!(social_pack.remote_id.as_deref(), Some(SOCIAL_PACK));
        assert!(!social_pack.requires_pro);

        // 4. The pack connections exist in the join table.
        assert_eq!(
            count_pack_connections(&mut conn, ADULT_BLOCK, ADULT_PACK),
            1
        );
        assert_eq!(
            count_pack_connections(&mut conn, SOCIAL_BLOCK, SOCIAL_PACK),
            1
        );

        // 5. End-to-end through a manager loaded from this DB.
        let account_manager = AccountManager::from(db.clone()).unwrap();
        let manager = NetworkManager::from(db, account_manager).unwrap();

        let gambling_block = manager
            .get_network_block_from(&block_id(GAMBLING_BLOCK))
            .expect("gambling block must load");
        assert_eq!(
            gambling_block.is_content_blocked(&"https://stake.com/".parse::<http::Uri>().unwrap()),
            BlockingResult::Blocked
        );
        assert_eq!(
            gambling_block
                .is_content_blocked(&"https://stake.community/".parse::<http::Uri>().unwrap()),
            BlockingResult::NotBlocked
        );
        assert_eq!(
            gambling_block
                .is_content_blocked(&"https://notstake.com/".parse::<http::Uri>().unwrap()),
            BlockingResult::NotBlocked
        );

        let social_block = manager
            .get_network_block_from(&block_id(SOCIAL_BLOCK))
            .expect("social media block must load");
        // The pack connection resolved in memory instead of being skipped as
        // a dangling reference (MGR-01).
        assert!(social_block.has_site_pack(&SitePackId::from(SOCIAL_PACK.to_string())));
    }

    /// A DB that already ran 0021 upgrades cleanly: the old seeded rows are
    /// replaced with typed domain rules for the curated blocks, and the pack
    /// connections appear. A
    /// re-run of 0022's SQL is idempotent for the seeded rows.
    #[test]
    fn migration_0022_upgrades_cleanly_from_0021() {
        set_test_sites_dir();
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");

        // Simulate a DB that ran only through 0021.
        let mut conn = establish_connection(&db_path);
        apply_through_0021(&mut conn);

        // Pre-upgrade state: the unanchored 0021 seeds.
        let social_rules = load_all_rules_for(&mut conn, SOCIAL_BLOCK);
        assert_eq!(social_rules.len(), SOCIAL_RULE_COUNT);
        assert!(social_rules.iter().any(|rule| rule == "x.com"));
        let gambling_rules = load_all_rules_for(&mut conn, GAMBLING_BLOCK);
        assert_eq!(gambling_rules.len(), GAMBLING_RULE_COUNT);
        assert!(gambling_rules.iter().any(|rule| rule == "stake.com"));
        assert_eq!(
            count_pack_connections(&mut conn, ADULT_BLOCK, ADULT_PACK),
            0
        );

        // Upgrade with the remaining pending migrations (0022).
        conn.run_pending_migrations(MIGRATIONS).unwrap();

        // Post-upgrade state matches the fresh-DB outcome.
        assert!(load_all_rules_for(&mut conn, ADULT_BLOCK).is_empty());
        assert!(load_all_rules_for(&mut conn, SOCIAL_BLOCK).is_empty());
        let gambling_rules = load_all_rules_for(&mut conn, GAMBLING_BLOCK);
        assert_eq!(gambling_rules.len(), GAMBLING_RULE_COUNT);
        assert!(gambling_rules.iter().any(|rule| rule == "stake.com"));
        assert!(
            load_all_rule_types_for(&mut conn, GAMBLING_BLOCK)
                .iter()
                .all(|rule_type| rule_type == "domain")
        );
        assert_eq!(
            count_pack_connections(&mut conn, ADULT_BLOCK, ADULT_PACK),
            1
        );
        assert_eq!(
            count_pack_connections(&mut conn, SOCIAL_BLOCK, SOCIAL_PACK),
            1
        );

        // Re-running 0022's up.sql (the same SQL a migration replay would
        // execute) is idempotent: joins and packs are not duplicated and the
        // curated rules stay typed as domains.
        conn.batch_execute(include_str!(
            "../migrations/0022_seed_default_blocks_site_packs/up.sql"
        ))
        .unwrap();
        assert_eq!(
            count_pack_connections(&mut conn, ADULT_BLOCK, ADULT_PACK),
            1
        );
        assert_eq!(
            count_pack_connections(&mut conn, SOCIAL_BLOCK, SOCIAL_PACK),
            1
        );
        assert_eq!(count_site_packs(&mut conn, ADULT_PACK), 1);
        assert_eq!(count_site_packs(&mut conn, SOCIAL_PACK), 1);
        assert!(
            load_all_rule_types_for(&mut conn, GAMBLING_BLOCK)
                .iter()
                .all(|rule_type| rule_type == "domain")
        );
    }

    #[test]
    fn migration_0023_keeps_existing_blocks_enabled() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");
        let mut conn = establish_connection(&db_path);
        apply_through_0022(&mut conn);

        let existing_block_id = uuid::Uuid::now_v7().to_string();
        conn.batch_execute(&format!(
            "INSERT INTO network_blocks (id, name, safe_search, icon) VALUES ('{existing_block_id}', 'Existing Block', 0, NULL)"
        ))
        .unwrap();

        conn.run_next_migration(MIGRATIONS).unwrap();

        use crate::schema_main_db::network_blocks::dsl::{enabled, id, network_blocks};
        let is_enabled = network_blocks
            .filter(id.eq(existing_block_id))
            .select(enabled)
            .first::<bool>(&mut conn)
            .unwrap();
        assert!(is_enabled);
    }

    /// 0022's down.sql reverses cleanly: joins, domain rules, and the seeded
    /// packs are all removed (the original 0021 regex lists are not restored —
    /// documented in the down.sql itself).
    #[test]
    fn migration_0022_down_reverts_cleanly() {
        set_test_sites_dir();
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_db.sqlite");

        // Fresh DB with every migration applied.
        let _db = EngineDB::from(&db_path).unwrap();
        let mut conn = establish_connection(&db_path);

        // Pre-revert state.
        assert_eq!(
            count_pack_connections(&mut conn, ADULT_BLOCK, ADULT_PACK),
            1
        );
        assert_eq!(count_site_packs(&mut conn, ADULT_PACK), 1);
        assert_eq!(
            load_all_rules_for(&mut conn, GAMBLING_BLOCK).len(),
            GAMBLING_RULE_COUNT
        );

        // 0023 adds the Network Block enabled flag, so revert it first and then 0022.
        conn.revert_last_migration(MIGRATIONS).unwrap();
        conn.revert_last_migration(MIGRATIONS).unwrap();

        // Post-revert state.
        assert_eq!(
            count_pack_connections(&mut conn, ADULT_BLOCK, ADULT_PACK),
            0
        );
        assert_eq!(
            count_pack_connections(&mut conn, SOCIAL_BLOCK, SOCIAL_PACK),
            0
        );
        assert_eq!(count_site_packs(&mut conn, ADULT_PACK), 0);
        assert_eq!(count_site_packs(&mut conn, SOCIAL_PACK), 0);
        for block_id in [ADULT_BLOCK, SOCIAL_BLOCK, GAMBLING_BLOCK, STREAMING_BLOCK] {
            assert!(
                load_all_rules_for(&mut conn, block_id).is_empty(),
                "rules for {block_id} must be gone after the revert"
            );
        }
    }
}
