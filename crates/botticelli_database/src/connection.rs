//! Database connection utilities.

use crate::DatabaseResult;
use botticelli_error::{DatabaseError, DatabaseErrorKind};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::instrument;

/// Type alias for database connection pool.
pub type DbPool = Pool<ConnectionManager<PgConnection>>;

/// Establish a single connection to the PostgreSQL database.
///
/// Reads the `DATABASE_URL` environment variable to determine the connection string.
///
/// # Errors
///
/// Returns an error if:
/// - `DATABASE_URL` environment variable is not set
/// - Connection to the database fails
#[instrument(name = "database.establish_connection")]
pub fn establish_connection() -> DatabaseResult<PgConnection> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        tracing::error!("DATABASE_URL environment variable not set");
        DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_URL environment variable not set".to_string(),
        ))
    })?;

    tracing::debug!("Connecting to PostgreSQL database");
    PgConnection::establish(&database_url).map_err(|e| {
        tracing::error!(error = %e, "Failed to establish database connection");
        DatabaseError::new(DatabaseErrorKind::Connection(e.to_string()))
    })
}

/// Create a connection pool for PostgreSQL database.
///
/// Reads the `DATABASE_URL` environment variable to determine the connection string.
///
/// # Errors
///
/// Returns an error if:
/// - `DATABASE_URL` environment variable is not set
/// - Pool creation fails
#[instrument(name = "database.create_pool")]
pub fn create_pool() -> DatabaseResult<DbPool> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        tracing::error!("DATABASE_URL environment variable not set");
        DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_URL environment variable not set".to_string(),
        ))
    })?;

    create_pool_from_url(&database_url)
}

/// Create a connection pool from a specific database URL.
///
/// # Errors
///
/// Returns an error if pool creation fails.
#[instrument(name = "database.create_pool_from_url", skip(database_url))]
pub fn create_pool_from_url(database_url: &str) -> DatabaseResult<DbPool> {
    tracing::debug!("Creating PostgreSQL connection pool");
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder().max_size(10).build(manager).map_err(|e| {
        tracing::error!(error = %e, "Failed to create connection pool");
        DatabaseError::new(DatabaseErrorKind::Connection(e.to_string()))
    })
}
