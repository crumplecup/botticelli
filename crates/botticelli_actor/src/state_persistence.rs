//! Database-backed state persistence for actor servers.

use async_trait::async_trait;
use botticelli_database::{
    ActorServerExecutionRow, ActorServerStateRow, NewActorServerExecutionBuilder,
    NewActorServerStateBuilder,
};
use botticelli_server::{ActorServerResult, StatePersistence};
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::{debug, info, instrument};

/// Database execution result for logging.
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

/// Database-backed state persistence using PostgreSQL with connection pooling.
///
/// Stores actor server state in the `actor_server_state` table for
/// recovery after server restarts. Uses r2d2 connection pooling for
/// efficient concurrent database access.
///
/// Note: Requires DATABASE_URL environment variable to be set.
#[derive(Debug, Clone)]
pub struct DatabaseStatePersistence {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DatabaseStatePersistence {
    /// Create a new database state persistence handler with a connection pool.
    ///
    /// Requires the DATABASE_URL environment variable to be set.
    /// Uses a default pool size of 10 connections.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_actor::DatabaseStatePersistence;
    ///
    /// // Requires DATABASE_URL=postgresql://localhost/botticelli in environment
    /// let persistence = DatabaseStatePersistence::new()
    ///     .expect("Failed to create persistence");
    /// ```
    pub fn new() -> ActorServerResult<Self> {
        Self::with_pool_size(10)
    }

    /// Create a new database state persistence handler with custom pool size.
    ///
    /// Useful for testing or specific deployment scenarios.
    ///
    /// # Arguments
    ///
    /// * `pool_size` - Maximum number of connections in the pool (1-100)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_actor::DatabaseStatePersistence;
    ///
    /// // Create with smaller pool for testing
    /// let persistence = DatabaseStatePersistence::with_pool_size(5)
    ///     .expect("Failed to create persistence");
    /// ```
    pub fn with_pool_size(pool_size: u32) -> ActorServerResult<Self> {
        let database_url = std::env::var("DATABASE_URL").map_err(
            |_| -> Box<dyn std::error::Error + Send + Sync> {
                "DATABASE_URL environment variable not set".into()
            },
        )?;

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().max_size(pool_size).build(manager).map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to create connection pool: {}", e).into()
            },
        )?;

        // Warm up the pool by getting and immediately releasing a connection
        {
            let _conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to warm up connection pool: {}", e).into()
                })?;
        }

        Ok(Self { pool })
    }
}

#[async_trait]
impl StatePersistence for DatabaseStatePersistence {
    type State = ActorServerStateRow;

    #[instrument(skip(self, state), fields(task_id = %state.task_id))]
    async fn save_state(&self, state: &Self::State) -> ActorServerResult<()> {
        debug!("Saving actor server state to database");

        // Run blocking database operation in dedicated thread pool
        let state = state.clone();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            // Use INSERT ... ON CONFLICT to upsert state
            diesel::insert_into(botticelli_database::schema::actor_server_state::table)
                .values(
                    &NewActorServerStateBuilder::default()
                        .task_id(&state.task_id)
                        .actor_name(&state.actor_name)
                        .last_run(state.last_run)
                        .next_run(state.next_run)
                        .consecutive_failures(state.consecutive_failures.unwrap_or(0))
                        .is_paused(state.is_paused.unwrap_or(false))
                        .metadata(state.metadata.clone().unwrap_or_default())
                        .build()
                        .expect("NewActorServerState with valid fields"),
                )
                .on_conflict(botticelli_database::schema::actor_server_state::task_id)
                .do_update()
                .set((
                    botticelli_database::schema::actor_server_state::last_run.eq(&state.last_run),
                    botticelli_database::schema::actor_server_state::next_run.eq(&state.next_run),
                    botticelli_database::schema::actor_server_state::consecutive_failures
                        .eq(&state.consecutive_failures),
                    botticelli_database::schema::actor_server_state::is_paused.eq(&state.is_paused),
                    botticelli_database::schema::actor_server_state::metadata.eq(&state.metadata),
                    botticelli_database::schema::actor_server_state::updated_at
                        .eq(diesel::dsl::now),
                ))
                .execute(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to save state: {}", e).into()
                })?;

            info!("Actor server state saved to database");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    #[instrument(skip(self))]
    async fn load_state(&self) -> ActorServerResult<Option<Self::State>> {
        debug!("Loading actor server state from database");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Option<ActorServerStateRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            // Load all state rows (for now, just get the first one)
            let states = botticelli_database::schema::actor_server_state::table
                .load::<ActorServerStateRow>(&mut conn)
                .optional()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to load state: {}", e).into()
                })?;

            if let Some(states_vec) = states {
                if !states_vec.is_empty() {
                    info!(count = states_vec.len(), "Loaded actor server states");
                    Ok(Some(states_vec.into_iter().next().unwrap()))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    #[instrument(skip(self))]
    async fn clear_state(&self) -> ActorServerResult<()> {
        debug!("Clearing all actor server state from database");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::delete(botticelli_database::schema::actor_server_state::table)
                .execute(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to clear state: {}", e).into()
                })?;

            info!("Cleared all actor server state");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }
}

impl DatabaseStatePersistence {
    /// Save state for a specific task.
    #[instrument(skip(self, state), fields(task_id))]
    pub async fn save_task_state(
        &self,
        task_id: &str,
        state: &ActorServerStateRow,
    ) -> ActorServerResult<()> {
        debug!(task_id, "Saving task state to database");

        let state = state.clone();
        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::insert_into(botticelli_database::schema::actor_server_state::table)
                .values(
                    &NewActorServerStateBuilder::default()
                        .task_id(&task_id)
                        .actor_name(&state.actor_name)
                        .last_run(state.last_run)
                        .next_run(state.next_run)
                        .consecutive_failures(state.consecutive_failures.unwrap_or(0))
                        .is_paused(state.is_paused.unwrap_or(false))
                        .metadata(state.metadata.clone().unwrap_or_default())
                        .build()
                        .expect("NewActorServerState with valid fields"),
                )
                .on_conflict(botticelli_database::schema::actor_server_state::task_id)
                .do_update()
                .set((
                    botticelli_database::schema::actor_server_state::last_run.eq(&state.last_run),
                    botticelli_database::schema::actor_server_state::next_run.eq(&state.next_run),
                    botticelli_database::schema::actor_server_state::consecutive_failures
                        .eq(&state.consecutive_failures),
                    botticelli_database::schema::actor_server_state::is_paused.eq(&state.is_paused),
                    botticelli_database::schema::actor_server_state::metadata.eq(&state.metadata),
                    botticelli_database::schema::actor_server_state::updated_at
                        .eq(diesel::dsl::now),
                ))
                .execute(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to save task state: {}", e).into()
                })?;

            info!(task_id, "Task state saved to database");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Load state for a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn load_task_state(
        &self,
        task_id: &str,
    ) -> ActorServerResult<Option<ActorServerStateRow>> {
        debug!(task_id, "Loading task state from database");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Option<ActorServerStateRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let state = botticelli_database::schema::actor_server_state::table
                .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id))
                .first::<ActorServerStateRow>(&mut conn)
                .optional()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to load task state: {}", e).into()
                })?;

            if state.is_some() {
                info!(task_id, "Task state loaded from database");
            } else {
                debug!(task_id, "No state found for task");
            }

            Ok(state)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Delete state for a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn delete_task_state(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Deleting task state from database");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::delete(
                botticelli_database::schema::actor_server_state::table
                    .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id)),
            )
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to delete task state: {}", e).into()
            })?;

            info!(task_id, "Task state deleted from database");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// List all tasks in the database.
    #[instrument(skip(self))]
    pub async fn list_all_tasks(&self) -> ActorServerResult<Vec<ActorServerStateRow>> {
        debug!("Listing all tasks from database");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerStateRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let tasks = botticelli_database::schema::actor_server_state::table
                .load::<ActorServerStateRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to list tasks: {}", e).into()
                })?;

            info!(count = tasks.len(), "Listed all tasks from database");
            Ok(tasks)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// List all tasks for a specific actor.
    #[instrument(skip(self), fields(actor_name))]
    pub async fn list_tasks_by_actor(
        &self,
        actor_name: &str,
    ) -> ActorServerResult<Vec<ActorServerStateRow>> {
        debug!(actor_name, "Listing tasks for actor");

        let actor_name = actor_name.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerStateRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let tasks = botticelli_database::schema::actor_server_state::table
                .filter(botticelli_database::schema::actor_server_state::actor_name.eq(&actor_name))
                .load::<ActorServerStateRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to list tasks by actor: {}", e).into()
                })?;

            info!(actor_name, count = tasks.len(), "Listed tasks for actor");
            Ok(tasks)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// List all active (non-paused) tasks.
    #[instrument(skip(self))]
    pub async fn list_active_tasks(&self) -> ActorServerResult<Vec<ActorServerStateRow>> {
        debug!("Listing active tasks");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerStateRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let tasks = botticelli_database::schema::actor_server_state::table
                .filter(
                    botticelli_database::schema::actor_server_state::is_paused
                        .eq(false)
                        .or(botticelli_database::schema::actor_server_state::is_paused.is_null()),
                )
                .load::<ActorServerStateRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to list active tasks: {}", e).into()
                })?;

            info!(count = tasks.len(), "Listed active tasks");
            Ok(tasks)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// List all paused tasks.
    #[instrument(skip(self))]
    pub async fn list_paused_tasks(&self) -> ActorServerResult<Vec<ActorServerStateRow>> {
        debug!("Listing paused tasks");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerStateRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let tasks = botticelli_database::schema::actor_server_state::table
                .filter(botticelli_database::schema::actor_server_state::is_paused.eq(true))
                .load::<ActorServerStateRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to list paused tasks: {}", e).into()
                })?;

            info!(count = tasks.len(), "Listed paused tasks");
            Ok(tasks)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Pause a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn pause_task(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Pausing task");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::update(
                botticelli_database::schema::actor_server_state::table
                    .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id)),
            )
            .set((
                botticelli_database::schema::actor_server_state::is_paused.eq(true),
                botticelli_database::schema::actor_server_state::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to pause task: {}", e).into()
            })?;

            info!(task_id, "Task paused");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Resume a specific task.
    #[instrument(skip(self), fields(task_id))]
    pub async fn resume_task(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Resuming task");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::update(
                botticelli_database::schema::actor_server_state::table
                    .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id)),
            )
            .set((
                botticelli_database::schema::actor_server_state::is_paused.eq(false),
                botticelli_database::schema::actor_server_state::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to resume task: {}", e).into()
            })?;

            info!(task_id, "Task resumed");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Update the next run time for a specific task.
    #[instrument(skip(self), fields(task_id, next_run = %next_run))]
    pub async fn update_next_run(
        &self,
        task_id: &str,
        next_run: NaiveDateTime,
    ) -> ActorServerResult<()> {
        debug!(task_id, next_run = %next_run, "Updating next run time");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::update(
                botticelli_database::schema::actor_server_state::table
                    .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id)),
            )
            .set((
                botticelli_database::schema::actor_server_state::next_run.eq(&next_run),
                botticelli_database::schema::actor_server_state::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to update next run time: {}", e).into()
            })?;

            info!(task_id, next_run = %next_run, "Next run time updated");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Start a new execution and return the execution ID.
    #[instrument(skip(self), fields(task_id, actor_name))]
    pub async fn start_execution(&self, task_id: &str, actor_name: &str) -> ActorServerResult<i64> {
        debug!(task_id, actor_name, "Starting execution");

        let task_id = task_id.to_string();
        let actor_name = actor_name.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<i64> {
            use diesel::Connection;

            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let execution = NewActorServerExecutionBuilder::default()
                .task_id(task_id.clone())
                .actor_name(actor_name.clone())
                .started_at(Utc::now().naive_utc())
                .build()
                .expect("NewActorServerExecution with valid fields");

            // Use explicit transaction to ensure commit
            let id =
                conn.transaction::<i64, Box<dyn std::error::Error + Send + Sync>, _>(|conn| {
                    let id = diesel::insert_into(
                        botticelli_database::schema::actor_server_executions::table,
                    )
                    .values(&execution)
                    .returning(botticelli_database::schema::actor_server_executions::id)
                    .get_result::<i64>(conn)
                    .map_err(
                        |e| -> Box<dyn std::error::Error + Send + Sync> {
                            format!("Failed to start execution: {}", e).into()
                        },
                    )?;
                    Ok(id)
                })?;

            info!(task_id, actor_name, execution_id = id, "Execution started");
            Ok(id)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Complete an execution with a result.
    #[instrument(skip(self, result), fields(execution_id))]
    pub async fn complete_execution(
        &self,
        execution_id: i64,
        result: DatabaseExecutionResult,
    ) -> ActorServerResult<()> {
        debug!(execution_id, "Completing execution");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::update(
                botticelli_database::schema::actor_server_executions::table.filter(
                    botticelli_database::schema::actor_server_executions::id.eq(execution_id),
                ),
            )
            .set((
                botticelli_database::schema::actor_server_executions::completed_at
                    .eq(Utc::now().naive_utc()),
                botticelli_database::schema::actor_server_executions::success.eq(true),
                botticelli_database::schema::actor_server_executions::skills_succeeded
                    .eq(result.skills_succeeded),
                botticelli_database::schema::actor_server_executions::skills_failed
                    .eq(result.skills_failed),
                botticelli_database::schema::actor_server_executions::skills_skipped
                    .eq(result.skills_skipped),
                botticelli_database::schema::actor_server_executions::metadata.eq(&result.metadata),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to complete execution: {}", e).into()
            })?;

            info!(execution_id, "Execution completed successfully");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Mark an execution as failed with an error message.
    #[instrument(skip(self), fields(execution_id, error))]
    pub async fn fail_execution(&self, execution_id: i64, error: &str) -> ActorServerResult<()> {
        debug!(execution_id, error, "Failing execution");

        let error = error.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::update(
                botticelli_database::schema::actor_server_executions::table.filter(
                    botticelli_database::schema::actor_server_executions::id.eq(execution_id),
                ),
            )
            .set((
                botticelli_database::schema::actor_server_executions::completed_at
                    .eq(Utc::now().naive_utc()),
                botticelli_database::schema::actor_server_executions::success.eq(false),
                botticelli_database::schema::actor_server_executions::error_message.eq(&error),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to fail execution: {}", e).into()
            })?;

            info!(execution_id, error, "Execution marked as failed");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Get execution history for a specific task.
    #[instrument(skip(self), fields(task_id, limit))]
    pub async fn get_execution_history(
        &self,
        task_id: &str,
        limit: i64,
    ) -> ActorServerResult<Vec<ActorServerExecutionRow>> {
        debug!(task_id, limit, "Getting execution history");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerExecutionRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let executions = botticelli_database::schema::actor_server_executions::table
                .filter(botticelli_database::schema::actor_server_executions::task_id.eq(&task_id))
                .order(botticelli_database::schema::actor_server_executions::started_at.desc())
                .limit(limit)
                .load::<ActorServerExecutionRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get execution history: {}", e).into()
                })?;

            info!(
                task_id,
                count = executions.len(),
                "Retrieved execution history"
            );
            Ok(executions)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Get failed executions for a specific task.
    #[instrument(skip(self), fields(task_id, limit))]
    pub async fn get_failed_executions(
        &self,
        task_id: &str,
        limit: i64,
    ) -> ActorServerResult<Vec<ActorServerExecutionRow>> {
        debug!(task_id, limit, "Getting failed executions");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerExecutionRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let executions = botticelli_database::schema::actor_server_executions::table
                .filter(botticelli_database::schema::actor_server_executions::task_id.eq(&task_id))
                .filter(botticelli_database::schema::actor_server_executions::success.eq(false))
                .order(botticelli_database::schema::actor_server_executions::started_at.desc())
                .limit(limit)
                .load::<ActorServerExecutionRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get failed executions: {}", e).into()
                })?;

            info!(
                task_id,
                count = executions.len(),
                "Retrieved failed executions"
            );
            Ok(executions)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Get recent executions across all tasks.
    #[instrument(skip(self), fields(limit))]
    pub async fn get_recent_executions(
        &self,
        limit: i64,
    ) -> ActorServerResult<Vec<ActorServerExecutionRow>> {
        debug!(limit, "Getting recent executions");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<Vec<ActorServerExecutionRow>> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let executions = botticelli_database::schema::actor_server_executions::table
                .order(botticelli_database::schema::actor_server_executions::started_at.desc())
                .limit(limit)
                .load::<ActorServerExecutionRow>(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get recent executions: {}", e).into()
                })?;

            info!(count = executions.len(), "Retrieved recent executions");
            Ok(executions)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Prune old executions older than the specified number of days.
    ///
    /// Returns the number of executions deleted.
    #[instrument(skip(self), fields(older_than_days))]
    pub async fn prune_old_executions(&self, older_than_days: i32) -> ActorServerResult<usize> {
        debug!(older_than_days, "Pruning old executions");

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<usize> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let cutoff =
                Utc::now().naive_utc() - chrono::Duration::days(i64::from(older_than_days));

            let deleted = diesel::delete(
                botticelli_database::schema::actor_server_executions::table.filter(
                    botticelli_database::schema::actor_server_executions::started_at.lt(cutoff),
                ),
            )
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to prune old executions: {}", e).into()
            })?;

            info!(deleted, older_than_days, "Pruned old executions");
            Ok(deleted)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Record a failure for a task and increment consecutive failure counter.
    ///
    /// Returns `true` if the failure count has exceeded the configured threshold.
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task identifier
    /// * `max_failures` - Maximum consecutive failures threshold
    #[instrument(skip(self), fields(task_id, max_failures))]
    pub async fn record_failure(
        &self,
        task_id: &str,
        max_failures: i32,
    ) -> ActorServerResult<bool> {
        debug!(task_id, max_failures, "Recording task failure");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<bool> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            // Get current failure count
            let current_failures = botticelli_database::schema::actor_server_state::table
                .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id))
                .select(botticelli_database::schema::actor_server_state::consecutive_failures)
                .first::<Option<i32>>(&mut conn)
                .optional()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get failure count: {}", e).into()
                })?
                .flatten()
                .unwrap_or(0);

            // Increment failure counter
            diesel::update(
                botticelli_database::schema::actor_server_state::table
                    .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id)),
            )
            .set((
                botticelli_database::schema::actor_server_state::consecutive_failures
                    .eq(current_failures + 1),
                botticelli_database::schema::actor_server_state::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to record failure: {}", e).into()
            })?;

            // Check current failure count
            let state = botticelli_database::schema::actor_server_state::table
                .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id))
                .first::<ActorServerStateRow>(&mut conn)
                .optional()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to check failure count: {}", e).into()
                })?;

            let threshold_exceeded = state
                .and_then(|s| s.consecutive_failures)
                .map(|count| count >= max_failures)
                .unwrap_or(false);

            if threshold_exceeded {
                info!(
                    task_id,
                    max_failures, "Task failure threshold exceeded, pausing task"
                );
                // Pause the task
                diesel::update(
                    botticelli_database::schema::actor_server_state::table.filter(
                        botticelli_database::schema::actor_server_state::task_id.eq(&task_id),
                    ),
                )
                .set((
                    botticelli_database::schema::actor_server_state::is_paused.eq(true),
                    botticelli_database::schema::actor_server_state::updated_at
                        .eq(diesel::dsl::now),
                ))
                .execute(&mut conn)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to pause task: {}", e).into()
                })?;
            } else {
                debug!(task_id, "Task failure recorded");
            }

            Ok(threshold_exceeded)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Record a success for a task and reset consecutive failure counter.
    #[instrument(skip(self), fields(task_id))]
    pub async fn record_success(&self, task_id: &str) -> ActorServerResult<()> {
        debug!(task_id, "Recording task success");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<()> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            diesel::update(
                botticelli_database::schema::actor_server_state::table
                    .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id)),
            )
            .set((
                botticelli_database::schema::actor_server_state::consecutive_failures.eq(0),
                botticelli_database::schema::actor_server_state::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to record success: {}", e).into()
            })?;

            info!(task_id, "Task success recorded, failure counter reset");
            Ok(())
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }

    /// Check if a task should execute based on pause state.
    ///
    /// Returns `false` if task is paused, `true` otherwise.
    #[instrument(skip(self), fields(task_id))]
    pub async fn should_execute(&self, task_id: &str) -> ActorServerResult<bool> {
        debug!(task_id, "Checking if task should execute");

        let task_id = task_id.to_string();

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> ActorServerResult<bool> {
            let mut conn = pool
                .get()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to get connection from pool: {}", e).into()
                })?;

            let state = botticelli_database::schema::actor_server_state::table
                .filter(botticelli_database::schema::actor_server_state::task_id.eq(&task_id))
                .first::<ActorServerStateRow>(&mut conn)
                .optional()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to check task state: {}", e).into()
                })?;

            let should_run = state
                .and_then(|s| s.is_paused)
                .map(|paused| !paused)
                .unwrap_or(true);

            if should_run {
                debug!(task_id, "Task should execute");
            } else {
                debug!(task_id, "Task is paused, skipping execution");
            }

            Ok(should_run)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Task join error: {}", e).into()
        })?
    }
}
