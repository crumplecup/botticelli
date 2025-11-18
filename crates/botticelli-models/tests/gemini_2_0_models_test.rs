use botticelli_core::{GenerateRequest, GenerateResponse, Input, Message, MessageRole as Role, FinishReason};
use botticelli_interface::{BotticelliDriver, Streaming};
//! Tests for Gemini 2.0 model compatibility.
//!
//! These tests validate that older Gemini 2.0 models work correctly
//! via the Model::Custom() variant with proper "models/" prefix.

#![cfg(feature = "gemini")]

use botticelli_models::{BotticelliDriver, GeminiClient, GenerateRequest, Input, Message, Role};

/// Test that Gemini 2.0 Flash works via Model::Custom with "models/" prefix.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_gemini_2_0_flash() {
    let client = GeminiClient::new().expect("Failed to create client");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'ok'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.0-flash".to_string()),
    };

    let response = client
        .generate(&request)
        .await
        .expect("API call failed with gemini-2.0-flash");

    assert!(!response.outputs.is_empty());
}

/// Test that Gemini 2.0 Flash Lite works via Model::Custom.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_gemini_2_0_flash_lite() {
    let client = GeminiClient::new().expect("Failed to create client");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'ok'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.0-flash-lite".to_string()),
    };

    let response = client
        .generate(&request)
        .await
        .expect("API call failed with gemini-2.0-flash-lite");

    assert!(!response.outputs.is_empty());
}

/// Test that multiple requests with mixed 2.0 and 2.5 models work correctly.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_mixed_2_0_and_2_5_models() {
    let client = GeminiClient::new().expect("Failed to create client");

    // Request 1: Use Gemini 2.0 Flash
    let request1 = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'one'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.0-flash".to_string()),
    };

    let response1 = client.generate(&request1).await.expect("Request 1 failed");
    assert!(!response1.outputs.is_empty());

    // Request 2: Use Gemini 2.5 Flash (should use enum variant)
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

    // Request 3: Use Gemini 2.0 Flash Lite
    let request3 = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'three'".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("gemini-2.0-flash-lite".to_string()),
    };

    let response3 = client.generate(&request3).await.expect("Request 3 failed");
    assert!(!response3.outputs.is_empty());
}

/// Test that explicit "models/" prefix is preserved.
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_explicit_models_prefix() {
    let client = GeminiClient::new().expect("Failed to create client");

    // Use explicit "models/" prefix
    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Hello".to_string())],
        }],
        max_tokens: Some(10),
        temperature: None,
        model: Some("models/gemini-2.0-flash".to_string()),
    };

    let response = client
        .generate(&request)
        .await
        .expect("API call failed with explicit models/ prefix");

    assert!(!response.outputs.is_empty());
}
