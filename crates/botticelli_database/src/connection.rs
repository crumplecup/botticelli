//! Database connection utilities.

use crate::DatabaseResult;
use botticelli_error::{DatabaseError, DatabaseErrorKind};
use diesel::pg::PgConnection;
use diesel::prelude::*;

/// Establish a connection to the PostgreSQL database.
///
/// Reads the `DATABASE_URL` environment variable to determine the connection string.
///
/// # Errors
///
/// Returns an error if:
/// - `DATABASE_URL` environment variable is not set
/// - Connection to the database fails
pub fn establish_connection() -> DatabaseResult<PgConnection> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_URL environment variable not set".to_string(),
        ))
    })?;

    PgConnection::establish(&database_url)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))
}
