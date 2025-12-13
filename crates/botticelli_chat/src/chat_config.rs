use botticelli_models::{ModelBounds, ModelId};
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
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model_bounds: None,
            initial_model: ModelId::Gemini(botticelli_models::GeminiModel::Gemini25Flash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use botticelli_models::{GeminiModel, GroqModel};

    #[test]
    fn test_default_config() {
        let config = ChatConfig::default();
        assert!(config.model_bounds().is_none());
        assert_eq!(
            config.initial_model(),
            &ModelId::Gemini(GeminiModel::Gemini25Flash)
        );
    }

    #[test]
    fn test_config_with_bounds() {
        let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));
        let config = ChatConfig::new(Some(bounds), ModelId::Gemini(GeminiModel::Gemini25Flash));

        assert!(config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25Flash)));
        assert!(config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25Pro)));
        assert!(!config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25FlashLite)));
    }

    #[test]
    fn test_config_without_bounds() {
        let config = ChatConfig::new(None, ModelId::Groq(GroqModel::Llama33_70BVersatile));

        assert!(config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25Pro)));
        assert!(config.is_within_bounds(&ModelId::Groq(GroqModel::Llama31_8BInstant)));
    }

    #[test]
    fn test_serialize_deserialize() {
        let bounds = ModelBounds::both(
            ModelId::Gemini(GeminiModel::Gemini25FlashLite),
            ModelId::Gemini(GeminiModel::Gemini25Pro),
        );
        let config = ChatConfig::new(Some(bounds), ModelId::Gemini(GeminiModel::Gemini25Flash));

        let toml = toml::to_string(&config).expect("Serialize failed");
        let deserialized: ChatConfig = toml::from_str(&toml).expect("Deserialize failed");

        assert_eq!(
            config.initial_model(),
            deserialized.initial_model()
        );
        assert!(deserialized.model_bounds().is_some());
    }
}
