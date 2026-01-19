//! Diesel models for content generation tracking.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

/// Database row for content_generations table.
///
/// Tracks metadata for each content generation execution, including
/// success/failure status, timing information, and error details.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, Serialize, derive_getters::Getters)]
#[diesel(table_name = crate::schema::content_generations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ContentGenerationRow {
    id: i32,
    table_name: String,
    narrative_file: String,
    narrative_name: String,
    generated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    row_count: Option<i32>,
    generation_duration_ms: Option<i32>,
    status: String,
    error_message: Option<String>,
    created_by: Option<String>,
}

/// Insertable struct for starting a new content generation.
///
/// Used to record the start of a content generation attempt.
/// The status should be 'running' initially.
#[derive(Debug, Clone, Insertable, elicitation::Elicit)]
#[diesel(table_name = crate::schema::content_generations)]
pub struct NewContentGenerationRow {
    pub table_name: String,
    pub narrative_file: String,
    pub narrative_name: String,
    pub status: String,
    pub created_by: Option<String>,
}

/// Updateable struct for completing a content generation.
///
/// Used to update the generation record with completion metadata.
/// Status should be 'success' or 'failed'.
#[derive(Debug, Clone, AsChangeset, elicitation::Elicit)]
#[diesel(table_name = crate::schema::content_generations)]
pub struct UpdateContentGenerationRow {
    pub completed_at: Option<DateTime<Utc>>,
    pub row_count: Option<i32>,
    pub generation_duration_ms: Option<i32>,
    pub status: Option<String>,
    pub error_message: Option<String>,
}
