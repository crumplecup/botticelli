//! Integration tests for rate limiting.
//!
//! These tests validate rate limiting behavior with minimal API usage.
//! Most tests use mock drivers to avoid API calls.
//!
//! To run API tests (requires GEMINI_API_KEY):
//!   BOTICELLI_RUN_API_TESTS=1 cargo test --test rate_limit_integration_test
//!
//! API tests are minimal (1 request, ~10 tokens) to conserve quota.

use boticelli::{RateLimiter, Tier, TierConfig};

#[cfg(feature = "gemini")]
use boticelli::BoticelliDriver;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "gemini")]
use boticelli::{GenerateRequest, Input, Message, Role};

/// Check if API tests should run
#[cfg(feature = "gemini")]
fn should_run_api_tests() -> bool {
    std::env::var("BOTICELLI_RUN_API_TESTS").is_ok()
}

/// Skip test if API tests are disabled
#[cfg(feature = "gemini")]
macro_rules! skip_unless_api_tests_enabled {
    () => {
        if !should_run_api_tests() {
            println!("Skipping API test (set BOTICELLI_RUN_API_TESTS=1 to enable)");
            return;
        }
    };
}

#[tokio::test]
async fn test_rate_limiter_blocks_on_rpm_limit() {
    // Test rate limiting with very low limits to see blocking quickly
    // This test doesn't hit any API - it's pure rate limiter logic

    let tier = TierConfig {
        name: "Test Tier".to_string(),
        rpm: Some(2), // Only 2 requests per minute
        tpm: None,
        rpd: None,
        max_concurrent: Some(10),
        daily_quota_usd: None,
        cost_per_million_input_tokens: None,
        cost_per_million_output_tokens: None,
    };

    let limiter = RateLimiter::new(tier);

    // First two requests should succeed immediately
    let start = Instant::now();
    let _guard1 = limiter.try_acquire(100).expect("First request should succeed");
    let _guard2 = limiter.try_acquire(100).expect("Second request should succeed");
    let immediate_duration = start.elapsed();

    // Should be very fast (< 10ms)
    assert!(
        immediate_duration < Duration::from_millis(10),
        "First two requests should be immediate"
    );

    // Third request should fail (rate limited)
    assert!(
        limiter.try_acquire(100).is_none(),
        "Third request should be rate limited"
    );

    // Using acquire() would wait, but we'll test that behavior is correct
    // by verifying try_acquire fails as expected
}

#[tokio::test]
async fn test_rate_limiter_releases_concurrent_slots() {
    // Test that concurrent slots are properly released via RAII

    let tier = TierConfig {
        name: "Test Tier".to_string(),
        rpm: Some(100),
        tpm: None,
        rpd: None,
        max_concurrent: Some(1), // Only 1 concurrent request
        daily_quota_usd: None,
        cost_per_million_input_tokens: None,
        cost_per_million_output_tokens: None,
    };

    let limiter = Arc::new(RateLimiter::new(tier));

    // First request acquires the slot
    {
        let _guard1 = limiter.acquire(100).await;

        // Second request should fail (slot held)
        assert!(
            limiter.try_acquire(100).is_none(),
            "Second request should fail while first holds slot"
        );

        // Guard drops here
    }

    // After first guard drops, slot should be available
    let _guard2 = limiter
        .try_acquire(100)
        .expect("Should acquire after first guard drops");
}

#[cfg(feature = "gemini")]
#[tokio::test]
async fn test_gemini_client_without_rate_limiting() {
    use boticelli::GeminiClient;

    skip_unless_api_tests_enabled!();

    // Minimal API test: single request with ~10 tokens
    // This verifies GeminiClient works without rate limiting

    let client = GeminiClient::new().expect("Failed to create client");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'hi'".to_string())], // ~2 tokens input
        }],
        temperature: Some(0.0), // Deterministic for testing
        max_tokens: Some(5),    // Minimal output tokens
        model: None,
    };

    let start = Instant::now();
    let response = client
        .generate(&request)
        .await
        .expect("API call failed");
    let duration = start.elapsed();

    // Verify we got a response
    assert!(!response.outputs.is_empty(), "Should have output");

    println!(
        "✓ API call succeeded in {:.2}s (~7 tokens used)",
        duration.as_secs_f64()
    );
}

#[cfg(feature = "gemini")]
#[tokio::test]
async fn test_gemini_client_with_rate_limiting() {
    use boticelli::{GeminiClient, GeminiTier};

    skip_unless_api_tests_enabled!();

    // Minimal API test: single request with rate limiting enabled
    // Uses free tier limits (10 RPM, 250k TPM)

    let tier = Box::new(GeminiTier::Free);
    let client = GeminiClient::new_with_tier(Some(tier)).expect("Failed to create client");

    let request = GenerateRequest {
        messages: vec![Message {
            role: Role::User,
            content: vec![Input::Text("Say 'ok'".to_string())], // ~2 tokens input
        }],
        temperature: Some(0.0),
        max_tokens: Some(5), // Minimal output
        model: None,
    };

    let start = Instant::now();
    let response = client
        .generate(&request)
        .await
        .expect("API call with rate limiting failed");
    let duration = start.elapsed();

    // Verify response
    assert!(!response.outputs.is_empty(), "Should have output");

    println!(
        "✓ Rate-limited API call succeeded in {:.2}s (~7 tokens used)",
        duration.as_secs_f64()
    );
}

#[tokio::test]
async fn test_rate_limiter_with_multiple_limits() {
    // Test that multiple limit types work together (no API calls)

    let tier = TierConfig {
        name: "Multi-Limit Test".to_string(),
        rpm: Some(5),      // 5 requests per minute
        tpm: Some(100),    // 100 tokens per minute
        rpd: Some(20),     // 20 requests per day
        max_concurrent: Some(2), // 2 concurrent
        daily_quota_usd: None,
        cost_per_million_input_tokens: None,
        cost_per_million_output_tokens: None,
    };

    let limiter = RateLimiter::new(tier);

    // Should be able to make requests within all limits
    let _g1 = limiter.try_acquire(10).expect("First request (10 tokens)");
    let _g2 = limiter.try_acquire(10).expect("Second request (10 tokens)");

    // Third request would exceed concurrent limit (only 2 concurrent allowed)
    // Note: try_acquire checks all limits synchronously, so RPM/TPM pass but concurrent fails
    assert!(
        limiter.try_acquire(10).is_none(),
        "Third request should fail (concurrent limit)"
    );
}

#[test]
fn test_tier_trait_from_config() {
    // Verify TierConfig implements Tier trait correctly (no API)

    let config = TierConfig {
        name: "Test".to_string(),
        rpm: Some(10),
        tpm: Some(250_000),
        rpd: Some(250),
        max_concurrent: Some(1),
        daily_quota_usd: None,
        cost_per_million_input_tokens: Some(0.0),
        cost_per_million_output_tokens: Some(0.0),
    };

    // Test Tier trait methods
    assert_eq!(config.rpm(), Some(10));
    assert_eq!(config.tpm(), Some(250_000));
    assert_eq!(config.rpd(), Some(250));
    assert_eq!(config.max_concurrent(), Some(1));
    assert_eq!(config.name(), "Test");
    assert_eq!(config.cost_per_million_input_tokens(), Some(0.0));
    assert_eq!(config.cost_per_million_output_tokens(), Some(0.0));
}

#[cfg(feature = "gemini")]
#[test]
fn test_gemini_tier_enum() {
    use boticelli::GeminiTier;

    // Verify GeminiTier enum has correct values (no API)

    let free = GeminiTier::Free;
    assert_eq!(free.name(), "Free");
    assert_eq!(free.rpm(), Some(10));
    assert_eq!(free.tpm(), Some(250_000));
    assert_eq!(free.rpd(), Some(250));
    assert_eq!(free.max_concurrent(), Some(1));

    let payg = GeminiTier::PayAsYouGo;
    assert_eq!(payg.name(), "Pay-as-you-go");
    assert_eq!(payg.rpm(), Some(360));
    assert_eq!(payg.tpm(), Some(4_000_000));
    assert_eq!(payg.rpd(), None); // No daily limit
    assert_eq!(payg.max_concurrent(), Some(1));
}
