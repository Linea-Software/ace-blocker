//! Shared Diesel connection pool setup, not a shared persistence model.
//!
//! Main engine and per-site-pack databases keep independent migrations and
//! repository methods. Only the identical pool/initial migration plumbing lives here.

use diesel::prelude::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::path::Path;

use super::DBError;

pub(crate) fn open_pool(
    path: &Path,
    migrations: EmbeddedMigrations,
) -> Result<Pool<ConnectionManager<SqliteConnection>>, DBError> {
    open_pool_with_size(path, migrations, None)
}

/// Restrict staging pools to one connection so connection-local PRAGMAs persist.
pub(crate) fn open_pool_with_size(
    path: &Path,
    migrations: EmbeddedMigrations,
    max_size: Option<u32>,
) -> Result<Pool<ConnectionManager<SqliteConnection>>, DBError> {
    let database_path = path
        .to_str()
        .ok_or_else(|| DBError::DatabaseConnectionError {
            message: "provided database path is not a valid string".to_string(),
        })?;
    let manager = ConnectionManager::<SqliteConnection>::new(database_path);
    let mut builder = Pool::builder();
    if let Some(max_size) = max_size {
        builder = builder.max_size(max_size);
    }
    let pool = builder
        .build(manager)
        .map_err(|error| DBError::DatabaseConnectionError {
            message: format!("failed to establish connection pool: {error}"),
        })?;

    {
        let mut connection = pool
            .get()
            .map_err(|error| DBError::DatabaseConnectionError {
                message: format!("failed to get connection from pool: {error}"),
            })?;
        connection
            .run_pending_migrations(migrations)
            .map_err(|error| DBError::MigrationError {
                message: error.to_string(),
            })?;
    }

    Ok(pool)
}
