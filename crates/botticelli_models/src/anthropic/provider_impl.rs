//! LlmProvider implementation for Anthropic client.

use crate::AnthropicClient;
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::ProviderError;
use botticelli_interface::{BotticelliDriver, LlmProvider};
use tracing::{debug, error, instrument};

#[async_trait]
impl LlmProvider for AnthropicClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ProviderError;

    #[instrument(skip(self, request))]
    async fn generate(&self, request: &Self::Request) -> Result<Self::Response, Self::Error> {
        debug!("Generating response via LlmProvider trait");

        // Delegate to BotticelliDriver::generate which handles the new architecture
        BotticelliDriver::generate(self, request)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to generate response");
                e.into()
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
