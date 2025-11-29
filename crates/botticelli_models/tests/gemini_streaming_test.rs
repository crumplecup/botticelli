#![cfg(feature = "gemini")]
mod test_utils;

// Tests for Gemini streaming support.
//
// These tests verify that streaming works with both standard and live models.
// Live models (e.g., gemini-2.0-flash-live) offer better rate limits on the free tier,
// which is the primary motivation for implementing streaming.

use botticelli_core::Output;
use botticelli_interface::{BotticelliDriver, Streaming};
use botticelli_models::GeminiClient;
use futures_util::StreamExt;
use test_utils::create_test_request;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_streaming_basic() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    let request = create_test_request("Say 'ok'", None, Some(10));

    let mut stream = client.generate_stream(&request).await?;

    let mut chunks = Vec::new();
    let mut saw_final = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
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
            Output::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    println!("Streaming result: {}", full_text);

    // Should have generated some text
    assert!(!full_text.is_empty(), "Response should contain text");

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_streaming_with_standard_model() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // Explicitly use standard flash model
    let request = create_test_request("Say 'ok'", Some("gemini-2.0-flash".to_string()), Some(10));

    let mut stream = client.generate_stream(&request).await?;

    let mut chunks = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        chunks.push(chunk.clone());

        if chunk.is_final {
            break;
        }
    }

    assert!(!chunks.is_empty(), "Should receive chunks");

    let full_text: String = chunks
        .iter()
        .filter_map(|c| match &c.content {
            Output::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    println!("Standard model result: {}", full_text);
    assert!(!full_text.is_empty(), "Should have generated text");

    Ok(())
}

#[tokio::test]
#[ignore] // Model doesn't exist - confirmed via API ListModels query (see GEMINI_STREAMING.md)
async fn test_streaming_with_live_model() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    // CRITICAL TEST: Use live model for better rate limits
    let request = create_test_request(
        "Say 'ok'",
        Some("gemini-2.5-flash-live".to_string()),
        Some(10),
    );

    let mut stream = client.generate_stream(&request).await?;

    let mut chunks = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        chunks.push(chunk.clone());

        if chunk.is_final {
            break;
        }
    }

    assert!(!chunks.is_empty(), "Live model should stream chunks");

    let full_text: String = chunks
        .iter()
        .filter_map(|c| match &c.content {
            Output::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    println!("Live model result: {}", full_text);
    assert!(!full_text.is_empty(), "Live model should generate text");

    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_streaming_finish_reasons() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    let request = create_test_request("Say 'ok'", None, Some(10));

    let mut stream = client.generate_stream(&request).await?;

    let mut final_chunk = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

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
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)] // Requires GEMINI_API_KEY
async fn test_streaming_vs_non_streaming_consistency() -> botticelli_error::BotticelliResult<()> {
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    let request = create_test_request("Say 'ok'", None, Some(10));

    // Get streaming response
    let mut stream = client.generate_stream(&request).await?;

    let mut streaming_text = String::new();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        if let Output::Text(t) = &chunk.content {
            streaming_text.push_str(&t);
        }
        if chunk.is_final {
            break;
        }
    }

    // Get non-streaming response
    let response = client.generate(&request).await?;
    let non_streaming_text = response
        .outputs
        .iter()
        .filter_map(|o| match o {
            Output::Text(t) => Some(t.clone()),
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

    Ok(())
}

#[tokio::test]
#[ignore] // OBSOLETE: 'Live' models don't exist - original premise was incorrect (see GEMINI_STREAMING.md)
async fn test_rate_limit_comparison() -> botticelli_error::BotticelliResult<()> {
    // DEPRECATED: This test compared rate limits between standard and "live" models.
    // Investigation on 2025-01-17 confirmed that no "live" models exist in the Gemini API.
    // The model "gemini-2.0-flash-live" returns 404 NOT_FOUND from Google's servers.
    //
    // Original hypothesis: Live models have better free tier rate limits
    // Reality: Model doesn't exist, hypothesis cannot be tested
    //
    // See GEMINI_STREAMING.md for complete investigation findings.

    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;

    println!("Testing standard model rate limits (3 requests)...");

    // Try 3 requests to standard model
    let mut standard_success = 0;
    for i in 0..3 {
        let request = create_test_request("ok", Some("gemini-2.0-flash".to_string()), Some(10));

        match client.generate_stream(&request).await {
            Ok(mut stream) => {
                // Consume stream
                while stream.next().await.is_some() {}
                standard_success += 1;
            }
            Err(e) => {
                println!("Standard model failed at request {}: {}", i, e);
                break;
            }
        }
    }

    println!("Standard model: {} successful requests", standard_success);

    // Brief pause
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("Testing live model rate limits (3 requests)...");

    // Try 3 requests to live model
    let mut live_success = 0;
    for i in 0..3 {
        let request =
            create_test_request("ok", Some("gemini-2.0-flash-live".to_string()), Some(10));

        match client.generate_stream(&request).await {
            Ok(mut stream) => {
                // Consume stream
                while stream.next().await.is_some() {}
                live_success += 1;
            }
            Err(e) => {
                println!("Live model failed at request {}: {}", i, e);
                break;
            }
        }
    }

    println!("Live model: {} successful requests", live_success);

    // Live model should allow equal or more requests
    assert!(
        live_success >= standard_success,
        "Live model should have equal or better rate limits. Standard: {}, Live: {}",
        standard_success,
        live_success
    );

    Ok(())
}
