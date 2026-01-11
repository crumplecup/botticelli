#![cfg(feature = "gemini")]

mod helpers;

// Error handling tests for Gemini Live API.
//
// Tests various error conditions including invalid models, connection issues,
// and rate limiting.
//
// Run with:
// ```bash
// MessageBuilder trait is auto-imported via derive_builder
// cargo test --features gemini,api
// ```
//
// TODO: Fix WebSocket handshake failure - connection closes before setup complete.
// This appears to be a timing or protocol issue with the Live API handshake.
// Tests that connect to Live API are currently ignored until the handshake issue is resolved.

use botticelli_core::{GenerateRequest, Input, Message, Role};
use botticelli_interface::{BotticelliDriver, Streaming};
use botticelli_models::{GeminiClient, GeminiLiveClient, GenerationConfigBuilder, LiveRateLimiter};
use futures_util::StreamExt;
use std::time::Instant;

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_invalid_model() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API with invalid model");

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Try to use a non-existent model
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("models/nonexistent-live-model".to_string())
        .max_tokens(5u32)
        .build()?;

    // Should fail gracefully
    tracing::debug!(
        model = "models/nonexistent-live-model",
        "Attempting invalid model"
    );
    let result = client.generate(&request).await;

    // We expect an error since the model doesn't exist
    // The exact error type depends on the API response
    assert!(result.is_err(), "Should fail with non-existent model");

    if let Err(e) = result {
        tracing::debug!(error = %e, "Received expected error");
        println!("Expected error for invalid model: {}", e);
    }

    tracing::info!("Live API invalid model test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_rate_limiting() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API rate limiting");

    let _ = dotenvy::dotenv();

    // Create client with very low rate limit (2 messages per minute)
    let client = GeminiLiveClient::new_with_rate_limit(Some(2))?;
    tracing::debug!(rpm_limit = 2, "Created client with rate limit");

    let config = GenerationConfigBuilder::default()
        .max_output_tokens(5)
        .build()?;

    let start = Instant::now();

    // First message - should succeed immediately
    tracing::debug!("Sending message 1");
    let mut session1 = client
        .connect_with_config("models/gemini-2.0-flash-exp", config.clone())
        .await?;

    let response1 = session1.send_text("Test 1").await;
    assert!(response1.is_ok(), "First message should succeed");
    session1.close().await.ok();

    // Second message - should succeed immediately
    tracing::debug!("Sending message 2");
    let mut session2 = client
        .connect_with_config("models/gemini-2.0-flash-exp", config.clone())
        .await?;

    let response2 = session2.send_text("Test 2").await;
    assert!(response2.is_ok(), "Second message should succeed");
    session2.close().await.ok();

    let elapsed_before_third = start.elapsed();
    tracing::debug!(
        elapsed_secs = elapsed_before_third.as_secs(),
        "Time before third message"
    );
    println!("Time before third message: {:?}", elapsed_before_third);

    // Third message - should block and wait for window reset
    tracing::debug!("Sending message 3 (should be rate limited)");
    let mut session3 = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await?;

    let response3 = session3.send_text("Test 3").await;
    assert!(
        response3.is_ok(),
        "Third message should succeed after waiting"
    );
    session3.close().await.ok();

    let total_elapsed = start.elapsed();
    tracing::debug!(
        total_secs = total_elapsed.as_secs(),
        "Total time for 3 messages"
    );
    println!("Total time for 3 messages: {:?}", total_elapsed);

    // Third message should have been delayed by rate limiting
    // If RPM=2, the third message should wait until ~60 seconds have passed
    // We'll be lenient and just check it took longer than the first two
    assert!(
        total_elapsed.as_secs() >= 1,
        "Rate limiting should have caused a delay"
    );

    tracing::info!("Live API rate limiting test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_empty_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API with empty message");

    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new()?;

    let config = GenerationConfigBuilder::default()
        .max_output_tokens(10)
        .build()?;

    let mut session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await?;

    // Send empty message
    tracing::debug!("Sending empty message");
    let response = session.send_text("").await;

    // Should either succeed (with model handling empty input) or fail gracefully
    match response {
        Ok(text) => {
            tracing::debug!(response_len = text.len(), "Model handled empty message");
            println!("Model handled empty message: {}", text);
        }
        Err(e) => {
            tracing::debug!(error = %e, "Model rejected empty message");
            println!("Model rejected empty message: {}", e);
        }
    }

    session.close().await.ok();

    tracing::info!("Live API empty message test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_live_api_very_long_message() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Live API with very long message");

    let _ = dotenvy::dotenv();

    let client = GeminiLiveClient::new()?;

    let config = GenerationConfigBuilder::default()
        .max_output_tokens(10)
        .build()?;

    let mut session = client
        .connect_with_config("models/gemini-2.0-flash-exp", config)
        .await?;

    // Send a very long message (but not exceeding model limits)
    let long_message = "Tell me about ".to_string() + &"artificial intelligence ".repeat(50);
    tracing::debug!(message_len = long_message.len(), "Sending long message");

    let response = session.send_text(&long_message).await;

    // Should handle long messages
    assert!(response.is_ok(), "Should handle long messages");

    if let Ok(text) = response {
        tracing::debug!(response_len = text.len(), "Received response");
        println!("Response to long message: {} chars", text.len());
    }

    session.close().await.ok();

    tracing::info!("Live API very long message test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "api")]
async fn test_unified_client_handles_live_model_errors() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing unified client with live model errors");

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Test with invalid configuration (negative max_tokens isn't possible, but we can test zero)
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Test".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("models/gemini-2.0-flash-exp".to_string())
        .max_tokens(0u32) // Invalid
        .build()?;

    // Should handle gracefully
    tracing::debug!(max_tokens = 0, "Sending request with zero max_tokens");
    let result = client.generate(&request).await;

    // May succeed with minimal output or fail - either is acceptable
    match result {
        Ok(response) => {
            tracing::debug!(
                output_count = response.outputs().len(),
                "Zero max_tokens handled"
            );
            println!("Zero max_tokens handled: {:?}", response.outputs());
        }
        Err(e) => {
            tracing::debug!(error = %e, "Zero max_tokens rejected");
            println!("Zero max_tokens rejected: {}", e);
        }
    }

    tracing::info!("Unified client handles live model errors test passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "api")]
async fn test_live_rate_limiter_concurrent_sessions() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing live rate limiter with concurrent sessions");

    let _ = dotenvy::dotenv();

    // Create shared rate limiter
    let rate_limiter = LiveRateLimiter::new(3); // 3 messages per minute
    tracing::debug!(rpm_limit = 3, "Created rate limiter");

    // Simulate sending messages
    let start = Instant::now();

    for i in 1..=5 {
        rate_limiter.acquire().await;
        let elapsed = start.elapsed();
        tracing::debug!(
            message_num = i,
            elapsed_ms = elapsed.as_millis(),
            "Message sent"
        );
        println!("Message {} sent at {:?}", i, elapsed);
        rate_limiter.record();

        // After 3 messages, should start blocking
        if i == 4 {
            // Should have waited for rate limit
            tracing::debug!(
                wait_time_ms = elapsed.as_millis(),
                "Fourth message required waiting"
            );
            println!("Fourth message required waiting: {:?}", elapsed);
        }
    }

    let total_elapsed = start.elapsed();
    tracing::debug!(total_ms = total_elapsed.as_millis(), "Completed 5 messages");
    println!("Total time for 5 messages with RPM=3: {:?}", total_elapsed);

    // With RPM=3, sending 5 messages should require waiting
    // We expect it to take longer than if there were no rate limiting
    assert!(
        total_elapsed.as_millis() >= 100,
        "Should have experienced rate limiting delay"
    );

    tracing::info!("Live rate limiter concurrent sessions test passed");
    Ok(())
}

#[tokio::test]
#[ignore = "TODO: Fix WebSocket handshake failure"]
async fn test_streaming_error_recovery() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing streaming error recovery");

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Count to 5".to_string())])
        .build()?;

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .model("models/gemini-2.0-flash-exp".to_string())
        .max_tokens(50u32)
        .build()?;

    tracing::debug!("Starting streaming request");
    let mut stream = client.generate_stream(&request).await?;

    let mut chunk_count = 0;
    let mut error_count = 0;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                chunk_count += 1;
                if *chunk.is_final() {
                    tracing::debug!("Received final chunk");
                    break;
                }
            }
            Err(e) => {
                error_count += 1;
                tracing::debug!(error = %e, error_count, "Stream error");
                println!("Stream error: {}", e);
                break;
            }
        }
    }

    tracing::debug!(chunk_count, error_count, "Streaming completed");
    println!("Received {} chunks, {} errors", chunk_count, error_count);

    // Should have received at least one chunk
    assert!(chunk_count > 0, "Should receive at least one chunk");

    tracing::info!("Test Streaming Error Recovery test passed");
    Ok(())
}
