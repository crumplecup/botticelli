//! Diesel models for actor server state management tables.

use chrono::NaiveDateTime;
use derive_builder::Builder;
use derive_getters::Getters;
use diesel::prelude::*;

/// Database row for actor_server_state table.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::actor_server_state)]
#[diesel(primary_key(task_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ActorServerStateRow {
    /// Unique task identifier
    pub task_id: String,
    /// Actor name
    pub actor_name: String,
    /// Last execution time
    pub last_run: Option<NaiveDateTime>,
    /// Next scheduled execution time
    pub next_run: NaiveDateTime,
    /// Consecutive failures count for circuit breaker
    pub consecutive_failures: Option<i32>,
    /// Whether task is paused
    pub is_paused: Option<bool>,
    /// Arbitrary JSON metadata
    pub metadata: Option<serde_json::Value>,
    /// Last update timestamp
    pub updated_at: NaiveDateTime,
}

/// Insertable struct for actor_server_state table with builder pattern.
#[derive(Debug, Clone, Insertable, Getters, Builder, elicitation::Elicit)]
#[diesel(table_name = crate::schema::actor_server_state)]
#[builder(setter(into))]
pub struct NewActorServerState {
    /// Unique task identifier
    pub task_id: String,
    /// Actor name
    pub actor_name: String,
    /// Last execution time
    #[builder(default)]
    pub last_run: Option<NaiveDateTime>,
    /// Next scheduled execution time
    pub next_run: NaiveDateTime,
    /// Consecutive failures count for circuit breaker
    #[builder(default)]
    pub consecutive_failures: i32,
    /// Whether task is paused
    #[builder(default)]
    pub is_paused: bool,
    /// Arbitrary JSON metadata
    #[builder(default)]
    pub metadata: serde_json::Value,
}

/// Database row for actor_server_executions table.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::actor_server_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ActorServerExecutionRow {
    /// Execution ID
    pub id: i64,
    /// Task identifier
    pub task_id: String,
    /// Actor name
    pub actor_name: String,
    /// Execution start time
    pub started_at: NaiveDateTime,
    /// Execution completion time
    pub completed_at: Option<NaiveDateTime>,
    /// Whether execution succeeded
    pub success: Option<bool>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Number of successfully executed skills
    pub skills_succeeded: Option<i32>,
    /// Number of failed skills
    pub skills_failed: Option<i32>,
    /// Number of skipped skills
    pub skills_skipped: Option<i32>,
    /// Arbitrary JSON metadata
    pub metadata: Option<serde_json::Value>,
    /// Record creation timestamp
    pub created_at: NaiveDateTime,
}

/// Insertable struct for actor_server_executions table with builder pattern.
#[derive(Debug, Clone, Insertable, Getters, Builder, elicitation::Elicit)]
#[diesel(table_name = crate::schema::actor_server_executions)]
#[builder(setter(into))]
pub struct NewActorServerExecution {
    /// Task identifier
    pub task_id: String,
    /// Actor name
    pub actor_name: String,
    /// Execution start time
    pub started_at: NaiveDateTime,
    /// Execution completion time
    #[builder(default)]
    pub completed_at: Option<NaiveDateTime>,
    /// Whether execution succeeded
    #[builder(default)]
    pub success: bool,
    /// Error message if failed
    #[builder(default)]
    pub error_message: Option<String>,
    /// Number of successfully executed skills
    #[builder(default)]
    pub skills_succeeded: i32,
    /// Number of failed skills
    #[builder(default)]
    pub skills_failed: i32,
    /// Number of skipped skills
    #[builder(default)]
    pub skills_skipped: i32,
    /// Arbitrary JSON metadata
    #[builder(default)]
    pub metadata: serde_json::Value,
}
