//! Database connection utilities.

use crate::DatabaseResult;
use botticelli_error::{DatabaseError, DatabaseErrorKind};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use tracing::instrument;

/// Establish a connection to the PostgreSQL database.
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
