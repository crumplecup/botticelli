//! Mock Gemini client for testing.
//!
//! Provides a simple mock client for unit testing without API calls.

use botticelli_core::{GenerateRequest, GenerateResponse, Output, StopReason};
use botticelli_error::{GeminiError, GeminiErrorKind, ModelsError, ModelsErrorKind};
use botticelli_interface::BotticelliDriver;
use botticelli_models::ModelCapabilities;
use botticelli_rate_limit::RateLimitConfig;

/// Simple mock Gemini client for testing
pub struct MockGeminiClient {
    response_text: String,
}

impl MockGeminiClient {
    /// Create a new mock that returns the given text
    pub fn new_success(text: impl Into<String>) -> Self {
        Self {
            response_text: text.into(),
        }
    }
}

#[async_trait::async_trait]
impl BotticelliDriver for MockGeminiClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ModelsError;
    type RateLimitConfig = RateLimitConfig;
    type Capabilities = ModelCapabilities;

    async fn generate(&self, _req: &Self::Request) -> Result<Self::Response, Self::Error> {
        GenerateResponse::builder()
            .outputs(vec![Output::Text(self.response_text.clone())])
            .stop_reason(StopReason::EndTurn)
            .build()
            .map_err(|e| ModelsErrorKind::Builder(e.to_string()).into())
    }

    fn provider_name(&self) -> &str {
        "mock-gemini"
    }

    fn model_name(&self) -> &str {
        "mock-gemini"
    }

    fn rate_limits(&self) -> &Self::RateLimitConfig {
        // Return a static rate limit config for mocking
        static RATE_LIMITS: RateLimitConfig = RateLimitConfig::new(60, 100000, 1000, 1000000);
        &RATE_LIMITS
    }

    fn capabilities(&self) -> Self::Capabilities {
        ModelCapabilities::standard()
    }
}

/// Helper to create an error
pub fn create_error(kind: GeminiErrorKind) -> ModelsError {
    ModelsError::from(GeminiError::new(kind))
}
