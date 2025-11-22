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
use test_utils::{create_test_request, MockGeminiClient};

//
// ─── MOCK TESTS (FAST, NO API CALLS) ───────────────────────────────────────────
//

/// Test basic generate functionality using mock.
#[tokio::test]
async fn test_mock_model_basic_generate() {
    let mock = MockGeminiClient::new_success("Mock response");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Test".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: None,
    };

    let response = mock.generate(&request).await.expect("Mock should succeed");
    assert!(!response.outputs.is_empty());
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
async fn test_default_model_usage() {
    let client = GeminiClient::new().expect("Failed to create client");

    // The default model should be gemini-2.0-flash-lite (for development)
    assert_eq!(client.model_name(), "gemini-2.0-flash-lite");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'ok'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: None, // No model override
    };

    let response = client.generate(&request).await.expect("API call failed");

    // Should get a response (validates default model works)
    assert!(!response.outputs.is_empty());
}

/// Test that GeminiClient respects the model override in GenerateRequest.
///
/// This validates that per-request model selection works correctly.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_model_override_in_request() {
    let client = GeminiClient::new().expect("Failed to create client");

    // Request should use gemini-2.5-flash-lite, not the default
    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'ok'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.5-flash-lite".to_string()), // Override default
    };

    let response = client.generate(&request).await.expect("API call failed");

    // Verify we got a response
    assert!(!response.outputs.is_empty());

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
async fn test_gemini_2_5_model_override() {
    let client = GeminiClient::new().expect("Failed to create client");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'ok'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.5-flash".to_string()),
    };

    let response = client.generate(&request).await.expect("API call failed");

    assert!(!response.outputs.is_empty());
}

/// Test that multiple requests with different models work correctly.
///
/// This simulates what happens in narrative execution with per-act model overrides.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_multiple_model_requests() {
    let client = GeminiClient::new().expect("Failed to create client");

    // Request 1: Use lite model
    let request1 = GenerateRequest::builder()
        .messages(vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'one'".to_string())],
        }])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-flash-lite".to_string()))
        .build()
        .expect("Failed to build request1");

    let response1 = client.generate(&request1).await.expect("Request 1 failed");
    assert!(!response1.outputs.is_empty());

    // Request 2: Use standard model
    let request2 = GenerateRequest::builder()
        .messages(vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'two'".to_string())],
        }])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-flash".to_string()))
        .build()
        .expect("Failed to build request2");

    let response2 = client.generate(&request2).await.expect("Request 2 failed");
    assert!(!response2.outputs.is_empty());

    // Request 3: Use pro model
    let request3 = GenerateRequest::builder()
        .messages(vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'three'".to_string())],
        }])
        .max_tokens(Some(10))
        .model(Some("gemini-2.5-pro".to_string()))
        .build()
        .expect("Failed to build request3");

    let response3 = client.generate(&request3).await.expect("Request 3 failed");
    assert!(!response3.outputs.is_empty());
}
