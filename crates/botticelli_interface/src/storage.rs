//! Backend-agnostic storage traits and domain record types.
//!
//! These traits abstract over both redb (default) and PostgreSQL (opt-in)
//! persistence backends. Consumers receive `Arc<dyn BotStorage>` and never
//! name the concrete backend type.

use chrono::{DateTime, Utc};
use elicit_db::DbError;
use serde::{Deserialize, Serialize};

// ── Error ─────────────────────────────────────────────────────────────────────

/// Error variants for storage operations.
#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::From)]
pub enum BotStorageError {
    /// Backend-level failure from the elicit_db layer.
    #[display("storage backend error: {}", _0)]
    Backend(DbError),

    /// JSON serialization or deserialization failure.
    #[display("json error: {}", _0)]
    Json(serde_json::Error),

    /// Datetime parse failure when reading a stored RFC 3339 string.
    #[display("datetime parse error: {}", _0)]
    DatetimeParse(chrono::format::ParseError),

    /// A required column was absent from a returned row.
    #[from(skip)]
    #[display("missing column: {}", _0)]
    MissingColumn(#[error(ignore)] String),

    /// Requested record does not exist.
    #[from(skip)]
    #[display("not found: {}", _0)]
    NotFound(#[error(ignore)] String),

    /// Configuration or environment variable error (e.g. missing REDB_PATH).
    #[from(skip)]
    #[display("config error: {}", _0)]
    Config(#[error(ignore)] String),
}

/// Convenience alias for storage results.
pub type BotStorageResult<T> = Result<T, BotStorageError>;

// ── Domain record types ───────────────────────────────────────────────────────
//
// Plain-Rust representations of each persisted entity, independent of any backend.

/// A single narrative execution run.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct NarrativeExecutionRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Narrative name from the TOML file.
    pub narrative_name: String,
    /// Optional description from the TOML file.
    pub narrative_description: Option<String>,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution finished (if complete).
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution status: "running", "success", "failed".
    pub status: String,
    /// Error message if status is "failed".
    pub error_message: Option<String>,
    /// Record creation time.
    pub created_at: DateTime<Utc>,
}

/// A single act execution within a narrative run.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ActExecutionRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Parent narrative execution ID.
    pub narrative_execution_id: String,
    /// Act name from the TOML file.
    pub act_name: String,
    /// Position within the narrative (0-based).
    pub sequence_number: i32,
    /// Model used for this act.
    pub model: Option<String>,
    /// Sampling temperature used.
    pub temperature: Option<f32>,
    /// Max tokens used.
    pub max_tokens: Option<i32>,
    /// Raw LLM response text.
    pub response: String,
    /// Record creation time.
    pub created_at: DateTime<Utc>,
}

/// A single input item passed to an act.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ActInputRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Parent act execution ID.
    pub act_execution_id: String,
    /// Position within the act's input list.
    pub input_order: i32,
    /// Input type: "text", "image", "file", etc.
    pub input_type: String,
    /// Text content if input_type is "text".
    pub text_content: Option<String>,
    /// MIME type for media inputs.
    pub mime_type: Option<String>,
    /// Filename for file inputs.
    pub filename: Option<String>,
    /// Media reference ID (UUID string) if stored separately.
    pub media_ref_id: Option<String>,
    /// Record creation time.
    pub created_at: DateTime<Utc>,
}

/// Persisted state for a scheduled actor task.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ActorServerStateRecord {
    /// Unique task identifier.
    pub task_id: String,
    /// Actor name.
    pub actor_name: String,
    /// Last execution time.
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled execution time.
    pub next_run: DateTime<Utc>,
    /// Consecutive failure count (for circuit breaker).
    pub consecutive_failures: i32,
    /// Whether the task is currently paused.
    pub is_paused: bool,
    /// Arbitrary JSON metadata.
    pub metadata: serde_json::Value,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
}

/// A single execution record for an actor task.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ActorServerExecutionRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Task identifier.
    pub task_id: String,
    /// Actor name.
    pub actor_name: String,
    /// Execution start time.
    pub started_at: DateTime<Utc>,
    /// Execution finish time.
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether the execution succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Skills that ran successfully.
    pub skills_succeeded: i32,
    /// Skills that failed.
    pub skills_failed: i32,
    /// Skills that were skipped.
    pub skills_skipped: i32,
    /// Arbitrary JSON metadata.
    pub metadata: serde_json::Value,
    /// Record creation time.
    pub created_at: DateTime<Utc>,
}

/// A generated content row stored in a named content table.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ContentRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Name of the logical content table this row belongs to.
    pub table_name: String,
    /// JSON-encoded content payload.
    pub content_json: serde_json::Value,
    /// Record creation time.
    pub created_at: DateTime<Utc>,
}

/// Metadata for a content generation run.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ContentGenerationRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Content table this generation targeted.
    pub table_name: String,
    /// Narrative TOML file path.
    pub narrative_file: String,
    /// Narrative name within the file.
    pub narrative_name: String,
    /// When generation started.
    pub generated_at: DateTime<Utc>,
    /// When generation completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of rows produced.
    pub row_count: Option<i32>,
    /// Wall-clock duration in milliseconds.
    pub generation_duration_ms: Option<i32>,
    /// Status: "running", "success", "failed".
    pub status: String,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Who or what triggered this generation.
    pub created_by: Option<String>,
}

/// A stored LLM request/response pair.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ModelResponseRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Record creation time.
    pub created_at: DateTime<Utc>,
    /// Provider name (e.g. "anthropic", "gemini").
    pub provider: String,
    /// Model name used for this request.
    pub model_name: String,
    /// JSON-encoded request messages.
    pub request_messages: serde_json::Value,
    /// Sampling temperature from the request.
    pub request_temperature: Option<f32>,
    /// Max tokens from the request.
    pub request_max_tokens: Option<i32>,
    /// Model override from the request.
    pub request_model: Option<String>,
    /// JSON-encoded response outputs.
    pub response_outputs: serde_json::Value,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: Option<i32>,
    /// Error message if the request failed.
    pub error_message: Option<String>,
}

/// A record of a post published to a social platform.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct PostHistoryRecord {
    /// Unique identifier (UUID string).
    pub id: String,
    /// Social platform name (e.g. "discord").
    pub platform: String,
    /// Platform-assigned post ID.
    pub post_id: String,
    /// JSON-encoded post payload.
    pub content_json: serde_json::Value,
    /// When the post was published.
    pub posted_at: DateTime<Utc>,
}

// ── Sub-traits ────────────────────────────────────────────────────────────────

/// Storage operations for narrative and act execution records.
#[async_trait::async_trait]
pub trait NarrativeStore: Send + Sync {
    /// Persist a new or updated narrative execution record.
    async fn save_narrative_execution(
        &self,
        record: &NarrativeExecutionRecord,
    ) -> BotStorageResult<()>;

    /// Retrieve a narrative execution by ID.
    async fn get_narrative_execution(
        &self,
        id: &str,
    ) -> BotStorageResult<Option<NarrativeExecutionRecord>>;

    /// List narrative executions, most-recent first.
    async fn list_narrative_executions(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<NarrativeExecutionRecord>>;

    /// Persist a new or updated act execution record.
    async fn save_act_execution(&self, record: &ActExecutionRecord) -> BotStorageResult<()>;

    /// List act executions for a given narrative execution.
    async fn list_act_executions(
        &self,
        narrative_execution_id: &str,
    ) -> BotStorageResult<Vec<ActExecutionRecord>>;

    /// Persist an act input record.
    async fn save_act_input(&self, record: &ActInputRecord) -> BotStorageResult<()>;
}

/// Storage operations for actor task state and execution history.
#[async_trait::async_trait]
pub trait ActorStateStore: Send + Sync {
    /// Persist actor task state (upsert by task_id).
    async fn save_actor_state(&self, record: &ActorServerStateRecord) -> BotStorageResult<()>;

    /// Retrieve current state for a named actor task.
    async fn get_actor_state(
        &self,
        task_id: &str,
    ) -> BotStorageResult<Option<ActorServerStateRecord>>;

    /// List all actor task states.
    async fn list_actor_states(&self) -> BotStorageResult<Vec<ActorServerStateRecord>>;

    /// Delete the state record for a task.
    async fn delete_actor_state(&self, task_id: &str) -> BotStorageResult<()>;

    /// Persist an actor execution record (upsert by id).
    async fn save_actor_execution(
        &self,
        record: &ActorServerExecutionRecord,
    ) -> BotStorageResult<()>;

    /// List execution records for a specific task, most-recent first.
    async fn list_actor_executions(
        &self,
        task_id: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ActorServerExecutionRecord>>;

    /// List all execution records across all tasks, most-recent first.
    async fn list_all_actor_executions(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ActorServerExecutionRecord>>;

    /// Delete an execution record by id (for pruning).
    async fn delete_actor_execution(&self, id: &str) -> BotStorageResult<()>;
}

/// Storage operations for generated content and model responses.
#[async_trait::async_trait]
pub trait ContentStore: Send + Sync {
    /// Persist a generated content row (upsert by id).
    async fn save_content(&self, record: &ContentRecord) -> BotStorageResult<()>;

    /// Retrieve a single content row by id.
    async fn get_content(&self, id: &str) -> BotStorageResult<Option<ContentRecord>>;

    /// List content rows for a named table, most-recent first.
    async fn list_content(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentRecord>>;

    /// Delete a content row by id.
    async fn delete_content(&self, id: &str) -> BotStorageResult<()>;

    /// Persist a content generation metadata record (upsert by id).
    async fn save_content_generation(
        &self,
        record: &ContentGenerationRecord,
    ) -> BotStorageResult<()>;

    /// List content generation records for a specific table, most-recent first.
    async fn list_content_generations_by_table(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentGenerationRecord>>;

    /// List all content generation records, most-recent first.
    async fn list_content_generations(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ContentGenerationRecord>>;

    /// Persist a model request/response pair (upsert by id).
    async fn save_model_response(&self, record: &ModelResponseRecord) -> BotStorageResult<()>;

    /// List model responses, most-recent first.
    async fn list_model_responses(
        &self,
        limit: usize,
    ) -> BotStorageResult<Vec<ModelResponseRecord>>;
}

/// Storage operations for social post history.
#[async_trait::async_trait]
pub trait PostStore: Send + Sync {
    /// Persist a post history record (upsert by id).
    async fn save_post_history(&self, record: &PostHistoryRecord) -> BotStorageResult<()>;

    /// List post history for a platform, most-recent first.
    async fn list_post_history(
        &self,
        platform: &str,
        limit: usize,
    ) -> BotStorageResult<Vec<PostHistoryRecord>>;
}

// ── Aggregate trait ───────────────────────────────────────────────────────────

/// A complete persistence backend for Botticelli.
///
/// Implementors provide all four storage sub-trait capabilities.
/// Consumers depend on `Arc<dyn BotStorage>` and never name the backend.
pub trait BotStorage: NarrativeStore + ActorStateStore + ContentStore + PostStore {}
