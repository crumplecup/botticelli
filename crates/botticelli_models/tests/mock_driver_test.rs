//! Mock-based tests for driver implementations.
//!
//! Demonstrates proper manual mocking patterns for testing without hitting real APIs.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Message, Output, Role, StopReason, Input};
use botticelli_interface::BotticelliDriver;
use botticelli_error::{ModelsError, ModelsErrorKind, ModelsResult};
use botticelli_rate_limit::{TierConfig, TierConfigBuilder};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Capabilities for mock driver.
#[derive(Debug, Clone)]
pub struct MockCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
}

/// Manual mock driver for testing.
///
/// This demonstrates the pattern for creating testable mocks without
/// hitting real APIs.
pub struct MockDriver {
    provider: String,
    model: String,
    rate_limits: TierConfig,
    response: Arc<Mutex<Option<GenerateResponse>>>,
    error: Arc<Mutex<Option<ModelsError>>>,
    call_count: Arc<Mutex<usize>>,
}

impl MockDriver {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            rate_limits: TierConfigBuilder::default()
                .name("test".to_string())
                .rpm(60u32)
                .tpm(90000u64)
                .rpd(1000u32)
                .build()
                .expect("Valid tier config"),
            response: Arc::new(Mutex::new(None)),
            error: Arc::new(Mutex::new(None)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Set the response this mock will return.
    pub async fn set_response(&self, response: GenerateResponse) {
        *self.response.lock().await = Some(response);
    }
    
    /// Set the error this mock will return.
    pub async fn set_error(&self, error: ModelsError) {
        *self.error.lock().await = Some(error);
    }
    
    /// Get the number of times generate was called.
    pub async fn call_count(&self) -> usize {
        *self.call_count.lock().await
    }
}

#[async_trait]
impl BotticelliDriver for MockDriver {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ModelsError;
    type RateLimitConfig = TierConfig;
    type Capabilities = MockCapabilities;
    
    async fn generate(&self, _req: &Self::Request) -> Result<Self::Response, Self::Error> {
        *self.call_count.lock().await += 1;
        
        if let Some(err) = self.error.lock().await.take() {
            return Err(err);
        }
        
        if let Some(response) = self.response.lock().await.clone() {
            return Ok(response);
        }
        
        Err(ModelsError::from(ModelsErrorKind::Builder(
            "No response or error configured in mock".to_string()
        )))
    }
    
    fn provider_name(&self) -> &str {
        &self.provider
    }
    
    fn model_name(&self) -> &str {
        &self.model
    }
    
    fn rate_limits(&self) -> &Self::RateLimitConfig {
        &self.rate_limits
    }
    
    fn capabilities(&self) -> Self::Capabilities {
        MockCapabilities {
            supports_streaming: false,
            supports_tools: false,
        }
    }
}

/// Test that a mock driver can be configured and used.
#[tokio::test]
async fn test_mock_driver_basic() -> ModelsResult<()> {
    let mock = MockDriver::new("mock_provider", "mock_model");
    
    // Configure response
    let response = GenerateResponse::builder()
        .outputs(vec![Output::Text("Mocked response".to_string())])
        .stop_reason(StopReason::EndTurn)
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    mock.set_response(response).await;
    
    // Use the mock
    assert_eq!(mock.provider_name(), "mock_provider");
    assert_eq!(mock.model_name(), "mock_model");
    
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("test".to_string())])
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    let response = mock.generate(&request).await?;
    
    assert_eq!(response.outputs().len(), 1);
    match &response.outputs()[0] {
        Output::Text(text) => assert_eq!(text, "Mocked response"),
        _ => panic!("Expected text output"),
    }
    
    assert_eq!(mock.call_count().await, 1);
    
    Ok(())
}

/// Test that mocks can simulate errors.
#[tokio::test]
async fn test_mock_driver_error() -> Result<(), Box<dyn std::error::Error>> {
    let mock = MockDriver::new("mock_provider", "mock_model");
    
    // Configure error
    mock.set_error(ModelsError::from(ModelsErrorKind::Builder(
        "Simulated error".to_string()
    ))).await;
    
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("test".to_string())])
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    let result = mock.generate(&request).await;
    assert!(result.is_err());
    assert_eq!(mock.call_count().await, 1);
    
    Ok(())
}

/// Test multiple calls to mock.
#[tokio::test]
async fn test_mock_driver_multiple_calls() -> ModelsResult<()> {
    let mock = MockDriver::new("test", "test");
    
    let response = GenerateResponse::builder()
        .outputs(vec![Output::Text("response".to_string())])
        .stop_reason(StopReason::EndTurn)
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("test".to_string())])
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()
        .map_err(|e| ModelsError::from(ModelsErrorKind::Builder(e.to_string())))?;
    
    // Call multiple times
    for i in 1..=3 {
        mock.set_response(response.clone()).await;
        let _ = mock.generate(&request).await?;
        assert_eq!(mock.call_count().await, i);
    }
    
    Ok(())
}
