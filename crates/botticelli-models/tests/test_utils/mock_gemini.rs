//! Mock Gemini client for testing.

use async_trait::async_trait;
use botticelli::{
    BotticelliDriver, BotticelliError, BotticelliResult, GenerateRequest, GenerateResponse,
    GeminiError, GeminiErrorKind, Metadata, ModelMetadata, Output, Streaming, StreamChunk,
    Vision,
};
use std::sync::{Arc, Mutex};

/// Behavior configuration for mock responses.
#[derive(Debug, Clone)]
pub enum MockBehavior {
    /// Always return success with the given text
    Success(String),
    /// Always return the specified error
    Error(GeminiErrorKind),
    /// Fail N times with the error, then succeed with the text
    FailThenSucceed {
        fail_count: usize,
        error: GeminiErrorKind,
        success_text: String,
    },
    /// Return a sequence of responses (errors or success)
    Sequence(Vec<MockResponse>),
}

/// A single mock response (success or error).
#[derive(Debug, Clone)]
pub enum MockResponse {
    Success(String),
    Error(GeminiErrorKind),
}

/// Mock Gemini client for testing.
///
/// This mock allows tests to control responses and verify behavior without
/// making actual API calls.
pub struct MockGeminiClient {
    behavior: MockBehavior,
    call_count: Arc<Mutex<usize>>,
    model_name: String,
}

impl MockGeminiClient {
    /// Create a mock client that always succeeds with the given text.
    pub fn new_success(text: impl Into<String>) -> Self {
        Self {
            behavior: MockBehavior::Success(text.into()),
            call_count: Arc::new(Mutex::new(0)),
            model_name: "mock-gemini".to_string(),
        }
    }

    /// Create a mock client that always fails with the given error.
    pub fn new_error(error: GeminiErrorKind) -> Self {
        Self {
            behavior: MockBehavior::Error(error),
            call_count: Arc::new(Mutex::new(0)),
            model_name: "mock-gemini".to_string(),
        }
    }

    /// Create a mock client that fails N times, then succeeds.
    ///
    /// Useful for testing retry behavior.
    pub fn new_fail_then_succeed(
        fail_count: usize,
        error: GeminiErrorKind,
        success_text: impl Into<String>,
    ) -> Self {
        Self {
            behavior: MockBehavior::FailThenSucceed {
                fail_count,
                error,
                success_text: success_text.into(),
            },
            call_count: Arc::new(Mutex::new(0)),
            model_name: "mock-gemini".to_string(),
        }
    }

    /// Create a mock client with a sequence of responses.
    pub fn new_sequence(responses: Vec<MockResponse>) -> Self {
        Self {
            behavior: MockBehavior::Sequence(responses),
            call_count: Arc::new(Mutex::new(0)),
            model_name: "mock-gemini".to_string(),
        }
    }

    /// Create a mock client with custom behavior.
    #[allow(dead_code)]
    pub fn new_with_behavior(behavior: MockBehavior) -> Self {
        Self {
            behavior,
            call_count: Arc::new(Mutex::new(0)),
            model_name: "mock-gemini".to_string(),
        }
    }

    /// Get the number of times generate() was called.
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    /// Reset the call count to zero.
    #[allow(dead_code)]
    pub fn reset_call_count(&self) {
        *self.call_count.lock().unwrap() = 0;
    }

    /// Get the next response based on the configured behavior.
    fn next_response(&self) -> BotticelliResult<GenerateResponse> {
        let mut count = self.call_count.lock().unwrap();
        let current_count = *count;
        *count += 1;

        match &self.behavior {
            MockBehavior::Success(text) => Ok(GenerateResponse {
                outputs: vec![Output::Text(text.clone())],
            }),
            MockBehavior::Error(error_kind) => {
                Err(BotticelliError::from(GeminiError::new(error_kind.clone())))
            }
            MockBehavior::FailThenSucceed {
                fail_count,
                error,
                success_text,
            } => {
                if current_count < *fail_count {
                    Err(BotticelliError::from(GeminiError::new(error.clone())))
                } else {
                    Ok(GenerateResponse {
                        outputs: vec![Output::Text(success_text.clone())],
                    })
                }
            }
            MockBehavior::Sequence(responses) => {
                if current_count >= responses.len() {
                    // Past end of sequence, return error
                    Err(BotticelliError::from(GeminiError::new(
                        GeminiErrorKind::ApiRequest(format!(
                            "Mock sequence exhausted (call {} beyond {} responses)",
                            current_count + 1,
                            responses.len()
                        )),
                    )))
                } else {
                    match &responses[current_count] {
                        MockResponse::Success(text) => Ok(GenerateResponse {
                            outputs: vec![Output::Text(text.clone())],
                        }),
                        MockResponse::Error(error_kind) => {
                            Err(BotticelliError::from(GeminiError::new(error_kind.clone())))
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl BotticelliDriver for MockGeminiClient {
    async fn generate(&self, _req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        // Small delay to simulate network latency (but keep it minimal for fast tests)
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        self.next_response()
    }

    fn provider_name(&self) -> &'static str {
        "mock-gemini"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl Metadata for MockGeminiClient {
    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            provider: "mock-gemini",
            model: self.model_name.clone(),
            max_input_tokens: 1_048_576,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_vision: true,
            supports_audio: true,
            supports_video: true,
            supports_documents: true,
            supports_tool_use: true,
            supports_json_mode: true,
            supports_embeddings: false,
            supports_batch: false,
        }
    }
}

impl Vision for MockGeminiClient {
    fn max_images_per_request(&self) -> usize {
        16
    }

    fn supported_image_formats(&self) -> &[&'static str] {
        &["image/png", "image/jpeg", "image/webp"]
    }

    fn max_image_size_bytes(&self) -> usize {
        20 * 1024 * 1024 // 20MB
    }
}

#[async_trait]
impl Streaming for MockGeminiClient {
    async fn generate_stream(
        &self,
        _req: &GenerateRequest,
    ) -> BotticelliResult<
        std::pin::Pin<Box<dyn futures_util::stream::Stream<Item = BotticelliResult<StreamChunk>> + Send>>,
    > {
        use futures_util::stream;

        // Get the response (reuse generate logic)
        let response = self.next_response()?;

        // Convert to a single-chunk stream
        let chunks = response.outputs.into_iter().map(|output| {
            Ok(StreamChunk {
                content: output,
                is_final: true,
                finish_reason: Some(botticelli::FinishReason::Stop),
            })
        });

        Ok(Box::pin(stream::iter(chunks)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_success() {
        let mock = MockGeminiClient::new_success("test response");
        let request = GenerateRequest::default();

        let response = mock.generate(&request).await.unwrap();
        assert_eq!(mock.call_count(), 1);

        match &response.outputs[0] {
            Output::Text(text) => assert_eq!(text, "test response"),
            _ => panic!("Expected text output"),
        }
    }

    #[tokio::test]
    async fn test_mock_error() {
        let mock = MockGeminiClient::new_error(GeminiErrorKind::HttpError {
            status_code: 503,
            message: "Service unavailable".to_string(),
        });
        let request = GenerateRequest::default();

        let result = mock.generate(&request).await;
        assert!(result.is_err());
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_fail_then_succeed() {
        let mock = MockGeminiClient::new_fail_then_succeed(
            2,
            GeminiErrorKind::HttpError {
                status_code: 503,
                message: "Overloaded".to_string(),
            },
            "success",
        );
        let request = GenerateRequest::default();

        // First two calls should fail
        assert!(mock.generate(&request).await.is_err());
        assert_eq!(mock.call_count(), 1);

        assert!(mock.generate(&request).await.is_err());
        assert_eq!(mock.call_count(), 2);

        // Third call should succeed
        let response = mock.generate(&request).await.unwrap();
        assert_eq!(mock.call_count(), 3);

        match &response.outputs[0] {
            Output::Text(text) => assert_eq!(text, "success"),
            _ => panic!("Expected text output"),
        }
    }

    #[tokio::test]
    async fn test_mock_sequence() {
        let mock = MockGeminiClient::new_sequence(vec![
            MockResponse::Success("first".to_string()),
            MockResponse::Error(GeminiErrorKind::HttpError {
                status_code: 429,
                message: "Rate limit".to_string(),
            }),
            MockResponse::Success("third".to_string()),
        ]);
        let request = GenerateRequest::default();

        // First call succeeds
        let response = mock.generate(&request).await.unwrap();
        match &response.outputs[0] {
            Output::Text(text) => assert_eq!(text, "first"),
            _ => panic!("Expected text output"),
        }

        // Second call fails
        assert!(mock.generate(&request).await.is_err());

        // Third call succeeds
        let response = mock.generate(&request).await.unwrap();
        match &response.outputs[0] {
            Output::Text(text) => assert_eq!(text, "third"),
            _ => panic!("Expected text output"),
        }

        assert_eq!(mock.call_count(), 3);
    }
}
