//! PostgreSQL integration for Botticelli.
//!
//! This crate provides database models, schema definitions, and repository
//! implementations for persisting narratives and content.
//!
//! # Features
//!
//! - Diesel-based PostgreSQL integration
//! - Narrative persistence and retrieval
//! - Content generation tracking
//! - Schema reflection and inference
//!
//! # Example
//!
//! ```rust,ignore
//! use botticelli_database::{establish_connection, NarrativeRepository, PostgresNarrativeRepository};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut conn = establish_connection()?;
//! let repo = PostgresNarrativeRepository::new();
//! 
//! // Use repository...
//! # Ok(())
//! # }
//! ```

pub mod schema;
pub mod models;
pub mod narrative_models;
pub mod narrative_conversions;
pub mod narrative_repository;
pub mod content_generation_models;
pub mod content_generation_repository;
pub mod content_management;
pub mod schema_reflection;
pub mod schema_inference;
pub mod schema_docs;

// Re-export key types
pub use models::*;
pub use narrative_models::*;
pub use narrative_repository::*;
pub use content_generation_models::*;
pub use content_generation_repository::*;

// Re-export error types from botticelli-error
pub use botticelli_error::{DatabaseError, DatabaseErrorKind};

/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;

use diesel::prelude::*;
use diesel::pg::PgConnection;

/// Establish a connection to the PostgreSQL database.
///
/// Reads the `DATABASE_URL` environment variable to determine the connection string.
pub fn establish_connection() -> DatabaseResult<PgConnection> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_URL environment variable not set".to_string()
        )))?;
    
    PgConnection::establish(&database_url)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))
}

// Diesel error conversions
impl From<diesel::result::Error> for DatabaseError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => DatabaseError::new(DatabaseErrorKind::NotFound),
            _ => DatabaseError::new(DatabaseErrorKind::Query(err.to_string())),
        }
    }
}

impl From<diesel::ConnectionError> for DatabaseError {
    fn from(err: diesel::ConnectionError) -> Self {
        DatabaseError::new(DatabaseErrorKind::Connection(err.to_string()))
    }
}

impl From<serde_json::Error> for DatabaseError {
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::new(DatabaseErrorKind::Serialization(err.to_string()))
    }
}
