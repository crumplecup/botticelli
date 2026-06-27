//! Environment-based storage initialization.

use botticelli_interface::{BotStorage, BotStorageError, BotStorageResult};
use std::sync::Arc;
use tracing::instrument;

/// Opens the configured storage backend from environment variables.
///
/// Backend selection (in priority order):
/// 1. When the `postgres` feature is active **and** `DATABASE_URL` is set →
///    [`PostgresStorage`](crate::PostgresStorage).
/// 2. When the `redb` feature is active → [`RedbStorage`](crate::RedbStorage)
///    at the path given by `REDB_PATH` (default: `./botticelli.redb`).
///
/// At least one of the `redb` or `postgres` Cargo features must be enabled.
#[instrument]
pub async fn open_storage_from_env() -> BotStorageResult<Arc<dyn BotStorage>> {
    open_storage_from_env_inner().await
}

// Postgres + redb both active: postgres wins when DATABASE_URL is set.
#[cfg(all(feature = "postgres", feature = "redb"))]
async fn open_storage_from_env_inner() -> BotStorageResult<Arc<dyn BotStorage>> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        tracing::info!(%url, "Opening PostgresStorage from DATABASE_URL");
        let storage = crate::PostgresStorage::connect(&url)
            .await
            .map_err(|e| BotStorageError::Config(format!("postgres connect failed: {}", e)))?;
        return Ok(Arc::new(storage));
    }
    open_redb()
}

// Only postgres active.
#[cfg(all(feature = "postgres", not(feature = "redb")))]
async fn open_storage_from_env_inner() -> BotStorageResult<Arc<dyn BotStorage>> {
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        BotStorageError::Config(
            "DATABASE_URL must be set when using the postgres backend".to_string(),
        )
    })?;
    tracing::info!(%url, "Opening PostgresStorage from DATABASE_URL");
    let storage = crate::PostgresStorage::connect(&url)
        .await
        .map_err(|e| BotStorageError::Config(format!("postgres connect failed: {}", e)))?;
    Ok(Arc::new(storage))
}

// Only redb active (default).
#[cfg(all(feature = "redb", not(feature = "postgres")))]
async fn open_storage_from_env_inner() -> BotStorageResult<Arc<dyn BotStorage>> {
    open_redb()
}

#[cfg(feature = "redb")]
fn open_redb() -> BotStorageResult<Arc<dyn BotStorage>> {
    let path = match std::env::var("REDB_PATH") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => std::path::PathBuf::from("./botticelli.redb"),
    };

    tracing::info!(path = %path.display(), "Opening RedbStorage");

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            BotStorageError::Config(format!(
                "could not create redb directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    let storage = crate::RedbStorage::open(&path)?;
    Ok(Arc::new(storage))
}
