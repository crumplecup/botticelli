//! In-memory implementation of NarrativeRepository for testing.
//!
//! This module provides a simple HashMap-based repository that stores executions
//! in memory. Useful for unit tests and demonstrating the trait interface.

use async_trait::async_trait;
use botticelli_core::{ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeExecution};
use botticelli_error::NarrativeError;
use botticelli_interface::NarrativeRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory repository for narrative executions.
///
/// Stores executions in a HashMap protected by an RwLock for thread-safe access.
/// All data is lost when the repository is dropped.
///
/// # Example
/// ```no_run
/// use botticelli_narrative::InMemoryNarrativeRepository;
/// use botticelli_interface::NarrativeRepository;
///
/// #[tokio::main]
/// async fn main() {
///     let repo = InMemoryNarrativeRepository::new();
///     // Use repo.save_execution(), load_execution(), etc.
/// }
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryNarrativeRepository {
    /// Storage for executions, keyed by ID
    executions: Arc<RwLock<HashMap<i32, StoredExecution>>>,
    /// Next ID to assign
    next_id: Arc<RwLock<i32>>,
    /// Storage for media binary data, keyed by UUID
    media_storage: Arc<RwLock<HashMap<uuid::Uuid, Vec<u8>>>>,
    /// Index for deduplication: content hash -> media reference
    media_by_hash: Arc<RwLock<HashMap<String, botticelli_storage::MediaReference>>>,
}

/// Internal storage structure for executions.
#[derive(Debug, Clone)]
struct StoredExecution {
    id: i32,
    narrative_name: String,
    narrative_description: Option<String>,
    status: ExecutionStatus,
    execution: NarrativeExecution,
    error_message: Option<String>,
}

impl InMemoryNarrativeRepository {
    /// Create a new empty in-memory repository.
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            media_storage: Arc::new(RwLock::new(HashMap::new())),
            media_by_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of stored executions (for testing).
    pub async fn len(&self) -> usize {
        self.executions.read().await.len()
    }

    /// Check if the repository is empty (for testing).
    pub async fn is_empty(&self) -> bool {
        self.executions.read().await.is_empty()
    }

    /// Clear all executions (for testing).
    pub async fn clear(&self) {
        self.executions.write().await.clear();
        *self.next_id.write().await = 1;
        self.media_storage.write().await.clear();
        self.media_by_hash.write().await.clear();
    }
}

impl Default for InMemoryNarrativeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NarrativeRepository for InMemoryNarrativeRepository {
    type Error = botticelli_error::NarrativeError;
    type MediaMetadata = botticelli_storage::MediaMetadata;
    type MediaReference = botticelli_storage::MediaReference;
    type Execution = NarrativeExecution;
    type Filter = ExecutionFilter;
    type Summary = ExecutionSummary;
    type Status = ExecutionStatus;

    #[tracing::instrument(skip(self, execution), fields(narrative = %execution.narrative_name()))]
    async fn save_execution(&self, execution: &Self::Execution) -> Result<i32, Self::Error> {
        let mut next_id_guard = self.next_id.write().await;
        let id = *next_id_guard;
        *next_id_guard += 1;
        drop(next_id_guard);

        let stored = StoredExecution {
            id,
            narrative_name: execution.narrative_name().clone(),
            narrative_description: None, // Not available in current NarrativeExecution
            status: ExecutionStatus::Completed,
            execution: execution.clone(),
            error_message: None,
        };

        self.executions.write().await.insert(id, stored);
        Ok(id)
    }

    #[tracing::instrument(skip(self))]
    async fn load_execution(&self, id: i32) -> Result<Self::Execution, Self::Error> {
        let executions = self.executions.read().await;
        executions
            .get(&id)
            .map(|stored| stored.execution.clone())
            .ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::FileRead(format!(
                        "Execution {} not found",
                        id
                    )),
                )
            })
    }

    #[tracing::instrument(skip(self, filter), fields(narrative = ?filter.narrative_name(), status = ?filter.status()))]
    async fn list_executions(
        &self,
        filter: &Self::Filter,
    ) -> Result<Vec<Self::Summary>, Self::Error> {
        let executions = self.executions.read().await;
        let mut results: Vec<ExecutionSummary> = executions
            .values()
            .filter(|stored| {
                // Apply narrative_name filter
                if let Some(name) = filter.narrative_name()
                    && &stored.narrative_name != name
                {
                    return false;
                }

                // Apply status filter
                if let Some(status) = filter.status()
                    && &stored.status != status
                {
                    return false;
                }

                true
            })
            .map(|stored| {
                ExecutionSummary::new(
                    stored.id,
                    stored.narrative_name.clone(),
                    stored.narrative_description.clone(),
                    stored.status,
                    stored.execution.act_executions().len(),
                    stored.error_message.clone(),
                )
            })
            .collect();

        // Sort by ID for consistent ordering
        results.sort_by_key(|s| *s.id());

        // Apply pagination
        let offset = filter.offset().unwrap_or(0);
        let limit = filter.limit().unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    #[tracing::instrument(skip(self))]
    async fn update_status(&self, id: i32, status: Self::Status) -> Result<(), Self::Error> {
        let mut executions = self.executions.write().await;
        executions
            .get_mut(&id)
            .map(|stored| {
                stored.status = status;
            })
            .ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::FileRead(format!(
                        "Execution {} not found",
                        id
                    )),
                )
            })
    }

    #[tracing::instrument(skip(self))]
    async fn delete_execution(&self, id: i32) -> Result<(), Self::Error> {
        self.executions
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| {
                botticelli_error::NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::FileRead(format!(
                        "Execution {} not found",
                        id
                    )),
                )
            })
    }

    #[tracing::instrument(skip(self, data, metadata), fields(data_len = data.len(), media_type = ?metadata.media_type()))]
    async fn store_media(
        &self,
        data: &[u8],
        metadata: &Self::MediaMetadata,
    ) -> Result<Self::MediaReference, Self::Error> {
        use sha2::{Digest, Sha256};
        use tracing::{debug, info};

        // Compute content hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());

        debug!(hash = %hash, size = data.len(), "Computed content hash");

        // Check if media with same hash already exists
        {
            let hash_index = self.media_by_hash.read().await;
            if let Some(existing) = hash_index.get(&hash) {
                info!(
                    hash = %hash,
                    id = %existing.id(),
                    "Media already exists (deduplication), returning existing reference"
                );
                return Ok(existing.clone());
            }
        }

        // Generate new UUID for this media
        let id = uuid::Uuid::new_v4();

        // Create media reference
        let reference = botticelli_storage::MediaReferenceBuilder::default()
            .id(id)
            .media_type(*metadata.media_type())
            .mime_type(metadata.mime_type().to_string())
            .size_bytes(data.len() as i64)
            .content_hash(hash.clone())
            .storage_backend("in-memory".to_string())
            .storage_path(format!("mem://{}", id))
            .build()
            .map_err(|e| {
                NarrativeError::new(botticelli_error::NarrativeErrorKind::ConfigurationError(
                    format!("Failed to build media reference: {}", e),
                ))
            })?;

        // Store binary data
        self.media_storage.write().await.insert(id, data.to_vec());

        // Index by hash for deduplication
        self.media_by_hash
            .write()
            .await
            .insert(hash.clone(), reference.clone());

        info!(
            id = %id,
            hash = %hash,
            size = data.len(),
            "Stored new media"
        );

        Ok(reference)
    }

    #[tracing::instrument(skip(self), fields(id = %reference.id()))]
    async fn load_media(&self, reference: &Self::MediaReference) -> Result<Vec<u8>, Self::Error> {
        use tracing::{debug, error};

        let storage = self.media_storage.read().await;

        match storage.get(reference.id()) {
            Some(data) => {
                debug!(size = data.len(), "Retrieved media");
                Ok(data.clone())
            }
            None => {
                error!("Media not found");
                Err(NarrativeError::new(
                    botticelli_error::NarrativeErrorKind::FileRead(format!(
                        "Media not found: {}",
                        reference.id()
                    )),
                ))
            }
        }
    }

    #[tracing::instrument(skip(self), fields(content_hash))]
    async fn get_media_by_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Self::MediaReference>, Self::Error> {
        use tracing::debug;

        let hash_index = self.media_by_hash.read().await;
        let result = hash_index.get(content_hash).cloned();

        if result.is_some() {
            debug!("Found media by hash");
        } else {
            debug!("Media not found by hash");
        }

        Ok(result)
    }

    // Video methods use default implementations (return NotImplemented)
}

// Implement NarrativeRegistryOperations trait for MCP tool integration
#[async_trait]
impl botticelli_interface::NarrativeRegistryOperations for InMemoryNarrativeRepository {
    type Error = botticelli_error::NarrativeError;
    type Execution = NarrativeExecution;
    type Status = ExecutionStatus;
    type Filter = ExecutionFilter;
    type Summary = ExecutionSummary;

    async fn save_execution(&self, execution: &Self::Execution) -> Result<i32, Self::Error> {
        botticelli_interface::NarrativeRepository::save_execution(self, execution).await
    }

    async fn load_execution(&self, id: i32) -> Result<Self::Execution, Self::Error> {
        botticelli_interface::NarrativeRepository::load_execution(self, id).await
    }

    async fn update_status(&self, id: i32, status: Self::Status) -> Result<(), Self::Error> {
        botticelli_interface::NarrativeRepository::update_status(self, id, status).await
    }

    async fn list_executions(
        &self,
        filter: &Self::Filter,
    ) -> Result<Vec<Self::Summary>, Self::Error> {
        botticelli_interface::NarrativeRepository::list_executions(self, filter).await
    }

    async fn delete_execution(&self, id: i32) -> Result<(), Self::Error> {
        botticelli_interface::NarrativeRepository::delete_execution(self, id).await
    }
}
