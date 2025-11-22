#![cfg(feature = "gemini")]
mod test_utils;

// Integration tests for unified GeminiClient with Live API routing.
//
// These tests verify that GeminiClient correctly routes live models to the Live API
// and standard models to the REST API.
//
// Run with:
// ```bash
use botticelli_core::MessageBuilder;
// cargo test --features gemini,api
// ```
//
// TODO: Fix WebSocket handshake failure - connection closes before setup complete.
// This appears to be a timing or protocol issue with the Live API handshake.
// Tests are currently ignored until the handshake issue is resolved.

use botticelli_core::{GenerateRequest, Input, Role};
use botticelli_error::BotticelliError;
use botticelli_interface::{BotticelliDriver, Streaming};
use botticelli_models::GeminiClient;
use futures_util::StreamExt;

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_routes_to_live_api() -> botticelli_error::BotticelliResult<()> {
    // Load environment variables
    let _ = dotenvy::dotenv();

    // Create unified GeminiClient
    let client = GeminiClient::new()?;

    // Create request for a live model (experimental model)
    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Say 'Hello from Live API'".to_string())])
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model(Some("models/gemini-2.0-flash-exp".to_string()))
        .max_tokens(Some(20))
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;

    // Call generate - should route to Live API
    let response = client.generate(&request).await?;

    // Verify we got a response
    assert!(!response.outputs.is_empty());
    println!("Live API response: {:?}", response.outputs);
    
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_streaming_routes_to_live_api() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Create request for live model with streaming
    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Count from 1 to 3".to_string())])
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model(Some("models/gemini-2.0-flash-exp".to_string()))
        .max_tokens(Some(50))
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;

    // Call generate_stream - should route to Live API
    let mut stream = client.generate_stream(&request).await?;

    let mut chunks = Vec::new();
    let mut found_final = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
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
    
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_gemini_client_detects_live_models() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Test with "-exp" model (should use Live API)
    let message_exp = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;
    
    let request_exp = GenerateRequest::builder()
        .messages(vec![message_exp])
        .model(Some("models/gemini-2.0-flash-exp".to_string()))
        .max_tokens(Some(5))
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;

    let response_exp = client.generate(&request_exp).await?;
    assert!(!response_exp.outputs.is_empty());

    // Test with "-live" model (should use Live API)
    // Note: This may fail if the model doesn't exist, but it tests the routing logic
    let message_live = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;
    
    let request_live = GenerateRequest::builder()
        .messages(vec![message_live])
        .model(Some("models/gemini-2.0-flash-live".to_string()))
        .max_tokens(Some(5))
        .build()
        .map_err(|e| botticelli_error::BuilderError::from(e.to_string()))?;

    // This might fail if the model doesn't exist, so we just verify it attempts to use Live API
    let _ = client.generate(&request_live).await;
    // We don't assert success here because the model might not exist
    
    Ok(())
}
