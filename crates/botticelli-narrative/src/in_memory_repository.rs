//! In-memory implementation of NarrativeRepository for testing.
//!
//! This module provides a simple HashMap-based repository that stores executions
//! in memory. Useful for unit tests and demonstrating the trait interface.

use crate::{
    BackendError, BotticelliError, BotticelliResult, ExecutionFilter, ExecutionStatus,
    ExecutionSummary, NarrativeExecution, NarrativeRepository,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "database")]
use chrono::Utc;

/// In-memory repository for narrative executions.
///
/// Stores executions in a HashMap protected by an RwLock for thread-safe access.
/// All data is lost when the repository is dropped.
///
/// # Example
/// ```no_run
/// use botticelli::{InMemoryNarrativeRepository, NarrativeRepository};
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
}

/// Internal storage structure for executions.
#[derive(Debug, Clone)]
struct StoredExecution {
    id: i32,
    narrative_name: String,
    narrative_description: Option<String>,
    status: ExecutionStatus,
    #[cfg(feature = "database")]
    started_at: chrono::DateTime<Utc>,
    #[cfg(feature = "database")]
    completed_at: Option<chrono::DateTime<Utc>>,
    execution: NarrativeExecution,
    error_message: Option<String>,
}

impl InMemoryNarrativeRepository {
    /// Create a new empty in-memory repository.
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
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
    }
}

impl Default for InMemoryNarrativeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NarrativeRepository for InMemoryNarrativeRepository {
    async fn save_execution(&self, execution: &NarrativeExecution) -> BotticelliResult<i32> {
        let mut next_id_guard = self.next_id.write().await;
        let id = *next_id_guard;
        *next_id_guard += 1;
        drop(next_id_guard);

        let stored = StoredExecution {
            id,
            narrative_name: execution.narrative_name.clone(),
            narrative_description: None, // Not available in current NarrativeExecution
            status: ExecutionStatus::Completed,
            #[cfg(feature = "database")]
            started_at: Utc::now(),
            #[cfg(feature = "database")]
            completed_at: Some(Utc::now()),
            execution: execution.clone(),
            error_message: None,
        };

        self.executions.write().await.insert(id, stored);
        Ok(id)
    }

    async fn load_execution(&self, id: i32) -> BotticelliResult<NarrativeExecution> {
        let executions = self.executions.read().await;
        executions
            .get(&id)
            .map(|stored| stored.execution.clone())
            .ok_or_else(|| {
                BotticelliError::from(BackendError::new(format!("Execution {} not found", id)))
            })
    }

    async fn list_executions(
        &self,
        filter: &ExecutionFilter,
    ) -> BotticelliResult<Vec<ExecutionSummary>> {
        let executions = self.executions.read().await;
        let mut results: Vec<ExecutionSummary> = executions
            .values()
            .filter(|stored| {
                // Apply narrative_name filter
                if let Some(ref name) = filter.narrative_name
                    && &stored.narrative_name != name
                {
                    return false;
                }

                // Apply status filter
                if let Some(ref status) = filter.status
                    && &stored.status != status
                {
                    return false;
                }

                // Apply date range filters (only with database feature)
                #[cfg(feature = "database")]
                {
                    if let Some(ref after) = filter.started_after
                        && &stored.started_at < after
                    {
                        return false;
                    }

                    if let Some(ref before) = filter.started_before
                        && &stored.started_at > before
                    {
                        return false;
                    }
                }

                true
            })
            .map(|stored| ExecutionSummary {
                id: stored.id,
                narrative_name: stored.narrative_name.clone(),
                narrative_description: stored.narrative_description.clone(),
                status: stored.status,
                #[cfg(feature = "database")]
                started_at: stored.started_at,
                #[cfg(feature = "database")]
                completed_at: stored.completed_at,
                act_count: stored.execution.act_executions.len(),
                error_message: stored.error_message.clone(),
            })
            .collect();

        // Sort by ID for consistent ordering
        results.sort_by_key(|s| s.id);

        // Apply pagination
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn update_status(&self, id: i32, status: ExecutionStatus) -> BotticelliResult<()> {
        let mut executions = self.executions.write().await;
        executions
            .get_mut(&id)
            .map(|stored| {
                stored.status = status;
                #[cfg(feature = "database")]
                if matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed) {
                    stored.completed_at = Some(Utc::now());
                }
            })
            .ok_or_else(|| {
                BotticelliError::from(BackendError::new(format!("Execution {} not found", id)))
            })
    }

    async fn delete_execution(&self, id: i32) -> BotticelliResult<()> {
        self.executions
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| {
                BotticelliError::from(BackendError::new(format!("Execution {} not found", id)))
            })
    }

    // Media storage methods - simple passthrough to future implementations
    async fn store_media(
        &self,
        _data: &[u8],
        _metadata: &crate::MediaMetadata,
    ) -> BotticelliResult<crate::MediaReference> {
        Err(crate::BotticelliError::from(
            crate::NotImplementedError::new(
                "Media storage not yet implemented for in-memory repository",
            ),
        ))
    }

    async fn load_media(&self, _reference: &crate::MediaReference) -> BotticelliResult<Vec<u8>> {
        Err(crate::BotticelliError::from(
            crate::NotImplementedError::new(
                "Media loading not yet implemented for in-memory repository",
            ),
        ))
    }

    async fn get_media_by_hash(
        &self,
        _content_hash: &str,
    ) -> BotticelliResult<Option<crate::MediaReference>> {
        Ok(None)
    }

    // Video methods use default implementations (return NotImplemented)
}
