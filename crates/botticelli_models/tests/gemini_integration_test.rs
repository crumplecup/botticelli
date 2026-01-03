#![cfg(feature = "gemini")]

// Tests for Gemini model selection functionality.
//
// These tests validate that the GeminiClient correctly uses the model
// specified in GenerateRequest.model, which is critical for:
// - Multi-model narrative execution
// - Cost control (using cheaper models when appropriate)
// - Feature testing across different model capabilities
//
// ## Test Strategy
//
// Most tests use MockGeminiClient for fast, deterministic testing without API calls.
// A small number of integration tests (marked with `#[cfg_attr(not(feature = "api"), ignore)]`)
// hit the real Gemini API to validate end-to-end behavior.

mod test_utils;

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_models::GeminiClient;
use test_utils::MockGeminiClient;

use botticelli_error::{BotticelliResult, BuilderError, BuilderErrorKind};

//
// ─── MOCK TESTS (FAST, NO API CALLS) ───────────────────────────────────────────
//

/// Test basic generate functionality using mock.
#[tokio::test]
async fn test_mock_model_basic_generate() -> BotticelliResult<()> {
    let mock = MockGeminiClient::new_success("Mock response");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(format!("{:?}", e))))?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(e)))?;

    let response = mock.generate(&request).await?;
    assert!(!response.outputs().is_empty());
    Ok(())
}

/// Test that model_name() returns the correct default for mock.
#[test]
fn test_mock_model_name() {
    let mock = MockGeminiClient::new_success("test");
    assert_eq!(mock.model_name(), "mock-gemini");
}

/// Test provider_name() returns "mock-gemini" for mock.
#[test]
fn test_mock_provider_name() {
    let mock = MockGeminiClient::new_success("test");
    assert_eq!(mock.provider_name(), "mock-gemini");
}

//
// ─── INTEGRATION TESTS (REAL API, REQUIRE API KEY) ─────────────────────────────
//

/// Test that GeminiClient uses the default model when no model is specified.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_default_model_usage() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    // The default model should be gemini-2.0-flash-lite (for development)
    assert_eq!(client.model_name(), "gemini-2.0-flash-lite");

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .model(None) // No model override
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    let response = client.generate(&request).await?;

    // Should get a response (validates default model works)
    assert!(!response.outputs().is_empty());
    Ok(())
}

/// Test that GeminiClient respects the model override in GenerateRequest.
///
/// This validates that per-request model selection works correctly.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_model_override_in_request() -> anyhow::Result<()> {
    let client = GeminiClient::new()?;

    // Request should use gemini-2.5-flash-lite, not the default
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-flash-lite".to_string())) // Override default
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    let response = client.generate(&request).await?;

    // Verify we got a response
    assert!(!response.outputs().is_empty());
    Ok(())

    // TODO: Once fixed, this should validate the correct model was used.
    // The challenge is that the response doesn't include metadata about
    // which model generated it. We need to either:
    // 1. Trust the implementation (verify via logs/debugging)
    // 2. Use model-specific features to detect which model responded
    // 3. Mock the underlying gemini-rust client to verify the model parameter
}

/// Test model override with different Gemini 2.5 model.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_gemini_2_5_model_override() -> BotticelliResult<()> {
    let client = GeminiClient::new()?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'ok'".to_string())])
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(format!("{:?}", e))))?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-flash".to_string()))
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(e)))?;

    let response = client.generate(&request).await?;

    assert!(!response.outputs().is_empty());
    Ok(())
}

/// Test that multiple requests with different models work correctly.
///
/// This simulates what happens in narrative execution with per-act model overrides.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_multiple_model_requests() -> BotticelliResult<()> {
    let client = GeminiClient::new()?;

    // Request 1: Use lite model
    let message1 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'one'".to_string())])
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(format!("{:?}", e))))?;

    let request1 = GenerateRequest::builder()
        .messages(vec![message1])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-flash-lite".to_string()))
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(e)))?;

    let response1 = client.generate(&request1).await?;
    assert!(!response1.outputs().is_empty());

    // Request 2: Use standard model
    let message2 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'two'".to_string())])
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(format!("{:?}", e))))?;

    let request2 = GenerateRequest::builder()
        .messages(vec![message2])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-flash".to_string()))
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(e)))?;

    let response2 = client.generate(&request2).await?;
    assert!(!response2.outputs().is_empty());

    // Request 3: Use pro model
    let message3 = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'three'".to_string())])
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(format!("{:?}", e))))?;

    let request3 = GenerateRequest::builder()
        .messages(vec![message3])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-pro".to_string()))
        .build()
        .map_err(|e| BuilderError::new(BuilderErrorKind::ValidationFailed(e)))?;

    let response3 = client.generate(&request3).await?;
    assert!(!response3.outputs().is_empty());
    Ok(())
}
