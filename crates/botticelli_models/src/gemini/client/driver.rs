//! BotticelliDriver trait implementation for GeminiClient.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::GeminiError;
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::TierConfig;

use crate::gemini::ModelCapabilities;
use super::core::GeminiClient;

#[async_trait]
impl BotticelliDriver for GeminiClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = GeminiError;
    type RateLimitConfig = TierConfig;
    type Capabilities = ModelCapabilities;

    async fn generate(&self, req: &Self::Request) -> Result<Self::Response, Self::Error> {
        self.generate_internal(req).await
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    fn model_name(&self) -> &str {
        self.model_name()
    }

    fn rate_limits(&self) -> &Self::RateLimitConfig {
        self.base_tier()
    }

    fn capabilities(&self) -> Self::Capabilities {
        self.capabilities()
    }
}
