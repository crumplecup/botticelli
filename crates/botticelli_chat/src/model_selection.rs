use crate::ChatError;
use botticelli_models::{ModelId, ModelSelector};

/// Chat session with automatic model fallback.
#[derive(Debug, Clone, derive_new::new)]
pub struct ChatSession {
    selector: ModelSelector,
    current_model: ModelId,
}

impl ChatSession {
    /// Get current model being used.
    pub fn current_model(&self) -> &ModelId {
        &self.current_model
    }

    /// Handle rate limit error by selecting next available model.
    ///
    /// Returns the new model to try, or error if no models available.
    #[tracing::instrument(skip(self, error_message), fields(current = ?self.current_model))]
    pub fn handle_rate_limit(&mut self, error_message: &str) -> Result<ModelId, ChatError> {
        tracing::warn!("Rate limit hit, attempting fallback");

        let next = self
            .selector
            .select_next(self.current_model, error_message)
            .ok_or_else(|| {
                ChatError::invalid_input("No available models within bounds")
            })?;

        tracing::info!(next = ?next, "Selected fallback model");
        self.current_model = next;
        Ok(next)
    }

    /// Get rate limit status for a model family.
    pub fn get_family_status(
        &self,
        family: botticelli_models::ModelFamily,
    ) -> Option<&botticelli_models::RateLimitStatus> {
        self.selector.get_status(family)
    }
}
