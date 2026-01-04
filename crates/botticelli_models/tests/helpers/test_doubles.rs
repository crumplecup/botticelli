/// Test doubles for models testing
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Output, StopReason};
use botticelli_error::ModelsResult;
use botticelli_interface::{BotticelliDriver, Metadata, Streaming, TokenCounting, ToolCalling};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

/// Test double for BotticelliDriver trait
#[derive(Debug, Clone)]
pub struct TestDriver {
    response: GenerateResponse,
}

impl TestDriver {
    /// Creates a new test driver with a success response
    pub fn new_success(text: impl Into<String>) -> ModelsResult<Self> {
        let response = GenerateResponse::builder()
            .outputs(vec![Output::Text(text.into())])
            .stop_reason(StopReason::EndTurn)
            .build()?;
        
        Ok(Self { response })
    }
}

#[async_trait]
impl BotticelliDriver for TestDriver {
    type Error = botticelli_error::ModelsError;

    async fn generate(&self, _req: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        Ok(self.response.clone())
    }

    fn provider_name(&self) -> &str {
        "test"
    }

    fn model_name(&self) -> &str {
        "test-model"
    }

    fn capabilities(&self) -> botticelli_interface::ModelCapabilities {
        botticelli_interface::ModelCapabilities::text_only()
    }
}

#[async_trait]
impl Streaming for TestDriver {
    type Error = botticelli_error::ModelsError;

    async fn generate_stream(
        &self,
        _req: &GenerateRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerateResponse, Self::Error>> + Send>>, Self::Error>
    {
        let response = self.response.clone();
        let stream = futures::stream::once(async move { Ok(response) });
        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl TokenCounting for TestDriver {
    type Error = botticelli_error::ModelsError;

    async fn count_tokens(&self, _req: &GenerateRequest) -> Result<usize, Self::Error> {
        Ok(100)
    }
}

#[async_trait]
impl ToolCalling for TestDriver {
    type Error = botticelli_error::ModelsError;

    async fn generate_with_tools(
        &self,
        req: &GenerateRequest,
    ) -> Result<GenerateResponse, Self::Error> {
        self.generate(req).await
    }
}

impl Metadata for TestDriver {
    fn metadata(&self) -> botticelli_core::LlmMetrics {
        botticelli_core::LlmMetrics::builder()
            .provider(self.provider_name())
            .model(self.model_name())
            .build()
    }
}
