//! Diesel models for narrative execution tables.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for narrative_executions table.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::database::schema::narrative_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NarrativeExecutionRow {
    pub id: i32,
    pub narrative_name: String,
    pub narrative_description: Option<String>,
    pub started_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
}

/// Insertable struct for narrative_executions table.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::narrative_executions)]
pub struct NewNarrativeExecutionRow {
    pub narrative_name: String,
    pub narrative_description: Option<String>,
    pub started_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
    pub status: String,
    pub error_message: Option<String>,
}

/// Database row for act_executions table.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, Associations)]
#[diesel(belongs_to(NarrativeExecutionRow, foreign_key = execution_id))]
#[diesel(table_name = crate::database::schema::act_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ActExecutionRow {
    pub id: i32,
    pub execution_id: i32,
    pub act_name: String,
    pub sequence_number: i32,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub response: String,
    pub created_at: NaiveDateTime,
}

/// Insertable struct for act_executions table.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::act_executions)]
pub struct NewActExecutionRow {
    pub execution_id: i32,
    pub act_name: String,
    pub sequence_number: i32,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub response: String,
}

/// Database row for act_inputs table.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, Associations)]
#[diesel(belongs_to(ActExecutionRow, foreign_key = act_execution_id))]
#[diesel(table_name = crate::database::schema::act_inputs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ActInputRow {
    pub id: i32,
    pub act_execution_id: i32,
    pub input_order: i32,
    pub input_type: String,
    pub text_content: Option<String>,
    pub mime_type: Option<String>,
    pub source_type: Option<String>,
    pub source_url: Option<String>,
    pub source_base64: Option<String>,
    pub source_binary: Option<Vec<u8>>,
    pub source_size_bytes: Option<i64>,
    pub content_hash: Option<String>,
    pub filename: Option<String>,
    pub created_at: NaiveDateTime,
    pub media_ref_id: Option<uuid::Uuid>,
}

/// Insertable struct for act_inputs table.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::act_inputs)]
pub struct NewActInputRow {
    pub act_execution_id: i32,
    pub input_order: i32,
    pub input_type: String,
    pub text_content: Option<String>,
    pub mime_type: Option<String>,
    pub source_type: Option<String>,
    pub source_url: Option<String>,
    pub source_base64: Option<String>,
    pub source_binary: Option<Vec<u8>>,
    pub source_size_bytes: Option<i64>,
    pub content_hash: Option<String>,
    pub filename: Option<String>,
    pub media_ref_id: Option<uuid::Uuid>,
}
