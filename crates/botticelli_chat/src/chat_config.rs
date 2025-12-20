use botticelli_models::{ModelBounds, ModelId, SelectionStrategy};
use serde::{Deserialize, Serialize};

/// Configuration for chat session model selection and fallback behavior.
///
/// Defines boundaries for model selection during rate limit fallback,
/// controlling which models can be used during automatic selection.
#[derive(Debug, Clone, Serialize, Deserialize, derive_new::new)]
pub struct ChatConfig {
    /// Model selection boundaries (optional).
    ///
    /// When None, no bounds are enforced during fallback.
    /// When Some, restricts model selection to specified range.
    #[serde(default)]
    model_bounds: Option<ModelBounds>,

    /// Initial model to use for the session.
    ///
    /// Fallback starts from this model when rate limits occur.
    initial_model: ModelId,

    /// Strategy for selecting fallback models.
    ///
    /// Controls whether to prefer models from the same family (loyal)
    /// or explore other families (friendly) during fallback.
    #[serde(default)]
    fallback_strategy: SelectionStrategy,
}

impl ChatConfig {
    /// Check if a model is within configured bounds.
    #[tracing::instrument(skip(self))]
    pub fn is_within_bounds(&self, model: &ModelId) -> bool {
        match &self.model_bounds {
            Some(bounds) => bounds.allows(*model),
            None => true,
        }
    }

    /// Get the initial model for this configuration.
    pub fn initial_model(&self) -> &ModelId {
        &self.initial_model
    }

    /// Get the model bounds if configured.
    pub fn model_bounds(&self) -> Option<&ModelBounds> {
        self.model_bounds.as_ref()
    }

    /// Get the fallback strategy.
    pub fn fallback_strategy(&self) -> &SelectionStrategy {
        &self.fallback_strategy
    }
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model_bounds: None,
            initial_model: ModelId::Gemini(botticelli_models::GeminiModel::Gemini25Flash),
            fallback_strategy: SelectionStrategy::FriendlyFirst,
        }
    }
}
