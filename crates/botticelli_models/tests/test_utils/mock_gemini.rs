//! Mock Gemini client for testing using mockall.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Output, StreamChunk};
use botticelli_error::{GeminiError, GeminiErrorKind, ModelsError};
use botticelli_interface::{BotticelliDriver, Streaming, Vision};
use botticelli_rate_limit::RateLimitConfig;
use mockall::mock;
use std::pin::Pin;
use futures_util::stream::Stream;

// Define the mock using mockall
mock! {
    pub GeminiClient {}

    #[async_trait]
    impl BotticelliDriver for GeminiClient {
        type Request = GenerateRequest;
        type Response = GenerateResponse;
        type Error = ModelsError;
        type RateLimitConfig = RateLimitConfig;
        type Capabilities = botticelli_models::ModelCapabilities;

        async fn generate(&self, req: &GenerateRequest) -> Result<GenerateResponse, ModelsError>;
        fn provider_name(&self) -> &str;
        fn model_name(&self) -> &str;
        fn rate_limits(&self) -> &RateLimitConfig;
        fn capabilities(&self) -> Self::Capabilities;
    }

    #[async_trait]
    impl Streaming for GeminiClient {
        type StreamChunk = StreamChunk;
        type Error = ModelsError;
        
        async fn generate_stream(
            &self,
            req: &GenerateRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ModelsError>> + Send>>, ModelsError>;
    }

    impl Vision for GeminiClient {
        fn max_images_per_request(&self) -> usize;
        fn supported_image_formats(&self) -> &[&'static str];
        fn max_image_size_bytes(&self) -> usize;
    }
}

/// Helper to create a successful mock response
pub fn create_success_response(text: impl Into<String>) -> GenerateResponse {
    GenerateResponse::builder()
        .outputs(vec![Output::Text(text.into())])
        .stop_reason(botticelli_core::StopReason::EndTurn)
        .build()
        .expect("Valid response")
}

/// Helper to create an error
pub fn create_error(kind: GeminiErrorKind) -> ModelsError {
    ModelsError::from(GeminiError::new(kind))
}
