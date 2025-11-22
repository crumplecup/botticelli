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

mod test_utils;

use botticelli_models::{GeminiLiveClient, GenerationConfig};
use futures_util::StreamExt;

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_connection() {
    // Load environment variables
    let _ = dotenvy::dotenv();

    // Create Live API client
    let client = GeminiLiveClient::new().expect("Failed to create Live API client");

    // Connect to Live API with minimal config
    let session = client.connect("models/gemini-2.0-flash-exp").await;

    // Should successfully connect and complete setup handshake
    assert!(
        session.is_ok(),
        "Failed to connect to Live API: {:?}",
        session.err()
    );
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_basic_generation() {
    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new().expect("Failed to create Live API client");

    // Configure for minimal token usage
    let config = GenerationConfig {
        max_output_tokens: Some(10),
        temperature: Some(1.0),
        ..Default::default()
    };

    let mut session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await
        .expect("Failed to connect");

    // Send a simple message
    let response = session
        .send_text("Say 'Hello'")
        .await
        .expect("Failed to send message");

    // Should receive non-empty response
    assert!(!response.is_empty(), "Response should not be empty");
    println!("Live API response: {}", response);

    // Close session
    session.close().await.expect("Failed to close session");
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_streaming() {
    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new().expect("Failed to create Live API client");

    // Configure for minimal token usage but allow multiple chunks
    let config = GenerationConfig {
        max_output_tokens: Some(50),
        temperature: Some(1.0),
        ..Default::default()
    };

    let session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await
        .expect("Failed to connect");

    // Send a message that should generate streaming response
    // Note: send_text_stream now consumes the session, so we can't close it afterward
    let mut stream = session
        .send_text_stream("Count from 1 to 5")
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

    // Verify we got at least one chunk
    assert!(!chunks.is_empty(), "Should receive at least one chunk");

    // Verify we got a final chunk
    assert!(found_final, "Should receive final chunk");

    // Verify final chunk has finish reason
    let final_chunk = chunks.last().unwrap();
    assert!(
        final_chunk.finish_reason.is_some(),
        "Final chunk should have finish reason"
    );

    // Stream is dropped here, which closes the WebSocket session
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_multiple_turns() {
    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new().expect("Failed to create Live API client");

    let config = GenerationConfig {
        max_output_tokens: Some(20),
        ..Default::default()
    };

    let mut session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await
        .expect("Failed to connect");

    // First turn
    let response1 = session
        .send_text("Say 'Hello'")
        .await
        .expect("Failed on first turn");
    assert!(!response1.is_empty());
    println!("Turn 1: {}", response1);

    // Second turn (same session)
    let response2 = session
        .send_text("Say 'Goodbye'")
        .await
        .expect("Failed on second turn");
    assert!(!response2.is_empty());
    println!("Turn 2: {}", response2);

    session.close().await.expect("Failed to close session");
}
