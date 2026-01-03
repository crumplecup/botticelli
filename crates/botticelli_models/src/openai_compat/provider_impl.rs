//! LlmProvider implementation for OpenAI-compatible clients.

use crate::openai_compat::OpenAICompatibleClient;
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::ProviderError;
use botticelli_interface::LlmProvider;
use tracing::{debug, error, instrument};

#[async_trait]
impl LlmProvider for OpenAICompatibleClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ProviderError;

    #[instrument(skip(self, request), fields(provider = self.provider_name()))]
    async fn generate(&self, request: &Self::Request) -> Result<Self::Response, Self::Error> {
        debug!("Generating response via LlmProvider trait");

        self.generate(request).await.map_err(|e| {
            error!(error = %e, "Failed to generate response");
            ProviderError::new(
                self.provider_name(),
                botticelli_error::ProviderErrorKind::ApiError(e.to_string()),
            )
        })
    }

    fn provider_name(&self) -> &str {
        self.provider_name()
    }

    fn default_model(&self) -> &str {
        self.model_name()
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
