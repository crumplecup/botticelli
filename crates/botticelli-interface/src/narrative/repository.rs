//! Repository trait for narrative persistence.
//!
//! This module defines the interface for storing and retrieving narrative executions.
//! Implementations can use databases, filesystems, or in-memory structures.

use crate::narrative::execution::NarrativeExecution;
use botticelli_error::BotticelliResult;
use botticelli_storage::{MediaMetadata, MediaReference};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Repository for storing and retrieving narrative executions.
///
/// This trait defines the interface for persistence operations. Implementations
/// can use databases, filesystems, object storage, or in-memory structures.
///
/// All methods are async to support async database drivers and network I/O.
#[async_trait]
pub trait NarrativeRepository: Send + Sync {
    /// Save a complete narrative execution and return its unique ID.
    ///
    /// This should atomically persist the execution metadata, all act executions,
    /// and all multimodal inputs. If any part fails, the entire save should be
    /// rolled back.
    async fn save_execution(&self, execution: &NarrativeExecution) -> BotticelliResult<i32>;

    /// Load a narrative execution by its unique ID.
    ///
    /// This reconstructs the complete execution including all acts and inputs.
    async fn load_execution(&self, id: i32) -> BotticelliResult<NarrativeExecution>;

    /// List executions matching the given filter criteria.
    ///
    /// Returns lightweight summaries without full act details for efficient querying.
    async fn list_executions(
        &self,
        filter: &ExecutionFilter,
    ) -> BotticelliResult<Vec<ExecutionSummary>>;

    /// Update the status of a running execution.
    ///
    /// Useful for marking executions as completed or failed, or updating
    /// progress for long-running narratives.
    async fn update_status(&self, id: i32, status: ExecutionStatus) -> BotticelliResult<()>;

    /// Delete an execution and all associated data.
    ///
    /// This should cascade delete all acts and inputs associated with the execution.
    async fn delete_execution(&self, id: i32) -> BotticelliResult<()>;

    /// Store media using configured storage backend and save metadata to database.
    ///
    /// This stores the binary data in the configured backend (filesystem, S3, etc.)
    /// and records metadata in the media_references table. Handles deduplication
    /// automatically via content hash.
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &MediaMetadata,
    ) -> BotticelliResult<MediaReference>;

    /// Retrieve media by reference.
    async fn load_media(&self, reference: &MediaReference) -> BotticelliResult<Vec<u8>>;

    /// Get media reference by content hash for deduplication.
    ///
    /// Check if media with the same content hash already exists.
    async fn get_media_by_hash(
        &self,
        content_hash: &str,
    ) -> BotticelliResult<Option<MediaReference>>;
}

/// Filter criteria for querying executions.
///
/// All fields are optional to allow flexible queries. Combining multiple
/// criteria creates an AND condition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionFilter {
    /// Filter by narrative name (exact match)
    pub narrative_name: Option<String>,
    /// Filter by execution status
    pub status: Option<ExecutionStatus>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Number of results to skip (for pagination)
    pub offset: Option<usize>,
}

impl ExecutionFilter {
    /// Create an empty filter (returns all executions).
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by narrative name.
    pub fn with_narrative_name<S: Into<String>>(mut self, name: S) -> Self {
        self.narrative_name = Some(name.into());
        self
    }

    /// Filter by execution status.
    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set pagination limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set pagination offset.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Summary of an execution (lightweight view without full act details).
///
/// Used by `list_executions` to return metadata about executions without
/// loading all the act data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// Unique execution ID
    pub id: i32,
    /// Name of the narrative that was executed
    pub narrative_name: String,
    /// Optional description from narrative metadata
    pub narrative_description: Option<String>,
    /// Execution status
    pub status: ExecutionStatus,
    /// Number of acts in this execution
    pub act_count: usize,
    /// Error message if status is Failed
    pub error_message: Option<String>,
}

/// Execution status enumeration.
///
/// Tracks the lifecycle state of a narrative execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution is currently in progress
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed with an error
    Failed,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for ExecutionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" => Ok(ExecutionStatus::Running),
            "completed" => Ok(ExecutionStatus::Completed),
            "failed" => Ok(ExecutionStatus::Failed),
            _ => Err(format!("Invalid execution status: {}", s)),
        }
    }
}
