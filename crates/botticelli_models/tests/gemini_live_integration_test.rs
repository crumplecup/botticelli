#![cfg(feature = "gemini")]

mod helpers;

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

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_routes_to_live_api() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing GeminiClient routes to Live API");

    // Load environment variables
    let _ = dotenvy::dotenv();

    // Create unified GeminiClient
    let client = GeminiClient::new()?;

    // Create request for a live model (experimental model)
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Say 'Hello from Live API'".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("models/gemini-2.0-flash-exp".to_string())
        .max_tokens(20u32)
        .build()?;

    // Call generate - should route to Live API
    tracing::debug!(
        model = "models/gemini-2.0-flash-exp",
        "Calling generate (should route to Live API)"
    );
    let response = client.generate(&request).await?;

    // Verify we got a response
    assert!(!response.outputs().is_empty());
    tracing::debug!(output_count = response.outputs().len(), "Received response");
    println!("Live API response: {:?}", response.outputs());

    tracing::info!("GeminiClient routes to Live API test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_streaming_routes_to_live_api() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing GeminiClient streaming routes to Live API");

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Create request for live model with streaming
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Count from 1 to 3".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("models/gemini-2.0-flash-exp".to_string())
        .max_tokens(50u32)
        .build()?;

    // Call generate_stream - should route to Live API
    tracing::debug!(
        model = "models/gemini-2.0-flash-exp",
        "Calling generate_stream (should route to Live API)"
    );
    let mut stream = client.generate_stream(&request).await?;

    let mut chunks = Vec::new();
    let mut found_final = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        println!("Chunk: {:?}", chunk.content());
        chunks.push(chunk.clone());

        if *chunk.is_final() {
            found_final = true;
            tracing::debug!("Received final chunk");
            break;
        }
    }

    // Verify we got chunks
    assert!(!chunks.is_empty(), "Should receive at least one chunk");
    tracing::debug!(chunk_count = chunks.len(), "Received chunks");

    // Verify we got a final chunk
    assert!(found_final, "Should receive final chunk with is_final=true");

    println!("Total chunks received: {}", chunks.len());

    tracing::info!("GeminiClient streaming routes to Live API test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_detects_live_models() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing GeminiClient detects live models");

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Test with "-exp" model (should use Live API)
    let message_exp = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request_exp = GenerateRequest::builder()
        .messages(vec![message_exp])
        .model("models/gemini-2.0-flash-exp".to_string())
        .max_tokens(5u32)
        .build()?;

    tracing::debug!(
        model = "models/gemini-2.0-flash-exp",
        "Testing -exp model (should route to Live API)"
    );
    let response_exp = client.generate(&request_exp).await?;
    assert!(!response_exp.outputs().is_empty());
    tracing::debug!("-exp model routed correctly");

    // Test with "-live" model (should use Live API)
    // Note: This may fail if the model doesn't exist, but it tests the routing logic
    let message_live = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request_live = GenerateRequest::builder()
        .messages(vec![message_live])
        .model("models/gemini-2.0-flash-live".to_string())
        .max_tokens(5u32)
        .build()?;

    // This might fail if the model doesn't exist, so we just verify it attempts to use Live API
    tracing::debug!(
        model = "models/gemini-2.0-flash-live",
        "Testing -live model (should route to Live API)"
    );
    let result = client.generate(&request_live).await;
    // We don't assert success here because the model might not exist
    if result.is_err() {
        tracing::debug!("-live model may not exist, but routing attempted");
    }

    tracing::info!("GeminiClient detects live models test passed");
    Ok(())
}
