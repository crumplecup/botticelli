//! Core actor implementation.

use crate::{
    ActorConfig, ActorError, ActorErrorKind, ActorResult, KnowledgeTable, Platform, SkillContext,
    SkillContextBuilder, SkillOutput, SkillRegistry,
};
use botticelli_interface::BotStorage;
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
    /// * `storage` - Storage backend for knowledge queries and skill execution
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
    #[tracing::instrument(
        skip(self, storage),
        fields(
            actor_name = %self.config.name(),
            skill_count = self.config.skills().len(),
            knowledge_tables = self.config.knowledge().len(),
        )
    )]
    pub async fn execute(&self, storage: &Arc<dyn BotStorage>) -> ActorResult<ExecutionResult> {
        tracing::info!("Starting actor execution");

        let knowledge = {
            let _span = tracing::debug_span!(
                "load_knowledge",
                table_count = self.config.knowledge().len()
            )
            .entered();
            self.load_knowledge(storage).await?
        };

        let mut result = ExecutionResultBuilder::default()
            .build()
            .expect("ExecutionResult with valid defaults");

        for skill_name in self.config.skills() {
            let skill_span = tracing::info_span!("execute_skill", skill = %skill_name);
            let _enter = skill_span.enter();

            tracing::debug!("Preparing skill execution");

            if let Some(skill_config) = self.config.skill_configs().get(skill_name)
                && !skill_config.enabled()
            {
                tracing::info!(skill = %skill_name, "Skill disabled, skipping");
                result.skipped.push(skill_name.clone());
                continue;
            }

            let context = SkillContextBuilder::default()
                .knowledge(knowledge.clone())
                .config(self.extract_skill_config(skill_name))
                .platform(Arc::clone(&self.platform))
                .storage(Arc::clone(storage))
                .build()
                .expect("SkillContext with valid fields");

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

                    if !error.is_recoverable() && *self.config.execution().stop_on_unrecoverable() {
                        tracing::error!("Unrecoverable error, stopping execution");
                        return Err(error);
                    }

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
    #[tracing::instrument(skip(self, storage))]
    async fn load_knowledge(
        &self,
        storage: &Arc<dyn BotStorage>,
    ) -> ActorResult<HashMap<String, Vec<JsonValue>>> {
        tracing::debug!(
            table_count = self.config.knowledge().len(),
            "Loading knowledge tables"
        );

        let mut knowledge = HashMap::new();

        for table_name in self.config.knowledge() {
            let table = KnowledgeTable::new(table_name);

            if !table.exists(storage).await {
                tracing::warn!(table = %table_name, "Knowledge table does not exist");
                if *self.config.execution().stop_on_unrecoverable() {
                    return Err(ActorError::new(ActorErrorKind::KnowledgeTableNotFound(
                        table_name.clone(),
                    )));
                }
                continue;
            }

            let rows = table.query(storage).await?;
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
    #[tracing::instrument(
        skip(self, context),
        fields(
            skill_name = %skill_name,
            max_retries = %self.config.execution().max_retries(),
            has_config = self.config.skill_configs().contains_key(skill_name),
        )
    )]
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
                Ok(output) => {
                    if attempt > 0 {
                        tracing::info!(attempt, "Skill succeeded after retry");
                    }
                    return Ok(output);
                }
                Err(error) => {
                    tracing::Span::current().record("error", format!("{}", error));
                    tracing::Span::current().record("error_recoverable", error.is_recoverable());

                    if !error.is_recoverable() {
                        tracing::error!(
                            error = %error,
                            "Unrecoverable error, cannot retry"
                        );
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

        Err(last_error.expect("loop always sets last_error before exhausting retries"))
    }
}

/// Builder for [`Actor`].
#[derive(Default)]
pub struct ActorBuilder {
    config: Option<ActorConfig>,
    skills: Option<SkillRegistry>,
    platform: Option<Arc<dyn Platform>>,
}

impl ActorBuilder {
    /// Set the actor configuration.
    pub fn config(mut self, config: ActorConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the skill registry.
    pub fn skills(mut self, skills: SkillRegistry) -> Self {
        self.skills = Some(skills);
        self
    }

    /// Set the platform.
    pub fn platform(mut self, platform: Arc<dyn Platform>) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Build the actor.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing.
    pub fn build(self) -> ActorResult<Actor> {
        Ok(Actor {
            config: self.config.ok_or_else(|| {
                ActorError::new(ActorErrorKind::InvalidConfiguration(
                    "Actor config is required".to_string(),
                ))
            })?,
            skills: self.skills.ok_or_else(|| {
                ActorError::new(ActorErrorKind::InvalidConfiguration(
                    "Skill registry is required".to_string(),
                ))
            })?,
            platform: self.platform.ok_or_else(|| {
                ActorError::new(ActorErrorKind::InvalidConfiguration(
                    "Platform is required".to_string(),
                ))
            })?,
        })
    }
}
