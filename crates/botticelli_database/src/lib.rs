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

mod connection;
mod content_generation_models;
mod content_generation_repository;
mod content_management;
mod models;
mod narrative_conversions;
mod narrative_models;
mod narrative_repository;

// Public modules for external access
pub mod schema;
pub mod schema_docs;
pub mod schema_inference;
pub mod schema_reflection;

// Re-export connection utilities
pub use connection::establish_connection;

// Re-export content management functions
pub use content_management::{
    delete_content, get_content_by_id, list_content, promote_content, update_content_metadata,
    update_review_status,
};

// Re-export content generation types
pub use content_generation_models::{
    ContentGenerationRow, NewContentGenerationRow, UpdateContentGenerationRow,
};
pub use content_generation_repository::{
    ContentGenerationRepository, PostgresContentGenerationRepository,
};

// Re-export model types
pub use models::{ModelResponse, NewModelResponse, SerializableModelResponse};

// Re-export narrative types
pub use narrative_models::{
    ActExecutionRow, ActInputRow, NarrativeExecutionRow, NewActExecutionRow, NewActInputRow,
    NewNarrativeExecutionRow,
};
pub use narrative_repository::PostgresNarrativeRepository;

use botticelli_error::DatabaseError;

/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;
