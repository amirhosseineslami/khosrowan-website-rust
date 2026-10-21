//! Database layer: connection setup, migrations and seed data.

pub mod seed;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;

use crate::config::anyhow_lite::Result;
use crate::config::Config;

/// Open (and if necessary create) the SQLite database and run migrations.
pub async fn connect(config: &Config) -> Result<SqlitePool> {
    // Make sure the parent directory of the database file exists.
    if let Some(path) = config
        .database_url
        .strip_prefix("sqlite://")
        .and_then(|rest| rest.split('?').next())
    {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }

    let mut options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(10));

    if config.environment.is_production() {
        options = options.synchronous(sqlx::sqlite::SqliteSynchronous::Full);
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| crate::config::anyhow_lite::Error::msg(format!("migration failed: {e}")))?;
    Ok(pool)
}
