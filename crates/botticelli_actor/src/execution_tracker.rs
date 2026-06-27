//! Helper for tracking actor execution with persistence

use crate::{BotStorageStatePersistence, DatabaseExecutionResult};
use botticelli_server::{ActorServerResult, StatePersistence};
use chrono::DateTime;
use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

/// Helper for integrating persistence into actor execution.
///
/// Simplifies common patterns like circuit breaking, execution logging,
/// and state updates during actor execution.
///
/// # Example
///
/// ```no_run
/// use botticelli_actor::{ActorExecutionTracker, BotStorageStatePersistence, DatabaseExecutionResult};
/// use botticelli_server::ActorServerResult;
/// use std::sync::Arc;
///
/// # async fn example() -> ActorServerResult<()> {
/// # let storage: Arc<dyn botticelli_interface::BotStorage> = unimplemented!();
/// let persistence = Arc::new(BotStorageStatePersistence::new(storage));
/// let tracker = ActorExecutionTracker::new(
///     persistence,
///     "my-task-id".to_string(),
///     "content-poster".to_string(),
/// );
///
/// // Check if should execute (circuit breaker)
/// if !tracker.should_execute().await? {
///     return Ok(());
/// }
///
/// // Start execution
/// let exec_id = tracker.start_execution().await?;
///
/// // Execute actor logic
/// match execute_actor_skills().await {
///     Ok(result) => {
///         tracker.record_success(exec_id, result).await?;
///     }
///     Err(e) => {
///         let paused = tracker.record_failure(exec_id, &e.to_string()).await?;
///         if paused {
///             // Circuit breaker triggered
///         }
///     }
/// }
/// # Ok(())
/// # }
/// # async fn execute_actor_skills() -> ActorServerResult<DatabaseExecutionResult> {
/// #     Ok(DatabaseExecutionResult {
/// #         skills_succeeded: 1,
/// #         skills_failed: 0,
/// #         skills_skipped: 0,
/// #         metadata: serde_json::json!({}),
/// #     })
/// # }
/// ```
#[derive(Clone)]
pub struct ActorExecutionTracker<P> {
    persistence: Arc<P>,
    task_id: String,
    actor_name: String,
}

impl<P> ActorExecutionTracker<P>
where
    P: StatePersistence,
{
    /// Create a new execution tracker
    ///
    /// # Parameters
    /// - `persistence`: Shared persistence backend
    /// - `task_id`: Unique task identifier
    /// - `actor_name`: Name of the actor being executed
    pub fn new(persistence: Arc<P>, task_id: String, actor_name: String) -> Self {
        Self {
            persistence,
            task_id,
            actor_name,
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Get the actor name
    pub fn actor_name(&self) -> &str {
        &self.actor_name
    }

    /// Get reference to persistence backend
    pub fn persistence(&self) -> &Arc<P> {
        &self.persistence
    }
}

// BotStorageStatePersistence-specific methods
impl ActorExecutionTracker<BotStorageStatePersistence> {
    /// Start execution and return execution ID (UUID string) for logging.
    #[instrument(skip(self), fields(task_id = %self.task_id, actor = %self.actor_name))]
    pub async fn start_execution(&self) -> ActorServerResult<String> {
        debug!("Starting execution");
        self.persistence
            .start_execution(&self.task_id, &self.actor_name)
            .await
    }

    /// Record successful execution.
    ///
    /// Updates execution record and resets consecutive failure count.
    #[instrument(skip(self, result), fields(task_id = %self.task_id, exec_id = %exec_id))]
    pub async fn record_success(
        &self,
        exec_id: String,
        result: DatabaseExecutionResult,
    ) -> ActorServerResult<()> {
        debug!("Recording success");
        self.persistence
            .complete_execution(&exec_id, &self.task_id, result)
            .await?;
        self.persistence.record_success(&self.task_id).await?;
        Ok(())
    }

    /// Record failed execution.
    ///
    /// Updates execution record, increments failure count, and checks circuit breaker.
    ///
    /// # Returns
    /// `true` if circuit breaker threshold exceeded and task should pause
    #[instrument(skip(self, error), fields(task_id = %self.task_id, exec_id = %exec_id))]
    pub async fn record_failure(&self, exec_id: String, error: &str) -> ActorServerResult<bool> {
        debug!("Recording failure");
        self.persistence
            .fail_execution(&exec_id, &self.task_id, error)
            .await?;

        let state = self
            .persistence
            .load_task_state(&self.task_id)
            .await?
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Task {} not found", self.task_id).into()
            })?;

        let max_failures = state
            .metadata
            .get("max_failures")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(10);

        let should_pause = self
            .persistence
            .record_failure(&self.task_id, max_failures)
            .await?;

        if should_pause {
            warn!("Circuit breaker triggered");
        }

        Ok(should_pause)
    }

    /// Check if task should execute (not paused by circuit breaker).
    #[instrument(skip(self), fields(task_id = %self.task_id))]
    pub async fn should_execute(&self) -> ActorServerResult<bool> {
        self.persistence.should_execute(&self.task_id).await
    }

    /// Update next scheduled run time.
    #[instrument(skip(self), fields(task_id = %self.task_id, next_run = %next_run))]
    pub async fn update_next_run(&self, next_run: DateTime<Utc>) -> ActorServerResult<()> {
        debug!("Updating next run");
        self.persistence
            .update_next_run(&self.task_id, next_run)
            .await
    }
}
