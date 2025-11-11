//! Database module for storing and retrieving model responses and narrative executions.

mod error;
mod models;
mod narrative_conversions;
mod narrative_models;
mod narrative_repository;
mod schema;

use diesel::pg::PgConnection;
use diesel::prelude::*;

pub use error::{DatabaseError, DatabaseErrorKind, DatabaseResult};
pub use models::{ModelResponse, NewModelResponse, SerializableModelResponse};
pub use narrative_models::{
    ActExecutionRow, ActInputRow, NarrativeExecutionRow, NewActExecutionRow, NewActInputRow,
    NewNarrativeExecutionRow,
};
pub use narrative_repository::PostgresNarrativeRepository;

use crate::{GenerateRequest, GenerateResponse};

/// Establish a connection to the PostgreSQL database.
///
/// Composes the connection URL from environment variables:
/// - `DATABASE_USER` - PostgreSQL username (required)
/// - `DATABASE_PASSWORD` - PostgreSQL password (required)
/// - `DATABASE_HOST` - Database host (defaults to "localhost")
/// - `DATABASE_PORT` - Database port (defaults to "5432")
/// - `DATABASE_NAME` - Database name (defaults to "boticelli")
///
/// Alternatively, you can provide a complete `DATABASE_URL` which takes precedence.
pub fn establish_connection() -> DatabaseResult<PgConnection> {
    let _ = dotenvy::dotenv();

    // If DATABASE_URL is set, use it directly
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        return PgConnection::establish(&database_url).map_err(Into::into);
    }

    // Otherwise, compose from components
    let user = std::env::var("DATABASE_USER").map_err(|_| {
        DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_USER environment variable not set".to_string(),
        ))
    })?;

    let password = std::env::var("DATABASE_PASSWORD").map_err(|_| {
        DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_PASSWORD environment variable not set".to_string(),
        ))
    })?;

    let host = std::env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("DATABASE_PORT").unwrap_or_else(|_| "5432".to_string());
    let name = std::env::var("DATABASE_NAME").unwrap_or_else(|_| "boticelli".to_string());

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, host, port, name
    );

    PgConnection::establish(&database_url).map_err(Into::into)
}

/// Store a model response in the database.
pub fn store_response(
    conn: &mut PgConnection,
    provider: &str,
    model_name: &str,
    request: &GenerateRequest,
    response: &GenerateResponse,
    duration_ms: Option<i32>,
) -> DatabaseResult<ModelResponse> {
    use schema::model_responses;

    let new_response = NewModelResponse::new(provider, model_name, request, response, duration_ms)?;

    diesel::insert_into(model_responses::table)
        .values(&new_response)
        .get_result(conn)
        .map_err(Into::into)
}

/// Store an error response in the database.
pub fn store_error(
    conn: &mut PgConnection,
    provider: &str,
    model_name: &str,
    request: &GenerateRequest,
    error: impl std::fmt::Display,
    duration_ms: Option<i32>,
) -> DatabaseResult<ModelResponse> {
    use schema::model_responses;

    let new_response = NewModelResponse::error(provider, model_name, request, error, duration_ms)?;

    diesel::insert_into(model_responses::table)
        .values(&new_response)
        .get_result(conn)
        .map_err(Into::into)
}

/// Get a model response by ID.
pub fn get_response_by_id(
    conn: &mut PgConnection,
    response_id: uuid::Uuid,
) -> DatabaseResult<ModelResponse> {
    use schema::model_responses::dsl::*;

    model_responses
        .find(response_id)
        .first(conn)
        .map_err(Into::into)
}

/// Get all responses for a specific provider and model.
pub fn get_responses_by_model(
    conn: &mut PgConnection,
    provider_name: &str,
    model: &str,
    limit: i64,
) -> DatabaseResult<Vec<ModelResponse>> {
    use schema::model_responses::dsl::*;

    model_responses
        .filter(provider.eq(provider_name))
        .filter(model_name.eq(model))
        .order(created_at.desc())
        .limit(limit)
        .load(conn)
        .map_err(Into::into)
}

/// Get recent responses across all models.
pub fn get_recent_responses(
    conn: &mut PgConnection,
    limit: i64,
) -> DatabaseResult<Vec<ModelResponse>> {
    use schema::model_responses::dsl::*;

    model_responses
        .order(created_at.desc())
        .limit(limit)
        .load(conn)
        .map_err(Into::into)
}

/// Delete a response by ID.
pub fn delete_response(conn: &mut PgConnection, response_id: uuid::Uuid) -> DatabaseResult<usize> {
    use schema::model_responses::dsl::*;

    diesel::delete(model_responses.find(response_id))
        .execute(conn)
        .map_err(Into::into)
}

/// Run pending migrations.
#[cfg(feature = "database")]
pub fn run_migrations(conn: &mut PgConnection) -> DatabaseResult<()> {
    use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    conn.run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Migration(e.to_string())))
}
