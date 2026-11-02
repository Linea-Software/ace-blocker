use crate::core::network_block::{BlockingResult, uri_hash::UriHash};
use crate::db::error::DBError;
use diesel::dsl::count;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::r2d2::PooledConnection;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use std::path::Path;

// Migrations for the site pack content databases.
// This lives in its own folder (migrations_site_packs/) so it is never
// embedded into, or run against, the main engine database.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations_site_packs");

/// A single site pack's content database.
///
/// Each site pack gets its own SQLite file (replacing the per-pack RocksDB
/// directory). `entries` stores the 64-bit XXH3 host hashes, `pack_meta` the
/// pack metadata. This database is unrelated to the main engine database.
pub struct SitePackDB {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl std::fmt::Debug for SitePackDB {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SitePackDB").finish_non_exhaustive()
    }
}

/// XXH3-64 hash of a host name, as stored in `entries.host_hash`.
///
/// Every insert and lookup of a host must go through this function so the
/// hashing convention stays consistent.
pub fn hash_host(host: &str) -> UriHash {
    UriHash::from_text(host)
}

impl SitePackDB {
    pub fn from(db_path: &Path) -> Result<SitePackDB, DBError> {
        // SQLite does not create missing parent directories.
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| DBError::DatabaseConnectionError {
                message: format!("failed to create database directory: {}", error),
            })?;
        }

        let pool = crate::db::sqlite_pool::open_pool(db_path, MIGRATIONS)?;
        Ok(SitePackDB { db_pool: pool })
    }

    /// Fast settings apply exclusively to disposable, newly created staging DBs.
    /// A one-connection pool keeps connection-scoped PRAGMAs consistent.
    pub fn from_staging(db_path: &Path) -> Result<SitePackDB, DBError> {
        if db_path.exists() {
            return Err(DBError::DatabaseConnectionError {
                message: "staging database already exists".to_string(),
            });
        }
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| DBError::DatabaseConnectionError {
                message: format!("failed to create staging directory: {error}"),
            })?;
        }
        let db_pool = crate::db::sqlite_pool::open_pool_with_size(db_path, MIGRATIONS, Some(1))?;
        let db = SitePackDB { db_pool };
        let connection = &mut db.get_db_connection()?;
        diesel::sql_query("PRAGMA journal_mode = MEMORY")
            .execute(connection)
            .map_err(staging_error)?;
        diesel::sql_query("PRAGMA synchronous = OFF")
            .execute(connection)
            .map_err(staging_error)?;
        Ok(db)
    }

    /// Validate the B-tree and return to ordinary journal settings before swap.
    /// Only call this after the final metadata write has completed.
    pub fn finish_staging(&self, db_path: &Path) -> Result<(), DBError> {
        {
            let connection = &mut self.get_db_connection()?;
            let integrity: IntegrityRow = diesel::sql_query("PRAGMA quick_check")
                .get_result(connection)
                .map_err(staging_error)?;
            if integrity.result != "ok" {
                return Err(DBError::DatabaseConnectionError {
                    message: format!("staging integrity check failed: {}", integrity.result),
                });
            }
            diesel::sql_query("PRAGMA synchronous = FULL")
                .execute(connection)
                .map_err(staging_error)?;
            diesel::sql_query("PRAGMA journal_mode = DELETE")
                .execute(connection)
                .map_err(staging_error)?;
        }
        std::fs::File::open(db_path)
            .and_then(|file| file.sync_all())
            .map_err(|error| DBError::DatabaseConnectionError {
                message: format!("failed to sync staging database: {error}"),
            })
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

    pub fn get_entry_count(&self) -> Result<i32, DBError> {
        use crate::schema_site_packs::pack_meta::table;

        let conn = &mut self.get_db_connection()?;

        let pack_meta = table
            .select(PackMetaModel::as_select())
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "pack_meta".to_string(),
                message: error.to_string(),
            })?;

        Ok(pack_meta.entry_count)
    }

    /// Removes all entries and the pack metadata row. Used when rebuilding
    /// a pack from scratch.
    pub fn clear(&self) -> Result<(), DBError> {
        use crate::schema_site_packs::{entries, pack_meta};

        let conn = &mut self.get_db_connection()?;
        diesel::delete(entries::table)
            .execute(conn)
            .map_err(|error| DBError::DeletionError {
                table: "entries".to_string(),
                message: error.to_string(),
            })?;
        diesel::delete(pack_meta::table)
            .execute(conn)
            .map_err(|error| DBError::DeletionError {
                table: "pack_meta".to_string(),
                message: error.to_string(),
            })?;
        Ok(())
    }

    /// Inserts host hashes into `entries`, ignoring duplicates (via
    /// `INSERT OR IGNORE`).
    pub fn insert_entries(&self, host_hashes: &[UriHash]) -> Result<(), DBError> {
        use crate::schema_site_packs::entries;

        let rows: Vec<EntryModel> = host_hashes
            .iter()
            .map(|host_hash| EntryModel {
                host_hash: uri_hash_to_db(host_hash),
            })
            .collect();

        // SQLite limits bound variables per statement (999 on older builds).
        // Group multiple 500-row INSERTs in one transaction rather than
        // autocommitting each SQL statement.
        const ROWS_PER_STATEMENT: usize = 500;
        let conn = &mut self.get_db_connection()?;
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            for chunk in rows.chunks(ROWS_PER_STATEMENT) {
                diesel::insert_or_ignore_into(entries::table)
                    .values(chunk)
                    .execute(conn)?;
            }
            Ok(())
        })
        .map_err(|error| DBError::InsertionError {
            table: "entries".to_string(),
            message: error.to_string(),
        })?;
        Ok(())
    }

    /// Number of rows currently stored in `entries`.
    pub fn count_entries(&self) -> Result<i64, DBError> {
        use crate::schema_site_packs::entries;

        let conn = &mut self.get_db_connection()?;
        entries::table
            .select(count(entries::host_hash))
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "entries".to_string(),
                message: error.to_string(),
            })
    }

    /// Returns the blocking result for a previously computed host hash.
    pub fn blocking_result(&self, uri_hash: &UriHash) -> Result<BlockingResult, DBError> {
        use crate::schema_site_packs::entries;

        let conn = &mut self.get_db_connection()?;
        let matches: i64 = entries::table
            .filter(entries::host_hash.eq(uri_hash_to_db(uri_hash)))
            .count()
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "entries".to_string(),
                message: error.to_string(),
            })?;
        Ok(if matches > 0 {
            BlockingResult::Blocked
        } else {
            BlockingResult::NotBlocked
        })
    }

    /// Inserts the pack metadata row. `entry_count` must equal the number of
    /// rows in `entries` (enforced by a database trigger).
    pub fn insert_pack_meta(&self, pack_meta: NewPackMetaModel) -> Result<(), DBError> {
        use crate::schema_site_packs::pack_meta;

        let conn = &mut self.get_db_connection()?;
        diesel::insert_into(pack_meta::table)
            .values(pack_meta)
            .execute(conn)
            .map_err(|error| DBError::InsertionError {
                table: "pack_meta".to_string(),
                message: error.to_string(),
            })?;
        Ok(())
    }
}

#[derive(diesel::QueryableByName)]
struct IntegrityRow {
    #[diesel(sql_type = diesel::sql_types::Text, column_name = "quick_check")]
    result: String,
}

fn staging_error(error: diesel::result::Error) -> DBError {
    DBError::DatabaseConnectionError {
        message: format!("staging database operation failed: {error}"),
    }
}

fn uri_hash_to_db(uri_hash: &UriHash) -> i64 {
    *uri_hash.as_ref() as i64
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema_site_packs::entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct EntryModel {
    pub host_hash: i64,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema_site_packs::pack_meta)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PackMetaModel {
    pub id: String,
    pub format_version: String,
    pub name: Option<String>,
    pub entry_count: i32,
    pub built_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema_site_packs::pack_meta)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPackMetaModel {
    pub id: String,
    pub format_version: String,
    pub name: Option<String>,
    pub entry_count: i32,
    pub built_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_site_packs::{entries, pack_meta};
    use diesel_migrations::MigrationHarness;

    fn test_connection() -> SqliteConnection {
        let mut connection =
            SqliteConnection::establish(":memory:").expect("in-memory connection failed");
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("site pack migrations failed");
        connection
    }

    fn insert_entries(connection: &mut SqliteConnection, host_hashes: &[i64]) {
        let rows: Vec<EntryModel> = host_hashes
            .iter()
            .map(|host_hash| EntryModel {
                host_hash: *host_hash,
            })
            .collect();

        diesel::insert_into(entries::table)
            .values(&rows)
            .execute(connection)
            .expect("failed to insert entries");
    }

    fn valid_pack_meta(entry_count: i32) -> NewPackMetaModel {
        NewPackMetaModel {
            id: "pack-adult-free".to_string(),
            format_version: "1.0".to_string(),
            name: Some("Adult Websites — Free".to_string()),
            entry_count,
            built_at: "2026-08-21T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn staging_settings_are_transient_and_database_reopens_normally() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("staged.db");
        let db = SitePackDB::from_staging(&path).unwrap();
        db.insert_entries(&[hash_host("example.com")]).unwrap();
        db.insert_pack_meta(valid_pack_meta(1)).unwrap();
        db.finish_staging(&path).unwrap();
        drop(db);
        let reopened = SitePackDB::from(&path).unwrap();
        assert_eq!(reopened.get_entry_count().unwrap(), 1);
        assert!(
            reopened
                .blocking_result(&hash_host("example.com"))
                .unwrap()
                .is_blocked()
        );
        assert!(SitePackDB::from_staging(&path).is_err());
    }

    #[test]
    fn migrations_create_expected_tables() {
        let mut connection = test_connection();

        insert_entries(&mut connection, &[1, 2, 3]);

        diesel::insert_into(pack_meta::table)
            .values(&valid_pack_meta(3))
            .execute(&mut connection)
            .expect("pack_meta insert with matching entry_count should succeed");

        let stored_meta: PackMetaModel = pack_meta::table
            .select(PackMetaModel::as_select())
            .first(&mut connection)
            .expect("pack_meta should contain one row");

        assert_eq!(stored_meta.id, "pack-adult-free");
        assert_eq!(stored_meta.entry_count, 3);
        assert_eq!(stored_meta.name.as_deref(), Some("Adult Websites — Free"));
    }

    #[test]
    fn pack_meta_rejects_entry_count_mismatch_on_insert() {
        let mut connection = test_connection();

        insert_entries(&mut connection, &[1, 2, 3]);

        let result = diesel::insert_into(pack_meta::table)
            .values(&valid_pack_meta(2))
            .execute(&mut connection);

        assert!(result.is_err(), "entry_count mismatch must be rejected");
    }

    #[test]
    fn pack_meta_rejects_entry_count_mismatch_on_update() {
        let mut connection = test_connection();

        insert_entries(&mut connection, &[1, 2, 3]);
        diesel::insert_into(pack_meta::table)
            .values(&valid_pack_meta(3))
            .execute(&mut connection)
            .expect("valid pack_meta insert should succeed");

        let result = diesel::update(pack_meta::table)
            .set(pack_meta::entry_count.eq(4))
            .execute(&mut connection);

        assert!(result.is_err(), "entry_count mismatch must be rejected");
    }

    #[test]
    fn entries_rejects_duplicate_host_hash() {
        let mut connection = test_connection();

        insert_entries(&mut connection, &[42]);

        let duplicate = vec![EntryModel { host_hash: 42 }];
        let result = diesel::insert_into(entries::table)
            .values(&duplicate)
            .execute(&mut connection);

        assert!(result.is_err(), "duplicate host_hash must be rejected");
    }
}
