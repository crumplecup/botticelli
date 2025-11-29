//! Narrative execution skill for running narrative workflows.

use crate::{
    ActorError, ActorErrorKind, Skill, SkillContext, SkillOutput, SkillOutputBuilder, SkillResult,
};
use async_trait::async_trait;
use botticelli_database::{DatabaseTableQueryRegistry, TableQueryExecutor, establish_connection};
use botticelli_models::GeminiClient;
use botticelli_narrative::{
    MultiNarrative, Narrative, NarrativeExecutor, NarrativeProvider, ProcessorRegistry,
};
use ractor::Actor;
use serde_json::json;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Skill for executing narrative workflows.
pub struct NarrativeExecutionSkill {
    name: String,
}

impl NarrativeExecutionSkill {
    /// Create a new narrative execution skill.
    pub fn new() -> Self {
        Self {
            name: "narrative_execution".to_string(),
        }
    }
}

impl Default for NarrativeExecutionSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for NarrativeExecutionSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Execute narrative workflows using botticelli_narrative"
    }

    #[tracing::instrument(skip(self, context), fields(skill = %self.name))]
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        tracing::debug!("Executing narrative execution skill");

        let narrative_path = context.config().get("narrative_path").ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Missing narrative_path configuration".to_string(),
            ))
        })?;

        let narrative_name = context.config().get("narrative_name");

        tracing::info!(
            narrative_path,
            narrative_name = ?narrative_name,
            "Loading narrative for execution"
        );

        // Load narrative from file
        let path = Path::new(narrative_path);

        // Determine which type of narrative to load
        // MultiNarrative preserves composition context for carousel mode
        let use_multi_narrative = narrative_name.is_some();

        let (narrative_for_single, multi_for_composition) = if use_multi_narrative {
            let name = narrative_name.as_ref().unwrap();

            // Load multi-narrative file - preserves composition context
            tracing::debug!(
                narrative_name = name,
                "Loading multi-narrative file with composition context"
            );

            let multi = MultiNarrative::from_file(path, name).map_err(|e| {
                ActorError::new(ActorErrorKind::FileIo {
                    path: path.to_path_buf(),
                    message: format!("Failed to load multi-narrative file: {}", e),
                })
            })?;

            // Verify target narrative exists
            let target = multi.get_narrative(name).ok_or_else(|| {
                ActorError::new(ActorErrorKind::InvalidConfiguration(format!(
                    "Narrative '{}' not found in file",
                    name
                )))
            })?;

            tracing::debug!(
                narrative_name = target.name(),
                act_count = target.acts().len(),
                "Multi-narrative loaded successfully with composition context"
            );

            (None, Some(multi))
        } else {
            // Load single narrative file
            tracing::debug!("Loading single narrative from file");
            let narrative = Narrative::from_file(path).map_err(|e| {
                ActorError::new(ActorErrorKind::FileIo {
                    path: path.to_path_buf(),
                    message: format!("Failed to load narrative: {}", e),
                })
            })?;

            tracing::debug!(
                narrative_name = narrative.name(),
                act_count = narrative.acts().len(),
                "Narrative loaded successfully"
            );

            (Some(narrative), None)
        };

        // Create Gemini client for narrative execution
        // TODO: Make this configurable to support other LLM providers
        // GeminiClient::new_with_config() reads GEMINI_API_KEY from environment
        // and loads tier config + budget multipliers from botticelli.toml
        let client = GeminiClient::new_with_config(None).map_err(|e| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(format!(
                "Failed to create Gemini client: {}",
                e
            )))
        })?;

        // Spawn storage actor for database operations
        tracing::debug!("Spawning storage actor");
        let storage_actor = botticelli_narrative::StorageActor::new(context.db_pool().clone());
        let (storage_ref, _handle) = Actor::spawn(None, storage_actor, context.db_pool().clone())
            .await
            .map_err(|e| {
                ActorError::new(ActorErrorKind::Narrative(format!(
                    "Failed to spawn storage actor: {}",
                    e
                )))
            })?;

        // Create processor registry with content generation processor
        tracing::debug!("Creating processor registry");
        let processor = botticelli_narrative::ContentGenerationProcessor::new(storage_ref.clone());
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(processor));

        // Create bot command registry for narrative bot commands
        #[cfg(feature = "discord")]
        let bot_registry = {
            use botticelli_social::{BotCommandRegistryImpl, DatabaseCommandExecutor};

            // Load .env file if present
            let _ = dotenvy::dotenv();

            tracing::debug!("Creating bot command registry");
            let mut bot_registry = BotCommandRegistryImpl::new();

            // Always register database executor
            let database_executor = DatabaseCommandExecutor::new();
            bot_registry.register(database_executor);
            tracing::debug!("Database command executor registered");

            // Register Discord executor if token is available
            if let Ok(token) = std::env::var("DISCORD_TOKEN") {
                use botticelli_social::DiscordCommandExecutor;
                tracing::debug!("Configuring Discord bot executor");
                let discord_executor = DiscordCommandExecutor::new(token);
                bot_registry.register(discord_executor);
                tracing::debug!("Discord bot executor registered");
            } else {
                tracing::debug!("DISCORD_TOKEN not set, Discord commands will not be available");
            }

            Some(Box::new(bot_registry) as Box<dyn botticelli_narrative::BotCommandRegistry>)
        };

        #[cfg(not(feature = "discord"))]
        let bot_registry: Option<Box<dyn botticelli_narrative::BotCommandRegistry>> = None;

        // Create table query registry for database table access
        tracing::debug!("Creating table query registry");

        // Establish a standalone connection for table queries
        // TODO: Refactor TableQueryExecutor to use connection pool
        let conn = establish_connection().map_err(|e| {
            ActorError::new(ActorErrorKind::DatabaseFailed(format!(
                "Failed to establish database connection for table queries: {}",
                e
            )))
        })?;

        let table_executor = TableQueryExecutor::new(Arc::new(Mutex::new(conn)));
        let table_registry = DatabaseTableQueryRegistry::new(table_executor);

        // Create executor with the client, processors, table registry, and bot registry
        let mut executor = NarrativeExecutor::with_processors(client, registry)
            .with_table_registry(Box::new(table_registry));
        tracing::debug!("Table query registry configured");

        if let Some(bot_reg) = bot_registry {
            executor = executor.with_bot_registry(bot_reg);
            tracing::debug!("Bot command registry configured");
        }

        // Execute narrative with appropriate type (MultiNarrative or single Narrative)
        // Both implement NarrativeProvider trait
        let (result, executed_narrative_name, executed_act_count) =
            if let Some(multi) = multi_for_composition {
                // Get target narrative name from original parameter
                let target_name = narrative_name
                    .as_ref()
                    .expect("multi_for_composition implies narrative_name is Some");

                tracing::info!(
                    narrative_name = target_name,
                    "Executing multi-narrative with composition context"
                );

                let result = executor.execute(&multi).await.map_err(|e| {
                    ActorError::new(ActorErrorKind::Narrative(format!(
                        "Multi-narrative execution failed: {}",
                        e
                    )))
                })?;

                let act_count = multi
                    .get_narrative(target_name)
                    .map(|n| n.acts().len())
                    .unwrap_or(0);

                (result, target_name.to_string(), act_count)
            } else if let Some(narrative) = narrative_for_single {
                tracing::info!(narrative_name = narrative.name(), "Executing narrative");

                let result = executor.execute(&narrative).await.map_err(|e| {
                    ActorError::new(ActorErrorKind::Narrative(format!(
                        "Narrative execution failed: {}",
                        e
                    )))
                })?;

                (result, narrative.name().to_string(), narrative.acts().len())
            } else {
                unreachable!("Either narrative_for_single or multi_for_composition must be Some");
            };

        // Shutdown the storage actor
        tracing::debug!("Shutting down storage actor");
        storage_ref.stop(None);

        tracing::debug!("Waiting for storage actor to stop");
        _handle.await.map_err(|e| {
            ActorError::new(ActorErrorKind::Narrative(format!(
                "Storage actor shutdown error: {}",
                e
            )))
        })?;

        tracing::info!(
            narrative_name = %executed_narrative_name,
            "Narrative execution completed successfully"
        );

        Ok(SkillOutputBuilder::default()
            .skill_name(self.name.clone())
            .data(json!({
                "status": "executed",
                "narrative_path": narrative_path,
                "narrative_name": executed_narrative_name,
                "act_count": executed_act_count,
                "result": result,
            }))
            .build()
            .map_err(|e| ActorError::new(ActorErrorKind::Narrative(e)))?)
    }
}
