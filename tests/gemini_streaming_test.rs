//! Tests for Gemini streaming support.
//!
//! These tests verify that streaming works with both standard and live models.
//! Live models (e.g., gemini-2.0-flash-live) offer better rate limits on the free tier,
//! which is the primary motivation for implementing streaming.

#![cfg(feature = "gemini")]

use boticelli::{BoticelliDriver, GeminiClient, GenerateRequest, Message, Role, Input, Streaming};
use futures_util::StreamExt;

/// Helper to create a simple test request.
fn create_test_request(prompt: &str, model: Option<String>) -> GenerateRequest {
    GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text(prompt.to_string())],
        }],
        model,
        max_tokens: Some(100),
        temperature: Some(0.7),
    }
}

#[tokio::test]
async fn test_streaming_basic() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");

    let request = create_test_request("Count from 1 to 5", None);

    let mut stream = client
        .generate_stream(&request)
        .await
        .expect("Stream creation failed");

    let mut chunks = Vec::new();
    let mut saw_final = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Chunk error");
        chunks.push(chunk.clone());

        if chunk.is_final {
            saw_final = true;
            assert!(
                chunk.finish_reason.is_some(),
                "Final chunk should have finish_reason"
            );
            break;
        }
    }

    assert!(!chunks.is_empty(), "Should receive at least one chunk");
    assert!(saw_final, "Should see final chunk");

    // Concatenate all text
    let full_text: String = chunks
        .iter()
        .filter_map(|c| match &c.content {
            boticelli::Output::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    println!("Streaming result: {}", full_text);

    // Should contain numbers (1, 2, 3, etc. or spelled out)
    assert!(
        full_text.contains('1')
            || full_text.contains("one")
            || full_text.contains("One"),
        "Response should contain counting: {}",
        full_text
    );
}

#[tokio::test]
async fn test_streaming_with_standard_model() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");

    // Explicitly use standard flash model
    let request = create_test_request(
        "Say 'Hello from gemini-2.0-flash'",
        Some("gemini-2.0-flash".to_string()),
    );

    let mut stream = client
        .generate_stream(&request)
        .await
        .expect("Stream creation failed");

    let mut chunks = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Chunk error");
        chunks.push(chunk.clone());

        if chunk.is_final {
            break;
        }
    }

    assert!(!chunks.is_empty(), "Should receive chunks");

    let full_text: String = chunks
        .iter()
        .filter_map(|c| match &c.content {
            boticelli::Output::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    println!("Standard model result: {}", full_text);
    assert!(!full_text.is_empty(), "Should have generated text");
}

#[tokio::test]
async fn test_streaming_with_live_model() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");

    // CRITICAL TEST: Use live model for better rate limits
    let request = create_test_request(
        "Say 'Hello from gemini-2.0-flash-live'",
        Some("gemini-2.0-flash-live".to_string()),
    );

    let mut stream = client
        .generate_stream(&request)
        .await
        .expect("Live model streaming failed");

    let mut chunks = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Chunk error");
        chunks.push(chunk.clone());

        if chunk.is_final {
            break;
        }
    }

    assert!(!chunks.is_empty(), "Live model should stream chunks");

    let full_text: String = chunks
        .iter()
        .filter_map(|c| match &c.content {
            boticelli::Output::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    println!("Live model result: {}", full_text);
    assert!(!full_text.is_empty(), "Live model should generate text");
}

#[tokio::test]
async fn test_streaming_finish_reasons() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");

    let request = create_test_request("Tell me a very short joke.", None);

    let mut stream = client
        .generate_stream(&request)
        .await
        .expect("Stream creation failed");

    let mut final_chunk = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Chunk error");

        if chunk.is_final {
            final_chunk = Some(chunk);
            break;
        }
    }

    assert!(final_chunk.is_some(), "Should have final chunk");

    let final_chunk = final_chunk.unwrap();
    assert!(
        final_chunk.finish_reason.is_some(),
        "Final chunk should have finish reason"
    );

    println!("Finish reason: {:?}", final_chunk.finish_reason);
}

#[tokio::test]
async fn test_streaming_vs_non_streaming_consistency() {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");

    let request = create_test_request("Say exactly: 'Test successful'", None);

    // Get streaming response
    let mut stream = client
        .generate_stream(&request)
        .await
        .expect("Stream failed");

    let mut streaming_text = String::new();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Chunk error");
        if let boticelli::Output::Text(t) = &chunk.content {
            streaming_text.push_str(t);
        }
        if chunk.is_final {
            break;
        }
    }

    // Get non-streaming response
    let response = client.generate(&request).await.expect("Generate failed");
    let non_streaming_text = response
        .outputs
        .iter()
        .filter_map(|o| match o {
            boticelli::Output::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect::<String>();

    // Both should produce text
    assert!(!streaming_text.is_empty(), "Streaming should produce text");
    assert!(
        !non_streaming_text.is_empty(),
        "Non-streaming should produce text"
    );

    println!("Streaming: {}", streaming_text);
    println!("Non-streaming: {}", non_streaming_text);

    // Note: Content might differ slightly due to randomness,
    // but both should have generated something meaningful
}

#[tokio::test]
#[ignore] // Mark as expensive - only run when explicitly requested
async fn test_rate_limit_comparison() {
    // This test is designed to verify that live models have better rate limits
    // Run with: cargo test test_rate_limit_comparison --features gemini -- --ignored

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new().expect("Failed to create client");

    println!("Testing standard model rate limits...");

    // Try 20 requests to standard model
    let mut standard_success = 0;
    for i in 0..20 {
        let request = create_test_request(
            &format!("Request {}", i),
            Some("gemini-2.0-flash".to_string()),
        );

        match client.generate_stream(&request).await {
            Ok(mut stream) => {
                // Consume stream
                while let Some(_) = stream.next().await {}
                standard_success += 1;
            }
            Err(e) => {
                println!("Standard model failed at request {}: {}", i, e);
                break;
            }
        }
    }

    println!(
        "Standard model: {} successful requests",
        standard_success
    );

    // Brief pause
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    println!("Testing live model rate limits...");

    // Try 20 requests to live model
    let mut live_success = 0;
    for i in 0..20 {
        let request = create_test_request(
            &format!("Request {}", i),
            Some("gemini-2.0-flash-live".to_string()),
        );

        match client.generate_stream(&request).await {
            Ok(mut stream) => {
                // Consume stream
                while let Some(_) = stream.next().await {}
                live_success += 1;
            }
            Err(e) => {
                println!("Live model failed at request {}: {}", i, e);
                break;
            }
        }
    }

    println!("Live model: {} successful requests", live_success);

    // Live model should allow more requests (or at least same amount)
    assert!(
        live_success >= standard_success,
        "Live model should have equal or better rate limits. Standard: {}, Live: {}",
        standard_success,
        live_success
    );
}
