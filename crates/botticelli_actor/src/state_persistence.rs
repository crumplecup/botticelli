//! Storage-backed state persistence for actor servers.

use async_trait::async_trait;
use botticelli_interface::{ActorServerExecutionRecord, ActorServerStateRecord, BotStorage};
use botticelli_server::{ActorServerResult, StatePersistence};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tracing::{debug, info, instrument};
use uuid::Uuid;

/// Execution result for logging (passed when completing an execution record).
#[derive(Debug, Clone)]
pub struct DatabaseExecutionResult {
    /// Number of skills that succeeded.
    pub skills_succeeded: i32,
    /// Number of skills that failed.
    pub skills_failed: i32,
    /// Number of skills that were skipped.
    pub skills_skipped: i32,
    /// Additional metadata as JSON.
    pub metadata: serde_json::Value,
}

/// Storage-backed state persistence using [`BotStorage`].
///
/// Stores actor server state and execution history via the `ActorStateStore`
/// interface. Works with both the redb (default) and PostgreSQL backends.
#[derive(Clone)]
pub struct BotStorageStatePersistence {
    storage: Arc<dyn BotStorage>,
}

impl std::fmt::Debug for BotStorageStatePersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotStorageStatePersistence").finish_non_exhaustive()
    }
}

impl BotStorageStatePersistence {
    /// Create a new persistence handler backed by the given storage.
    pub fn new(storage: Arc<dyn BotStorage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl StatePersistence for BotStorageStatePersistence {
    type State = ActorServerStateRecord;

    #[instrument(skip(self, state), fields(task_id = %state.task_id))]
    async fn save_state(&self, state: &Self::State) -> ActorServerResult<()> {
        debug!("Saving actor server state");
        self.storage
            .save_actor_state(state)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!("Actor server state saved");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn load_state(&self) -> ActorServerResult<Option<Self::State>> {
        debug!("Loading actor server state");
        let states = self
            .storage
            .list_actor_states()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        if states.is_empty() {
            return Ok(None);
        }
        info!(count = states.len(), "Loaded actor server states");
        Ok(states.into_iter().next())
    }

    #[instrument(skip(self))]
    async fn clear_state(&self) -> ActorServerResult<()> {
        debug!("Clearing all actor server state");
        let states = self
            .storage
            .list_actor_states()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        for state in states {
            self.storage
                .delete_actor_state(&state.task_id)
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        }
        info!("Cleared all actor server state");
        Ok(())
    }
}

impl BotStorageStatePersistence {
    /// Save state for a specific task.
    #[instrument(skip(self, state), fields(task_id))]
    pub async fn save_task_state(
        &self,
        task_id: &str,
        state: &ActorServerStateRecord,
    ) -> ActorServerResult<()> {
        debug!(task_id, "Saving task state");
        self.storage
            .save_actor_state(state)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(task_id, "Task state saved");
        Ok(())
    }

    /// Load state for a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn load_task_state(
        &self,
        task_id: &str,
    ) -> ActorServerResult<Option<ActorServerStateRecord>> {
        debug!(task_id, "Loading task state");
        let state = self
            .storage
            .get_actor_state(task_id)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        if state.is_some() {
            info!(task_id, "Task state loaded");
        } else {
            debug!(task_id, "No state found for task");
        }
        Ok(state)
    }

    /// Delete state for a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn delete_task_state(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Deleting task state");
        self.storage
            .delete_actor_state(task_id)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(task_id, "Task state deleted");
        Ok(())
    }

    /// List all tasks.
    #[instrument(skip(self))]
    pub async fn list_all_tasks(&self) -> ActorServerResult<Vec<ActorServerStateRecord>> {
        debug!("Listing all tasks");
        let tasks = self
            .storage
            .list_actor_states()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(count = tasks.len(), "Listed all tasks");
        Ok(tasks)
    }

    /// List all tasks for a specific actor.
    #[instrument(skip(self), fields(actor_name))]
    pub async fn list_tasks_by_actor(
        &self,
        actor_name: &str,
    ) -> ActorServerResult<Vec<ActorServerStateRecord>> {
        debug!(actor_name, "Listing tasks for actor");
        let all = self.list_all_tasks().await?;
        let tasks: Vec<_> = all
            .into_iter()
            .filter(|t| t.actor_name == actor_name)
            .collect();
        info!(actor_name, count = tasks.len(), "Listed tasks for actor");
        Ok(tasks)
    }

    /// List all active (non-paused) tasks.
    #[instrument(skip(self))]
    pub async fn list_active_tasks(&self) -> ActorServerResult<Vec<ActorServerStateRecord>> {
        debug!("Listing active tasks");
        let all = self.list_all_tasks().await?;
        let tasks: Vec<_> = all.into_iter().filter(|t| !t.is_paused).collect();
        info!(count = tasks.len(), "Listed active tasks");
        Ok(tasks)
    }

    /// List all paused tasks.
    #[instrument(skip(self))]
    pub async fn list_paused_tasks(&self) -> ActorServerResult<Vec<ActorServerStateRecord>> {
        debug!("Listing paused tasks");
        let all = self.list_all_tasks().await?;
        let tasks: Vec<_> = all.into_iter().filter(|t| t.is_paused).collect();
        info!(count = tasks.len(), "Listed paused tasks");
        Ok(tasks)
    }

    /// Pause a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn pause_task(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Pausing task");
        let mut state = self
            .load_task_state(task_id)
            .await?
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Task {task_id} not found").into()
            })?;
        state.is_paused = true;
        state.updated_at = Utc::now();
        self.save_task_state(task_id, &state).await?;
        info!(task_id, "Task paused");
        Ok(())
    }

    /// Resume a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn resume_task(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Resuming task");
        let mut state = self
            .load_task_state(task_id)
            .await?
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Task {task_id} not found").into()
            })?;
        state.is_paused = false;
        state.updated_at = Utc::now();
        self.save_task_state(task_id, &state).await?;
        info!(task_id, "Task resumed");
        Ok(())
    }

    /// Update the next run time for a specific task.
    #[instrument(skip(self), fields(task_id, next_run = %next_run))]
    pub async fn update_next_run(
        &self,
        task_id: &str,
        next_run: DateTime<Utc>,
    ) -> ActorServerResult<()> {
        debug!(task_id, next_run = %next_run, "Updating next run time");
        let mut state = self
            .load_task_state(task_id)
            .await?
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Task {task_id} not found").into()
            })?;
        state.next_run = next_run;
        state.updated_at = Utc::now();
        self.save_task_state(task_id, &state).await?;
        info!(task_id, next_run = %next_run, "Next run time updated");
        Ok(())
    }

    /// Start a new execution and return the execution ID (UUID string).
    #[instrument(skip(self), fields(task_id, actor_name))]
    pub async fn start_execution(
        &self,
        task_id: &str,
        actor_name: &str,
    ) -> ActorServerResult<String> {
        debug!(task_id, actor_name, "Starting execution");
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let record = ActorServerExecutionRecord {
            id: id.clone(),
            task_id: task_id.to_string(),
            actor_name: actor_name.to_string(),
            started_at: now,
            completed_at: None,
            success: false,
            error_message: None,
            skills_succeeded: 0,
            skills_failed: 0,
            skills_skipped: 0,
            metadata: serde_json::json!({}),
            created_at: now,
        };
        self.storage
            .save_actor_execution(&record)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(task_id, actor_name, execution_id = %id, "Execution started");
        Ok(id)
    }

    /// Complete an execution with a result.
    #[instrument(skip(self, result), fields(execution_id, task_id))]
    pub async fn complete_execution(
        &self,
        exec_id: &str,
        task_id: &str,
        result: DatabaseExecutionResult,
    ) -> ActorServerResult<()> {
        debug!(exec_id, "Completing execution");
        let executions = self
            .storage
            .list_actor_executions(task_id, usize::MAX)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let mut record = executions
            .into_iter()
            .find(|e| e.id == exec_id)
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Execution {exec_id} not found for task {task_id}").into()
            })?;
        record.completed_at = Some(Utc::now());
        record.success = true;
        record.skills_succeeded = result.skills_succeeded;
        record.skills_failed = result.skills_failed;
        record.skills_skipped = result.skills_skipped;
        record.metadata = result.metadata;
        self.storage
            .save_actor_execution(&record)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(exec_id, "Execution completed successfully");
        Ok(())
    }

    /// Mark an execution as failed with an error message.
    #[instrument(skip(self), fields(execution_id, task_id, error))]
    pub async fn fail_execution(
        &self,
        exec_id: &str,
        task_id: &str,
        error: &str,
    ) -> ActorServerResult<()> {
        debug!(exec_id, error, "Failing execution");
        let executions = self
            .storage
            .list_actor_executions(task_id, usize::MAX)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let mut record = executions
            .into_iter()
            .find(|e| e.id == exec_id)
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Execution {exec_id} not found for task {task_id}").into()
            })?;
        record.completed_at = Some(Utc::now());
        record.success = false;
        record.error_message = Some(error.to_string());
        self.storage
            .save_actor_execution(&record)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(exec_id, error, "Execution marked as failed");
        Ok(())
    }

    /// Get execution history for a specific task.
    #[instrument(skip(self), fields(task_id, limit))]
    pub async fn get_execution_history(
        &self,
        task_id: &str,
        limit: i64,
    ) -> ActorServerResult<Vec<ActorServerExecutionRecord>> {
        debug!(task_id, limit, "Getting execution history");
        let executions = self
            .storage
            .list_actor_executions(task_id, limit as usize)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(task_id, count = executions.len(), "Retrieved execution history");
        Ok(executions)
    }

    /// Get failed executions for a specific task.
    #[instrument(skip(self), fields(task_id, limit))]
    pub async fn get_failed_executions(
        &self,
        task_id: &str,
        limit: i64,
    ) -> ActorServerResult<Vec<ActorServerExecutionRecord>> {
        debug!(task_id, limit, "Getting failed executions");
        let all = self
            .storage
            .list_actor_executions(task_id, usize::MAX)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let executions: Vec<_> = all
            .into_iter()
            .filter(|e| !e.success)
            .take(limit as usize)
            .collect();
        info!(task_id, count = executions.len(), "Retrieved failed executions");
        Ok(executions)
    }

    /// Get recent executions across all tasks.
    #[instrument(skip(self), fields(limit))]
    pub async fn get_recent_executions(
        &self,
        limit: i64,
    ) -> ActorServerResult<Vec<ActorServerExecutionRecord>> {
        debug!(limit, "Getting recent executions");
        let executions = self
            .storage
            .list_all_actor_executions(limit as usize)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        info!(count = executions.len(), "Retrieved recent executions");
        Ok(executions)
    }

    /// Prune execution records older than the specified number of days.
    ///
    /// Returns the number of executions deleted.
    #[instrument(skip(self), fields(older_than_days))]
    pub async fn prune_old_executions(&self, older_than_days: i32) -> ActorServerResult<usize> {
        debug!(older_than_days, "Pruning old executions");
        let cutoff = Utc::now() - Duration::days(i64::from(older_than_days));
        let all = self
            .storage
            .list_all_actor_executions(usize::MAX)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let to_delete: Vec<_> = all
            .into_iter()
            .filter(|e| e.started_at < cutoff)
            .collect();
        let count = to_delete.len();
        for exec in to_delete {
            self.storage
                .delete_actor_execution(&exec.id)
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        }
        info!(deleted = count, older_than_days, "Pruned old executions");
        Ok(count)
    }

    /// Record a failure for a task and increment the consecutive failure counter.
    ///
    /// Returns `true` if the failure count has reached or exceeded `max_failures`.
    #[instrument(skip(self), fields(task_id, max_failures))]
    pub async fn record_failure(
        &self,
        task_id: &str,
        max_failures: i32,
    ) -> ActorServerResult<bool> {
        debug!(task_id, max_failures, "Recording task failure");
        let mut state = self
            .load_task_state(task_id)
            .await?
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Task {task_id} not found").into()
            })?;
        state.consecutive_failures += 1;
        state.updated_at = Utc::now();
        let threshold_exceeded = state.consecutive_failures >= max_failures;
        if threshold_exceeded {
            state.is_paused = true;
            info!(task_id, max_failures, "Task failure threshold exceeded, pausing task");
        } else {
            debug!(task_id, "Task failure recorded");
        }
        self.save_task_state(task_id, &state).await?;
        Ok(threshold_exceeded)
    }

    /// Record a success for a task and reset the consecutive failure counter.
    #[instrument(skip(self), fields(task_id))]
    pub async fn record_success(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Recording task success");
        let mut state = self
            .load_task_state(task_id)
            .await?
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Task {task_id} not found").into()
            })?;
        state.consecutive_failures = 0;
        state.updated_at = Utc::now();
        self.save_task_state(task_id, &state).await?;
        info!(task_id, "Task success recorded, failure counter reset");
        Ok(())
    }

    /// Check if a task should execute (not paused).
    #[instrument(skip(self), fields(task_id))]
    pub async fn should_execute(&self, task_id: &str) -> ActorServerResult<bool> {
        debug!(task_id, "Checking if task should execute");
        let should_run = self
            .load_task_state(task_id)
            .await?
            .map(|s| !s.is_paused)
            .unwrap_or(true);
        if should_run {
            debug!(task_id, "Task should execute");
        } else {
            debug!(task_id, "Task is paused, skipping execution");
        }
        Ok(should_run)
    }
}
