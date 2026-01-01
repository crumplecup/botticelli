//! Repository trait for narrative persistence.
//!
//! This module defines the interface for storing and retrieving narrative executions.
//! Implementations can use databases, filesystems, or in-memory structures.

use async_trait::async_trait;

/// Repository for storing and retrieving narrative executions.
///
/// This trait defines the interface for persistence operations. Implementations
/// can use databases, filesystems, object storage, or in-memory structures.
///
/// All methods are async to support async database drivers and network I/O.
#[async_trait]
pub trait NarrativeRepository: Send + Sync {
    /// Error type for operations.
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Media metadata type for this repository.
    type MediaMetadata: Send + Sync;
    /// Media reference type for this repository.
    type MediaReference: Send + Sync;

    /// Execution type for this repository.
    type Execution: Send + Sync;
    /// Filter type for querying executions.
    type Filter: Send + Sync;
    /// Summary type for execution listings.
    type Summary: Send + Sync;
    /// Status type for execution state.
    type Status: Send + Sync;

    /// Save a complete narrative execution and return its unique ID.
    ///
    /// This should atomically persist the execution metadata, all act executions,
    /// and all multimodal inputs. If any part fails, the entire save should be
    /// rolled back.
    async fn save_execution(&self, execution: &Self::Execution) -> Result<i32, Self::Error>;

    /// Load a narrative execution by its unique ID.
    ///
    /// This reconstructs the complete execution including all acts and inputs.
    async fn load_execution(&self, id: i32) -> Result<Self::Execution, Self::Error>;

    /// List executions matching the given filter criteria.
    ///
    /// Returns lightweight summaries without full act details for efficient querying.
    async fn list_executions(
        &self,
        filter: &Self::Filter,
    ) -> Result<Vec<Self::Summary>, Self::Error>;

    /// Update the status of a running execution.
    ///
    /// Useful for marking executions as completed or failed, or updating
    /// progress for long-running narratives.
    async fn update_status(&self, id: i32, status: Self::Status) -> Result<(), Self::Error>;

    /// Delete an execution and all associated data.
    ///
    /// This should cascade delete all acts and inputs associated with the execution.
    async fn delete_execution(&self, id: i32) -> Result<(), Self::Error>;

    /// Store media using configured storage backend and save metadata to database.
    ///
    /// This stores the binary data in the configured backend (filesystem, S3, etc.)
    /// and records metadata in the media_references table. Handles deduplication
    /// automatically via content hash.
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &Self::MediaMetadata,
    ) -> Result<Self::MediaReference, Self::Error>;

    /// Retrieve media by reference.
    async fn load_media(&self, reference: &Self::MediaReference) -> Result<Vec<u8>, Self::Error>;

    /// Get media reference by content hash for deduplication.
    ///
    /// Check if media with the same content hash already exists.
    async fn get_media_by_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Self::MediaReference>, Self::Error>;
}
