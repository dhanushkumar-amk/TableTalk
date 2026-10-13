pub mod error;
pub mod models;
pub mod repo;
pub mod seed;

pub use error::DbError;
pub use models::*;
pub use seed::seed_database;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::str::FromStr;
use std::time::Duration;

/// Initialize the SQLite connection pool with production-grade settings:
/// - WAL (Write-Ahead Logging) mode
/// - Enforced foreign keys (PRAGMA foreign_keys = ON)
/// - Normal synchronous mode for optimal balance of safety and performance
/// - 5-second busy timeout to handle concurrent access
/// - Automatic file/folder creation if missing
/// - Embedded schema migrations executed on startup
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, DbError> {
    // If the URL points to a file, ensure parent directory exists
    if let Some(path_str) = database_url.strip_prefix("sqlite://") {
        if !path_str.starts_with(":memory:") && !path_str.is_empty() {
            let path = std::path::Path::new(path_str);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }
    }

    let connection_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(connection_options)
        .await?;

    // Run embedded migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(DbError::Migrate)?;

    Ok(pool)
}

/// Convenience function to create an isolated in-memory pool for testing.
pub async fn init_test_pool() -> Result<SqlitePool, DbError> {
    let connection_options = SqliteConnectOptions::from_str("sqlite::memory:")?
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1) // Single connection keeps in-memory SQLite schema persistent
        .connect_with(connection_options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(DbError::Migrate)?;

    Ok(pool)
}
