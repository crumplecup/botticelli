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

        let anthropic_req = self.convert_request(request).map_err(|e| {
            error!(error = %e, "Failed to convert request");
            ProviderError::new(
                "anthropic",
                ProviderErrorKind::InvalidRequest(e.to_string()),
            )
        })?;

        let anthropic_resp = self.generate_anthropic(&anthropic_req).await.map_err(|e| {
            error!(error = %e, "Failed to call Anthropic API");
            ProviderError::new("anthropic", ProviderErrorKind::ApiError(e.to_string()))
        })?;

        Self::convert_response(&anthropic_resp).map_err(|e| {
            error!(error = %e, "Failed to convert response");
            ProviderError::new("anthropic", ProviderErrorKind::ParsingError(e.to_string()))
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
