//! Integration layer between sampling coordinator and chat commands.

use crate::{ChatLlmSampler, ServiceContainer};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_mcp::{PartialNarrative, SamplingCoordinator};
use std::sync::Arc;
use tracing::{debug, instrument};

/// Integrates LLM sampling with chat system for narrative generation.
pub struct SamplingIntegration {
    coordinator: Arc<SamplingCoordinator>,
    sampler: Arc<ChatLlmSampler>,
}

impl SamplingIntegration {
    /// Create new sampling integration.
    ///
    /// Integrates LLM provider from services with MCP tool registry
    /// for tool-enabled narrative generation.
    ///
    /// # Available with the `cli` feature
    #[cfg(feature = "cli")]
    #[instrument(skip(services))]
    pub async fn new(services: Arc<ServiceContainer>) -> ChatResult<Self> {
        // Create tool registry with default MCP tools
        let tool_registry = Arc::new(botticelli_mcp::ToolRegistry::default());

        // Get real provider from services with tool calling support
        let provider = services.llm_provider_with_tools().await?;

        let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry.clone()));

        let coordinator = Arc::new(SamplingCoordinator::new(sampler.clone(), tool_registry));

        debug!("Initialized sampling integration with real LLM provider");
        Ok(Self {
            coordinator,
            sampler,
        })
    }

    /// Start LLM-driven narrative generation from user description.
    #[instrument(skip(self))]
    pub async fn generate_narrative(&self, description: String) -> ChatResult<PartialNarrative> {
        debug!(description = %description, "Starting LLM-driven narrative generation");

        self.coordinator
            .generate_narrative(description)
            .await
            .map_err(|e| {
                ChatError::new(ChatErrorKind::SamplingError(format!(
                    "Failed to generate narrative: {}",
                    e
                )))
            })
    }

    /// Start interactive narrative creation with guided elicitation.
    #[instrument(skip(self))]
    pub async fn create_interactive(&self) -> ChatResult<PartialNarrative> {
        debug!("Starting interactive narrative creation");

        // TODO: This needs TUI dialog integration
        // For now, return placeholder
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "Interactive elicitation requires TUI dialog implementation".to_string(),
        )))
    }

    /// Continue refining an existing narrative via LLM.
    #[instrument(skip(self, narrative))]
    pub async fn refine_narrative(
        &self,
        narrative: PartialNarrative,
        user_feedback: String,
    ) -> ChatResult<PartialNarrative> {
        debug!(
            narrative_name = narrative.name().as_deref(),
            "Refining narrative with user feedback"
        );

        self.coordinator
            .refine_narrative(narrative, user_feedback)
            .await
            .map_err(|e| {
                ChatError::new(ChatErrorKind::SamplingError(format!(
                    "Failed to refine narrative: {}",
                    e
                )))
            })
    }

    /// Get reference to sampling coordinator.
    pub fn coordinator(&self) -> &Arc<SamplingCoordinator> {
        &self.coordinator
    }

    /// Get reference to LLM sampler.
    pub fn sampler(&self) -> &Arc<ChatLlmSampler> {
        &self.sampler
    }
}
