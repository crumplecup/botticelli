//! Core actor implementation.

use crate::{
    ActorConfig, ActorError, ActorErrorKind, ActorResult, KnowledgeTable, Platform, SkillContext,
    SkillContextBuilder, SkillOutput, SkillRegistry,
};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Execution result from running an actor.
#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(setter(into))]
pub struct ExecutionResult {
    /// Successfully executed skills.
    #[builder(default)]
    pub succeeded: Vec<SkillOutput>,
    /// Failed skill executions with errors.
    #[builder(default)]
    pub failed: Vec<(String, ActorError)>,
    /// Skipped skills.
    #[builder(default)]
    pub skipped: Vec<String>,
}

/// Core actor that orchestrates skills and knowledge.
pub struct Actor {
    config: ActorConfig,
    skills: SkillRegistry,
    platform: Arc<dyn Platform>,
}

impl Actor {
    /// Create a new actor with builder pattern.
    ///
    /// # Returns
    ///
    /// Actor builder.
    pub fn builder() -> ActorBuilder {
        ActorBuilder::default()
    }

    /// Execute the actor workflow.
    ///
    /// Loads knowledge from configured tables, executes skills in order,
    /// and handles errors according to execution configuration.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool for knowledge queries and skill execution
    ///
    /// # Returns
    ///
    /// Execution result with succeeded, failed, and skipped operations.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Knowledge tables cannot be loaded
    /// - Unrecoverable error occurs with stop_on_unrecoverable=true
    #[tracing::instrument(skip(self, pool), fields(actor_name = %self.config.name()))]
    pub async fn execute(
        &self,
        pool: &Pool<ConnectionManager<PgConnection>>,
    ) -> ActorResult<ExecutionResult> {
        tracing::info!("Starting actor execution");

        // Get connection from pool for knowledge loading
        let mut conn = pool.get().map_err(|e| {
            ActorError::new(ActorErrorKind::DatabaseFailed(format!(
                "Failed to get database connection: {}",
                e
            )))
        })?;

        // Load knowledge from configured tables
        let knowledge = self.load_knowledge(&mut conn)?;

        let mut result = ExecutionResultBuilder::default()
            .build()
            .expect("ExecutionResult with valid defaults");

        // Execute each configured skill
        for skill_name in self.config.skills() {
            tracing::debug!(skill = %skill_name, "Preparing skill execution");

            // Check if skill is enabled in configuration
            if let Some(skill_config) = self.config.skill_configs().get(skill_name)
                && !skill_config.enabled()
            {
                tracing::info!(skill = %skill_name, "Skill disabled, skipping");
                result.skipped.push(skill_name.clone());
                continue;
            }

            // Build skill context
            let context = SkillContextBuilder::default()
                .knowledge(knowledge.clone())
                .config(self.extract_skill_config(skill_name))
                .platform(Arc::clone(&self.platform))
                .db_pool(pool.clone())
                .build()
                .expect("SkillContext with valid fields");

            // Execute skill with retry logic
            match self.execute_skill_with_retry(skill_name, &context).await {
                Ok(output) => {
                    tracing::info!(skill = %skill_name, "Skill executed successfully");
                    result.succeeded.push(output);
                }
                Err(error) => {
                    tracing::error!(
                        skill = %skill_name,
                        error = ?error,
                        recoverable = error.is_recoverable(),
                        "Skill execution failed"
                    );

                    result.failed.push((skill_name.clone(), error.clone()));

                    // Check if we should stop on unrecoverable errors
                    if !error.is_recoverable() && *self.config.execution().stop_on_unrecoverable() {
                        tracing::error!("Unrecoverable error, stopping execution");
                        return Err(error);
                    }

                    // Check if we should fail fast on any error
                    if !*self.config.execution().continue_on_error() {
                        tracing::error!("Continue on error disabled, stopping execution");
                        return Err(error);
                    }
                }
            }
        }

        tracing::info!(
            succeeded = result.succeeded.len(),
            failed = result.failed.len(),
            skipped = result.skipped.len(),
            "Actor execution completed"
        );

        Ok(result)
    }

    /// Load knowledge from configured tables.
    #[tracing::instrument(skip(self, conn))]
    fn load_knowledge(
        &self,
        conn: &mut PgConnection,
    ) -> ActorResult<HashMap<String, Vec<JsonValue>>> {
        tracing::debug!(
            table_count = self.config.knowledge().len(),
            "Loading knowledge tables"
        );

        let mut knowledge = HashMap::new();

        for table_name in self.config.knowledge() {
            let table = KnowledgeTable::new(table_name);

            // Check if table exists
            if !table.exists(conn) {
                tracing::warn!(table = %table_name, "Knowledge table does not exist");
                if *self.config.execution().stop_on_unrecoverable() {
                    return Err(ActorError::new(ActorErrorKind::KnowledgeTableNotFound(
                        table_name.clone(),
                    )));
                }
                continue;
            }

            // Query table data
            let rows = table.query(conn)?;
            tracing::debug!(table = %table_name, rows = rows.len(), "Loaded knowledge table");
            knowledge.insert(table_name.clone(), rows);
        }

        Ok(knowledge)
    }

    /// Extract skill-specific configuration as string map.
    #[tracing::instrument(skip(self))]
    fn extract_skill_config(&self, skill_name: &str) -> HashMap<String, String> {
        let mut config = HashMap::new();

        if let Some(skill_config) = self.config.skill_configs().get(skill_name) {
            // Convert JSON values to strings
            for (key, value) in skill_config.settings() {
                if let Some(s) = value.as_str() {
                    config.insert(key.clone(), s.to_string());
                } else {
                    config.insert(key.clone(), value.to_string());
                }
            }
        }

        config
    }

    /// Execute a skill with retry logic for recoverable errors.
    #[tracing::instrument(skip(self, context), fields(skill_name))]
    async fn execute_skill_with_retry(
        &self,
        skill_name: &str,
        context: &SkillContext,
    ) -> ActorResult<SkillOutput> {
        let max_retries = *self.config.execution().max_retries();

        let mut last_error = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::debug!(
                    attempt,
                    max_retries,
                    "Retrying skill execution after recoverable error"
                );
            }

            match self.skills.execute(skill_name, context).await {
                Ok(output) => return Ok(output),
                Err(error) => {
                    if !error.is_recoverable() {
                        tracing::error!("Unrecoverable error, cannot retry");
                        return Err(error);
                    }

                    if attempt < max_retries {
                        tracing::warn!(
                            attempt,
                            max_retries,
                            error = ?error,
                            "Recoverable error, will retry"
                        );
                    }

                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap())
    }
}

/// Builder for creating Actor instances.
#[derive(Default)]
pub struct ActorBuilder {
    config: Option<ActorConfig>,
    skills: Option<SkillRegistry>,
    platform: Option<Arc<dyn Platform>>,
}

impl ActorBuilder {
    /// Set actor configuration.
    pub fn config(mut self, config: ActorConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set skill registry.
    pub fn skills(mut self, skills: SkillRegistry) -> Self {
        self.skills = Some(skills);
        self
    }

    /// Set platform implementation.
    pub fn platform(mut self, platform: Arc<dyn Platform>) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Build the actor.
    ///
    /// # Returns
    ///
    /// Configured actor instance.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing.
    pub fn build(self) -> ActorResult<Actor> {
        let config = self.config.ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Actor config is required".to_string(),
            ))
        })?;

        let skills = self.skills.unwrap_or_default();

        let platform = self.platform.ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Platform implementation is required".to_string(),
            ))
        })?;

        Ok(Actor {
            config,
            skills,
            platform,
        })
    }
}
