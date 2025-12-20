//! LlmProvider implementation for Anthropic client.

use crate::AnthropicClient;
use async_trait::async_trait;
use botticelli_core::{
    GenerateRequest, GenerateResponse, LlmProvider, ProviderError, ProviderErrorKind,
};
use botticelli_interface::BotticelliDriver;
use tracing::{debug, error, instrument};

#[async_trait]
impl LlmProvider for AnthropicClient {
    #[instrument(skip(self, request))]
    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, ProviderError> {
        debug!("Generating response via LlmProvider trait");

        // Delegate to BotticelliDriver::generate which handles the new architecture
        BotticelliDriver::generate(self, request)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to generate response");
                ProviderError::new("anthropic", ProviderErrorKind::ApiError(e.to_string()))
            })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        self.model_name()
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
