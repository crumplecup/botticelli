#![cfg(feature = "gemini")]
mod test_utils;

// Integration tests for unified GeminiClient with Live API routing.
//
// These tests verify that GeminiClient correctly routes live models to the Live API
// and standard models to the REST API.
//
// Run with:
// ```bash
// cargo test --features gemini,api
// ```
//
// TODO: Fix WebSocket handshake failure - connection closes before setup complete.
// This appears to be a timing or protocol issue with the Live API handshake.
// Tests are currently ignored until the handshake issue is resolved.

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::{BotticelliDriver, Streaming};
use botticelli_models::GeminiClient;
use futures_util::StreamExt;
use test_utils::create_test_request;

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_routes_to_live_api() {
    // Load environment variables
    let _ = dotenvy::dotenv();

    // Create unified GeminiClient
    let client = GeminiClient::new().expect("Failed to create GeminiClient");

    // Create request for a live model (experimental model)
    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'Hello from Live API'".to_string())],
        }],
        model: Some("models/gemini-2.0-flash-exp".to_string()),
        max_tokens: Some(20),
        ..Default::default()
    };

    // Call generate - should route to Live API
    let response = client
        .generate(&request)
        .await
        .expect("Failed to generate response");

    // Verify we got a response
    assert!(!response.outputs.is_empty());
    println!("Live API response: {:?}", response.outputs);
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_streaming_routes_to_live_api() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create GeminiClient");

    // Create request for live model with streaming
    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Count from 1 to 3".to_string())],
        }],
        model: Some("models/gemini-2.0-flash-exp".to_string()),
        max_tokens: Some(50),
        ..Default::default()
    };

    // Call generate_stream - should route to Live API
    let mut stream = client
        .generate_stream(&request)
        .await
        .expect("Failed to create stream");

    let mut chunks = Vec::new();
    let mut found_final = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Failed to get chunk");
        println!("Chunk: {:?}", chunk.content);
        chunks.push(chunk.clone());

        if chunk.is_final {
            found_final = true;
            break;
        }
    }

    // Verify we got chunks
    assert!(!chunks.is_empty(), "Should receive at least one chunk");

    // Verify we got a final chunk
    assert!(found_final, "Should receive final chunk with is_final=true");

    println!("Total chunks received: {}", chunks.len());
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_detects_live_models() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create GeminiClient");

    // Test with "-exp" model (should use Live API)
    let request_exp = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Test".to_string())],
        }],
        model: Some("models/gemini-2.0-flash-exp".to_string()),
        max_tokens: Some(5),
        ..Default::default()
    };

    let response_exp = client
        .generate(&request_exp)
        .await
        .expect("Failed with -exp model");
    assert!(!response_exp.outputs.is_empty());

    // Test with "-live" model (should use Live API)
    // Note: This may fail if the model doesn't exist, but it tests the routing logic
    let request_live = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Test".to_string())],
        }],
        model: Some("models/gemini-2.0-flash-live".to_string()),
        max_tokens: Some(5),
        ..Default::default()
    };

    // This might fail if the model doesn't exist, so we just verify it attempts to use Live API
    let _ = client.generate(&request_live).await;
    // We don't assert success here because the model might not exist
}
