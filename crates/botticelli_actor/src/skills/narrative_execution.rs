//! Narrative execution skill for running narrative workflows.

use crate::{
    ActorError, ActorErrorKind, Skill, SkillContext, SkillOutput, SkillOutputBuilder, SkillResult,
};
use async_trait::async_trait;
use botticelli_database::BotStorageTableQueryRegistry;
use botticelli_models::GeminiClient;
use botticelli_narrative::{NarrativeExecutor, ProcessorRegistry};
use ractor::Actor;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

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

        let path = Path::new(narrative_path);
        let narrative_source = botticelli_narrative::NarrativeSource::from_file(
            path,
            narrative_name.as_ref().map(|s| s.as_str()),
        )
        .map_err(|e| {
            ActorError::new(ActorErrorKind::FileIo {
                path: path.to_path_buf(),
                message: format!("Failed to load narrative: {}", e),
            })
        })?;

        tracing::debug!(
            narrative_name = narrative_source.name(),
            has_composition_context = narrative_source.has_composition_context(),
            "Narrative loaded successfully"
        );

        let client = GeminiClient::new_with_config(None).map_err(|e| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(format!(
                "Failed to create Gemini client: {}",
                e
            )))
        })?;

        tracing::debug!("Spawning storage actor");
        let storage_actor = botticelli_narrative::StorageActor::new(Arc::clone(context.storage()));
        let (storage_ref, _handle) =
            Actor::spawn(None, storage_actor, Arc::clone(context.storage()))
                .await
                .map_err(|e| {
                    ActorError::new(ActorErrorKind::Narrative(format!(
                        "Failed to spawn storage actor: {}",
                        e
                    )))
                })?;

        tracing::debug!("Creating processor registry");
        let processor = botticelli_narrative::ContentGenerationProcessor::new(storage_ref.clone());
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(processor));

        #[cfg(feature = "discord")]
        let bot_registry = {
            use botticelli_social::{BotCommandRegistryImpl, DatabaseCommandExecutor};

            let _ = dotenvy::dotenv();

            tracing::debug!("Creating bot command registry");
            let mut bot_registry = BotCommandRegistryImpl::new();

            let database_executor = DatabaseCommandExecutor::new(Arc::clone(context.storage()));
            bot_registry.register(database_executor);
            tracing::debug!("Database command executor registered");

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

        tracing::debug!("Creating table query registry");
        let table_registry = BotStorageTableQueryRegistry::new(Arc::clone(context.storage()));

        let mut executor = NarrativeExecutor::with_processors(client, registry)
            .with_table_registry(Box::new(table_registry));
        tracing::debug!("Table query registry configured");

        if let Some(bot_reg) = bot_registry {
            executor = executor.with_bot_registry(bot_reg);
            tracing::debug!("Bot command registry configured");
        }

        tracing::info!(
            narrative_name = narrative_source.name(),
            has_composition = narrative_source.has_composition_context(),
            "Executing narrative"
        );

        let result = executor
            .execute_from_source(&narrative_source)
            .await
            .map_err(|e| {
                ActorError::new(ActorErrorKind::Narrative(format!(
                    "Narrative execution failed: {}",
                    e
                )))
            })?;

        let executed_narrative_name = narrative_source.name().to_string();
        let executed_act_count = narrative_source
            .get_narrative()
            .map(|n| n.acts().len())
            .unwrap_or(0);

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
