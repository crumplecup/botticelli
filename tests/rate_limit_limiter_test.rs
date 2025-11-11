//! Tests for rate limiter implementation.

use boticelli::{RateLimiter, Tier, TierConfig};
use std::sync::Arc;

fn create_test_tier(
    rpm: Option<u32>,
    tpm: Option<u64>,
    rpd: Option<u32>,
    max_concurrent: Option<u32>,
) -> Box<dyn Tier> {
    Box::new(TierConfig {
        name: "Test".to_string(),
        rpm,
        tpm,
        rpd,
        max_concurrent,
        daily_quota_usd: None,
        cost_per_million_input_tokens: None,
        cost_per_million_output_tokens: None,
    })
}

#[tokio::test]
async fn test_acquire_releases_on_drop() {
    let tier = create_test_tier(Some(100), Some(10000), None, Some(1));
    let limiter = Arc::new(RateLimiter::new(tier));

    // First acquire should succeed
    let guard1 = limiter.acquire(1).await;

    // Second acquire should block (max_concurrent = 1)
    // We'll test this by trying try_acquire
    assert!(limiter.try_acquire(1).is_none());

    // Drop first guard
    drop(guard1);

    // Now try_acquire should succeed
    let _guard2 = limiter.try_acquire(1).expect("Should acquire after drop");
}

#[tokio::test]
async fn test_rpm_limiting() {
    // Very low RPM for testing
    let tier = create_test_tier(Some(2), None, None, Some(10));
    let limiter = RateLimiter::new(tier);

    // First two requests should succeed immediately
    let _guard1 = limiter.try_acquire(1).expect("First request");
    let _guard2 = limiter.try_acquire(1).expect("Second request");

    // Third request should fail (rate limited)
    assert!(
        limiter.try_acquire(1).is_none(),
        "Third request should be rate limited"
    );
}

#[tokio::test]
async fn test_unlimited_tier() {
    // No limits
    let tier = create_test_tier(None, None, None, None);
    let limiter = RateLimiter::new(tier);

    // Should be able to make many requests
    for _ in 0..100 {
        let _guard = limiter.try_acquire(1).expect("Should not be limited");
    }
}

#[tokio::test]
async fn test_tpm_limiting() {
    // Very low TPM for testing
    let tier = create_test_tier(None, Some(10), None, Some(10));
    let limiter = RateLimiter::new(tier);

    // First request with 5 tokens should succeed
    let _guard1 = limiter.try_acquire(5).expect("First request");

    // Second request with 5 tokens should succeed
    let _guard2 = limiter.try_acquire(5).expect("Second request");

    // Third request should fail (would exceed TPM)
    assert!(limiter.try_acquire(1).is_none(), "Should be TPM limited");
}
