//! Integration layer between sampling coordinator and chat commands.

use crate::{ChatLlmSampler, ChatSession, ServiceContainer};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_mcp::{PartialNarrative, SamplingCoordinator};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

/// Integrates LLM sampling with chat system for narrative generation.
pub struct SamplingIntegration {
    coordinator: Arc<SamplingCoordinator>,
    sampler: Arc<ChatLlmSampler>,
    chat_session: Arc<RwLock<ChatSession>>,
}

impl SamplingIntegration {
    /// Create new sampling integration.
    ///
    /// Integrates LLM provider from services with MCP tool registry
    /// for tool-enabled narrative generation with automatic fallback.
    ///
    /// # Available with the `cli` feature
    #[cfg(feature = "cli")]
    #[instrument(skip(services))]
    pub async fn new(services: Arc<ServiceContainer>) -> ChatResult<Self> {
        use botticelli_models::{ModelBounds, ModelSelector, RateLimitDetector};

        // Create tool registry with default MCP tools
        let tool_registry = Arc::new(botticelli_mcp::ToolRegistry::default());

        // Get configuration for fallback setup
        let initial_model = *services.config().chat.initial_model();
        let strategy = *services.config().chat.fallback_strategy();
        let bounds = services
            .config()
            .chat
            .model_bounds()
            .cloned()
            .unwrap_or_else(ModelBounds::none);

        debug!(
            model = ?initial_model,
            strategy = ?strategy,
            "Initializing sampling integration with fallback support"
        );

        // Create ChatSession for model selection and fallback
        let selector = ModelSelector::new(bounds, strategy, RateLimitDetector::new());
        let chat_session = Arc::new(RwLock::new(ChatSession::new(selector, initial_model)));

        // Get initial provider from services with tool calling support
        let provider = services.llm_provider_with_tools().await?;

        let sampler = Arc::new(ChatLlmSampler::new(
            provider,
            tool_registry.clone(),
            chat_session.clone(),
            services.clone(),
        ));

        let coordinator = Arc::new(SamplingCoordinator::new(sampler.clone(), tool_registry));

        debug!("Initialized sampling integration with fallback architecture");
        Ok(Self {
            coordinator,
            sampler,
            chat_session,
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

    /// Access to chat session for conversation tracking.
    pub fn chat_session(&self) -> &Arc<RwLock<ChatSession>> {
        &self.chat_session
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
