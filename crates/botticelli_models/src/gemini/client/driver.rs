//! BotticelliDriver trait implementation for GeminiClient.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::GeminiError;
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::TierConfig;

use super::core::GeminiClient;

#[async_trait]
impl BotticelliDriver for GeminiClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = GeminiError;
    type RateLimitConfig = TierConfig;
    type Capabilities = ();

    async fn generate(&self, req: &Self::Request) -> Result<Self::Response, Self::Error> {
        self.generate_internal(req).await
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    fn model_name(&self) -> &str {
        "gemini-2.0-flash-exp"
    }

    fn rate_limits(&self) -> &Self::RateLimitConfig {
        // TODO: Return actual rate limits from client
        unimplemented!("rate_limits not yet implemented")
    }

    fn capabilities(&self) -> Self::Capabilities {
        ()
    }
}
