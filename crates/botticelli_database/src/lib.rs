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

pub mod content_generation_models;
pub mod content_generation_repository;
pub mod content_management;
pub mod models;
pub mod narrative_conversions;
pub mod narrative_models;
pub mod narrative_repository;
pub mod schema;
pub mod schema_docs;
pub mod schema_inference;
pub mod schema_reflection;

// Re-export key types
pub use content_generation_models::*;
pub use content_generation_repository::*;
pub use content_management::{
    delete_content, list_content, promote_content, update_content_metadata, update_review_status,
};
pub use models::*;
pub use narrative_models::*;
pub use narrative_repository::*;

use botticelli_error::{DatabaseError, DatabaseErrorKind};

/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;

use diesel::pg::PgConnection;
use diesel::prelude::*;

/// Establish a connection to the PostgreSQL database.
///
/// Reads the `DATABASE_URL` environment variable to determine the connection string.
pub fn establish_connection() -> DatabaseResult<PgConnection> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        DatabaseError::new(DatabaseErrorKind::Connection(
            "DATABASE_URL environment variable not set".to_string(),
        ))
    })?;

    PgConnection::establish(&database_url)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))
}
