//! Test doubles for models testing.
//!
//! Provides configurable test implementations of model clients for unit testing.

use async_trait::async_trait;
use botticelli_core::{Capabilities, GenerateRequest, GenerateResponse, Message, Output, Role, StopReason};
use botticelli_error::{BotticelliResult, ModelsError, ModelsErrorKind};
use botticelli_interface::{BotticelliDriver, Metadata, Streaming, TokenCounting};
use futures::stream::{self, BoxStream};
use std::sync::{Arc, Mutex};

/// Test metadata implementation.
#[derive(Debug, Clone)]
pub struct TestMetadata {
    model_name: String,
    provider_name: String,
}

impl TestMetadata {
    /// Creates new test metadata.
    pub fn new() -> Self {
        Self {
            model_name: "test-model".to_string(),
            provider_name: "test".to_string(),
        }
    }
}

impl Default for TestMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Test driver that returns configurable responses.
///
/// Useful for testing code that depends on model clients without making API calls.
#[derive(Debug, Clone)]
pub struct TestDriver {
    /// Responses to return on successive generate() calls
    responses: Arc<Mutex<Vec<GenerateResponse>>>,
    /// Errors to return on successive generate() calls
    errors: Arc<Mutex<Vec<ModelsError>>>,
    /// Call count for tracking invocations
    call_count: Arc<Mutex<usize>>,
    /// Metadata
    metadata: TestMetadata,
}

impl TestDriver {
    /// Creates a new test driver with configured responses.
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            errors: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(Mutex::new(0)),
            metadata: TestMetadata::new(),
        }
    }

    /// Adds a successful response to the queue.
    pub fn with_response(self, response: GenerateResponse) -> Self {
        self.responses.lock().unwrap().push(response);
        self
    }

    /// Adds an error response to the queue.
    pub fn with_error(self, error: ModelsError) -> Self {
        self.errors.lock().unwrap().push(error);
        self
    }

    /// Adds a simple text response to the queue.
    pub fn with_text(self, text: impl Into<String>) -> Self {
        let response = GenerateResponse::builder()
            .outputs(vec![Output::Text(text.into())])
            .stop_reason(StopReason::EndTurn)
            .build()
            .unwrap();
        self.with_response(response)
    }

    /// Returns the number of times generate() was called.
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

impl Default for TestDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BotticelliDriver for TestDriver {
    type Error = ModelsError;

    async fn generate(&self, _req: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        // Check for queued errors first
        let mut errors = self.errors.lock().unwrap();
        if !errors.is_empty() {
            return Err(errors.remove(0));
        }

        // Return queued response or default
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            Ok(GenerateResponse::builder()
                .outputs(vec![Output::Text("test response".to_string())])
                .stop_reason(StopReason::EndTurn)
                .build()
                .unwrap())
        } else {
            Ok(responses.remove(0))
        }
    }

    fn provider_name(&self) -> &str {
        "test"
    }

    fn model_name(&self) -> &str {
        "test-model"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(
            true,  // streaming
            true,  // tool_calling
            true,  // vision
            false, // audio
            false, // video
            false, // embeddings
            false, // json_mode
            false, // batch_generation
        )
    }
}

impl Metadata for TestDriver {
    type ModelMetadata = TestMetadata;

    fn metadata(&self) -> &Self::ModelMetadata {
        &self.metadata
    }
}

#[async_trait]
impl Streaming for TestDriver {
    type Error = ModelsError;

    async fn generate_stream(
        &self,
        _req: &GenerateRequest,
    ) -> Result<BoxStream<'static, Result<GenerateResponse, Self::Error>>, Self::Error> {
        let response = GenerateResponse::builder()
            .outputs(vec![Output::Text("streaming test".to_string())])
            .stop_reason(StopReason::EndTurn)
            .build()
            .unwrap();

        Ok(Box::pin(stream::once(async move { Ok(response) })))
    }
}

#[async_trait]
impl TokenCounting for TestDriver {
    type Error = ModelsError;

    async fn count_tokens(&self, messages: &[Message]) -> Result<usize, Self::Error> {
        // Simple approximation: 4 chars per token
        let total_chars: usize = messages
            .iter()
            .map(|m| m.content().iter().map(|i| format!("{:?}", i).len()).sum::<usize>())
            .sum();
        Ok(total_chars / 4)
    }
}
