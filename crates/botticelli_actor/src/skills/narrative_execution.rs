//! Narrative execution skill for running narrative workflows.

use crate::{ActorError, ActorErrorKind, Skill, SkillContext, SkillOutput, SkillResult};
use async_trait::async_trait;
use botticelli_models::GeminiClient;
use botticelli_narrative::{
    MultiNarrative, Narrative, NarrativeExecutor, NarrativeProvider, ProcessorRegistry,
};
use ractor::Actor;
use serde_json::json;
use std::path::Path;

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

        let narrative_path = context.config.get("narrative_path").ok_or_else(|| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(
                "Missing narrative_path configuration".to_string(),
            ))
        })?;

        let narrative_name = context.config.get("narrative_name");

        tracing::info!(
            narrative_path,
            narrative_name = ?narrative_name,
            "Loading narrative for execution"
        );

        // Load narrative from file
        let path = Path::new(narrative_path);

        let narrative = if let Some(name) = narrative_name.as_ref() {
            // Load specific narrative from multi-narrative file
            tracing::debug!(
                narrative_name = name,
                "Loading specific narrative from file"
            );
            let multi = MultiNarrative::from_file(path, name).map_err(|e| {
                ActorError::new(ActorErrorKind::FileIo {
                    path: path.to_path_buf(),
                    message: format!("Failed to load multi-narrative file: {}", e),
                })
            })?;

            multi
                .get_narrative(name)
                .ok_or_else(|| {
                    ActorError::new(ActorErrorKind::InvalidConfiguration(format!(
                        "Narrative '{}' not found in file",
                        name
                    )))
                })?
                .clone()
        } else {
            // Load single narrative file
            tracing::debug!("Loading single narrative from file");
            Narrative::from_file(path).map_err(|e| {
                ActorError::new(ActorErrorKind::FileIo {
                    path: path.to_path_buf(),
                    message: format!("Failed to load narrative: {}", e),
                })
            })?
        };

        tracing::debug!(
            narrative_name = narrative.name(),
            act_count = narrative.acts().len(),
            "Narrative loaded successfully"
        );

        // Create Gemini client for narrative execution
        // TODO: Make this configurable to support other LLM providers
        // GeminiClient::new() reads GEMINI_API_KEY from environment
        let client = GeminiClient::new().map_err(|e| {
            ActorError::new(ActorErrorKind::InvalidConfiguration(format!(
                "Failed to create Gemini client: {}",
                e
            )))
        })?;

        // Spawn storage actor for database operations
        tracing::debug!("Spawning storage actor");
        let storage_actor = botticelli_narrative::StorageActor::new(context.db_pool.clone());
        let (storage_ref, _handle) = Actor::spawn(None, storage_actor, context.db_pool.clone())
            .await
            .map_err(|e| {
                ActorError::new(ActorErrorKind::Narrative(format!(
                    "Failed to spawn storage actor: {}",
                    e
                )))
            })?;

        // Create processor registry with content generation processor
        tracing::debug!("Creating processor registry");
        let processor =
            botticelli_narrative::ContentGenerationProcessor::new(storage_ref.clone());
        let mut registry = ProcessorRegistry::new();
        registry.register(Box::new(processor));

        // Create executor with the client and processors
        let executor = NarrativeExecutor::with_processors(client, registry);

        tracing::info!(narrative_name = narrative.name(), "Executing narrative");

        let result = executor.execute(&narrative).await.map_err(|e| {
            ActorError::new(ActorErrorKind::Narrative(format!(
                "Narrative execution failed: {}",
                e
            )))
        })?;

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
            narrative_name = narrative.name(),
            "Narrative execution completed successfully"
        );

        Ok(SkillOutput {
            skill_name: self.name.clone(),
            data: json!({
                "status": "executed",
                "narrative_path": narrative_path,
                "narrative_name": narrative.name(),
                "act_count": narrative.acts().len(),
                "result": result,
            }),
        })
    }
}
