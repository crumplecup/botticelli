//! Repository abstraction for storing and retrieving narrative executions.
//!
//! This module provides a trait-based abstraction for persisting narrative
//! execution history, allowing multiple storage backends (database, filesystem, etc.)
//! without coupling application logic to a specific implementation.

use crate::{BoticelliResult, NarrativeExecution};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(feature = "database")]
use chrono::{DateTime, Utc};

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
    ///
    /// # Arguments
    /// * `execution` - The complete execution to persist
    ///
    /// # Returns
    /// The unique ID of the saved execution
    async fn save_execution(&self, execution: &NarrativeExecution) -> BoticelliResult<i32>;

    /// Load a narrative execution by its unique ID.
    ///
    /// This reconstructs the complete execution including all acts and inputs.
    ///
    /// # Arguments
    /// * `id` - The unique execution ID
    ///
    /// # Returns
    /// The complete execution, or an error if not found
    async fn load_execution(&self, id: i32) -> BoticelliResult<NarrativeExecution>;

    /// List executions matching the given filter criteria.
    ///
    /// Returns lightweight summaries without full act details for efficient querying.
    ///
    /// # Arguments
    /// * `filter` - Filter and pagination criteria
    ///
    /// # Returns
    /// Vector of execution summaries matching the filter
    async fn list_executions(
        &self,
        filter: &ExecutionFilter,
    ) -> BoticelliResult<Vec<ExecutionSummary>>;

    /// Update the status of a running execution.
    ///
    /// Useful for marking executions as completed or failed, or updating
    /// progress for long-running narratives.
    ///
    /// # Arguments
    /// * `id` - The execution ID to update
    /// * `status` - The new status
    async fn update_status(&self, id: i32, status: ExecutionStatus) -> BoticelliResult<()>;

    /// Delete an execution and all associated data.
    ///
    /// This should cascade delete all acts and inputs associated with the execution.
    ///
    /// # Arguments
    /// * `id` - The execution ID to delete
    async fn delete_execution(&self, id: i32) -> BoticelliResult<()>;

    // Media storage methods - new unified approach for images/audio/video

    /// Store media using configured storage backend and save metadata to database.
    ///
    /// This stores the binary data in the configured backend (filesystem, S3, etc.)
    /// and records metadata in the media_references table. Handles deduplication
    /// automatically via content hash.
    ///
    /// # Arguments
    /// * `data` - Raw media bytes
    /// * `metadata` - Media metadata (type, mime type, dimensions, etc.)
    ///
    /// # Returns
    /// A `MediaReference` containing the UUID and location information
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &crate::MediaMetadata,
    ) -> BoticelliResult<crate::MediaReference>;

    /// Retrieve media by reference.
    ///
    /// # Arguments
    /// * `reference` - The media reference from `store_media`
    ///
    /// # Returns
    /// The raw media bytes
    async fn load_media(
        &self,
        reference: &crate::MediaReference,
    ) -> BoticelliResult<Vec<u8>>;

    /// Get media reference by content hash for deduplication.
    ///
    /// Check if media with the same content hash already exists.
    ///
    /// # Arguments
    /// * `content_hash` - SHA-256 hash of the content
    ///
    /// # Returns
    /// `Some(MediaReference)` if found, `None` otherwise
    async fn get_media_by_hash(
        &self,
        content_hash: &str,
    ) -> BoticelliResult<Option<crate::MediaReference>>;

    // Video storage methods - DEPRECATED, use store_media/load_media instead
    // Kept for backward compatibility, default implementations return NotImplemented

    /// Store video input data separately (for large file handling).
    ///
    /// Video files can be very large, so implementations may choose to store them
    /// in specialized storage (filesystem, S3, etc.) rather than in the main database.
    ///
    /// # Arguments
    /// * `video_data` - Raw video bytes
    /// * `metadata` - Video metadata (mime type, filename, etc.)
    ///
    /// # Returns
    /// A reference string that can be used to retrieve the video later
    async fn store_video(
        &self,
        _video_data: &[u8],
        _metadata: &VideoMetadata,
    ) -> BoticelliResult<String> {
        Err(crate::BoticelliError::from(crate::NotImplementedError::new(
            "Video storage not yet implemented for this repository",
        )))
    }

    /// Retrieve video input data by reference.
    ///
    /// # Arguments
    /// * `video_ref` - The reference string returned by `store_video`
    ///
    /// # Returns
    /// The raw video bytes
    async fn load_video(&self, _video_ref: &str) -> BoticelliResult<Vec<u8>> {
        Err(crate::BoticelliError::from(crate::NotImplementedError::new(
            "Video loading not yet implemented for this repository",
        )))
    }
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

    /// Filter by date range (inclusive)
    #[cfg(feature = "database")]
    pub started_after: Option<DateTime<Utc>>,

    /// Filter by date range (inclusive)
    #[cfg(feature = "database")]
    pub started_before: Option<DateTime<Utc>>,

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

    /// Set date range filter (started after).
    #[cfg(feature = "database")]
    pub fn with_started_after(mut self, date: DateTime<Utc>) -> Self {
        self.started_after = Some(date);
        self
    }

    /// Set date range filter (started before).
    #[cfg(feature = "database")]
    pub fn with_started_before(mut self, date: DateTime<Utc>) -> Self {
        self.started_before = Some(date);
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

    /// When execution started
    #[cfg(feature = "database")]
    pub started_at: DateTime<Utc>,

    /// When execution completed (None if still running)
    #[cfg(feature = "database")]
    pub completed_at: Option<DateTime<Utc>>,

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

/// Metadata for video inputs (for future video storage implementation).
///
/// This type is defined now to establish the trait interface, but video
/// storage is deferred to future work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// MIME type of the video (e.g., "video/mp4")
    pub mime_type: Option<String>,

    /// Optional filename
    pub filename: Option<String>,

    /// Size in bytes
    pub size_bytes: usize,

    /// Content hash (SHA256) for deduplication
    pub content_hash: Option<String>,
}

impl VideoMetadata {
    /// Create video metadata from raw video data.
    pub fn new(video_data: &[u8]) -> Self {
        Self {
            mime_type: None,
            filename: None,
            size_bytes: video_data.len(),
            content_hash: None,
        }
    }

    /// Set MIME type.
    pub fn with_mime_type<S: Into<String>>(mut self, mime: S) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    /// Set filename.
    pub fn with_filename<S: Into<String>>(mut self, name: S) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Set content hash.
    pub fn with_content_hash<S: Into<String>>(mut self, hash: S) -> Self {
        self.content_hash = Some(hash.into());
        self
    }
}
