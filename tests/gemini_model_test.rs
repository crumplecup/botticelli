//! Tests for Gemini model selection functionality.
//!
//! These tests validate that the GeminiClient correctly uses the model
//! specified in GenerateRequest.model, which is critical for:
//! - Multi-model narrative execution
//! - Cost control (using cheaper models when appropriate)
//! - Feature testing across different model capabilities

#![cfg(feature = "gemini")]

use boticelli::{BoticelliDriver, GeminiClient, GenerateRequest, Input, Message, Role};

/// Test that GeminiClient uses the default model when no model is specified.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn test_default_model_usage() {
    let client = GeminiClient::new().expect("Failed to create client");

    // The default model should be gemini-2.5-flash
    assert_eq!(client.model_name(), "gemini-2.5-flash");

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
#[ignore] // Requires GEMINI_API_KEY
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
#[ignore] // Requires GEMINI_API_KEY
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
#[ignore] // Requires GEMINI_API_KEY
async fn test_multiple_model_requests() {
    let client = GeminiClient::new().expect("Failed to create client");

    // Request 1: Use lite model
    let request1 = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'one'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.5-flash-lite".to_string()),
    };

    let response1 = client.generate(&request1).await.expect("Request 1 failed");
    assert!(!response1.outputs.is_empty());

    // Request 2: Use standard model
    let request2 = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'two'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.5-flash".to_string()),
    };

    let response2 = client.generate(&request2).await.expect("Request 2 failed");
    assert!(!response2.outputs.is_empty());

    // Request 3: Use pro model
    let request3 = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'three'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.5-pro".to_string()),
    };

    let response3 = client.generate(&request3).await.expect("Request 3 failed");
    assert!(!response3.outputs.is_empty());
}

/// Test that model_name() method returns the correct value.
///
/// Currently returns the default, but should return the model being used
/// for the current request (or default if not in a request context).
#[test]
fn test_model_name_method() {
    let client = GeminiClient::new().expect("Failed to create client");

    // Should return the default model
    assert_eq!(client.model_name(), "gemini-2.5-flash");
}

/// Integration test: Run the text_models narrative to verify multi-model support.
///
/// This test exercises the full narrative executor with different models per act.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY and narrative file
async fn test_narrative_multi_model_execution() {
    use boticelli::{Narrative, NarrativeExecutor};
    use std::path::Path;

    let client = GeminiClient::new().expect("Failed to create client");
    let executor = NarrativeExecutor::new(client);

    let narrative_path = Path::new("narrations/text_models.toml");
    let narrative =
        Narrative::from_file(narrative_path).expect("Failed to load text_models.toml narrative");

    let execution = executor
        .execute(&narrative)
        .await
        .expect("Narrative execution failed");

    // Should have executed 3 acts
    assert_eq!(execution.act_executions.len(), 3);

    // Verify each act used the correct model
    assert_eq!(
        execution.act_executions[0].model,
        Some("gemini-2.5-flash-lite".to_string())
    );
    assert_eq!(
        execution.act_executions[1].model,
        Some("gemini-2.5-flash".to_string())
    );
    assert_eq!(
        execution.act_executions[2].model,
        Some("gemini-2.5-pro".to_string())
    );

    // All acts should have produced responses
    for act in &execution.act_executions {
        assert!(!act.response.is_empty());
    }
}
