//! Integration tests for Gemini Live API basic functionality.
//!
//! These tests require the `api` feature flag to run:
//! ```bash
//! cargo test --features gemini,api
//! ```
//!
//! TODO: Fix WebSocket handshake failure - connection closes before setup complete.
//! This appears to be a timing or protocol issue with the Live API handshake.
//! Tests are currently ignored until the handshake issue is resolved.


#![cfg(feature = "gemini")]

mod helpers;


use botticelli_models::{GeminiLiveClient, GenerationConfig};
use futures_util::StreamExt;

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_connection() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API connection");
    
    // Load environment variables
    let _ = dotenvy::dotenv();

    // Create Live API client
    let client = GeminiLiveClient::new()?;
    tracing::debug!("Created GeminiLiveClient");

    // Connect to Live API with minimal config
    let _session = client.connect("models/gemini-2.0-flash-exp").await?;
    tracing::debug!("Connected to Live API");

    tracing::info!("Live API connection test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_basic_generation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API basic generation");
    
    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new()?;

    // Configure for minimal token usage
    let mut config = GenerationConfig::default();
    config = config
        .with_max_output_tokens(Some(10))
        .with_temperature(Some(1.0));

    tracing::debug!(max_tokens = 10, "Connecting with config");
    let mut session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await?;

    // Send a simple message
    tracing::debug!("Sending message");
    let response = session.send_text("Say 'Hello'").await?;

    // Should receive non-empty response
    assert!(!response.is_empty(), "Response should not be empty");
    tracing::debug!(response_len = response.len(), "Received response");
    println!("Live API response: {}", response);

    // Close session
    session.close().await?;
    tracing::info!("Live API basic generation test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_streaming() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API streaming");
    
    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new()?;

    // Configure for minimal token usage but allow multiple chunks
    let mut config = GenerationConfig::default();
    config = config
        .with_max_output_tokens(Some(50))
        .with_temperature(Some(1.0));

    let session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await?;

    // Send a message that should generate streaming response
    // Note: send_text_stream now consumes the session, so we can't close it afterward
    tracing::debug!("Sending streaming request");
    let mut stream = session.send_text_stream("Count from 1 to 5").await?;

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

    // Verify we got at least one chunk
    assert!(!chunks.is_empty(), "Should receive at least one chunk");
    tracing::debug!(chunk_count = chunks.len(), "Received chunks");

    // Verify we got a final chunk
    assert!(found_final, "Should receive final chunk");

    // Verify final chunk has finish reason
    let final_chunk = chunks.last().unwrap();
    assert!(
        final_chunk.finish_reason().is_some(),
        "Final chunk should have finish reason"
    );

    // Stream is dropped here, which closes the WebSocket session
    tracing::info!("Live API streaming test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_multiple_turns() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API multiple turns");
    
    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new()?;

    let mut config = GenerationConfig::default();
    config = config.with_max_output_tokens(Some(20));

    let mut session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await?;

    // First turn
    tracing::debug!("Sending turn 1");
    let response1 = session.send_text("Say 'Hello'").await?;
    assert!(!response1.is_empty());
    tracing::debug!(turn = 1, response_len = response1.len(), "Received response");
    println!("Turn 1: {}", response1);

    // Second turn (same session)
    tracing::debug!("Sending turn 2");
    let response2 = session.send_text("Say 'Goodbye'").await?;
    assert!(!response2.is_empty());
    tracing::debug!(turn = 2, response_len = response2.len(), "Received response");
    println!("Turn 2: {}", response2);

    session.close().await?;
    tracing::info!("Live API multiple turns test passed");
    Ok(())
}
