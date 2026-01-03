//! Tests for rate limiter implementation.

mod common;

use botticelli_error::BotticelliResult;
use botticelli_rate_limit::{RateLimiter, TierConfig, TierConfigBuilder};
use std::sync::Arc;

fn create_test_tier(
    rpm: Option<u32>,
    tpm: Option<u64>,
    rpd: Option<u32>,
    max_concurrent: Option<u32>,
) -> BotticelliResult<TierConfig> {
    let mut builder = TierConfigBuilder::default();
    builder.name("Test");

    if let Some(rpm_val) = rpm {
        builder.rpm(rpm_val);
    }
    if let Some(tpm_val) = tpm {
        builder.tpm(tpm_val);
    }
    if let Some(rpd_val) = rpd {
        builder.rpd(rpd_val);
    }
    if let Some(mc_val) = max_concurrent {
        builder.max_concurrent(mc_val);
    }

    builder.build().map_err(|e| e.to_string().into())
}

#[tokio::test]
async fn test_acquire_releases_on_drop() -> BotticelliResult<()> {
    common::init_tracing();
    
    let tier = create_test_tier(Some(100), Some(10000), None, Some(1))?;
    let limiter = Arc::new(RateLimiter::new(tier));

    // First acquire should succeed
    let guard1 = limiter.acquire(1).await?;

    // Second acquire should block (max_concurrent = 1)
    // We'll test this by trying try_acquire
    assert!(limiter.try_acquire(1).is_none());

    // Drop first guard
    drop(guard1);

    // Now try_acquire should succeed
    assert!(
        limiter.try_acquire(1).is_some(),
        "Should acquire after drop"
    );
    Ok(())
}

#[tokio::test]
async fn test_rpm_limiting() -> BotticelliResult<()> {
    common::init_tracing();
    // Very low RPM for testing
    let tier = create_test_tier(Some(2), None, None, Some(10))?;
    let limiter = RateLimiter::new(tier);

    // First two requests should succeed immediately
    assert!(limiter.try_acquire(1).is_some(), "First request");
    assert!(limiter.try_acquire(1).is_some(), "Second request");

    // Third request should fail (rate limited)
    assert!(
        limiter.try_acquire(1).is_none(),
        "Third request should be rate limited"
    );
    Ok(())
}

#[tokio::test]
async fn test_unlimited_tier() -> BotticelliResult<()> {
    common::init_tracing();
    // No limits
    let tier = create_test_tier(None, None, None, None)?;
    let limiter = RateLimiter::new(tier);

    // Should be able to make many requests
    for _ in 0..100 {
        assert!(limiter.try_acquire(1).is_some(), "Should not be limited");
    }
    Ok(())
}

#[tokio::test]
async fn test_tpm_limiting() -> BotticelliResult<()> {
    common::init_tracing();
    // Very low TPM for testing
    let tier = create_test_tier(None, Some(10), None, Some(10))?;
    let limiter = RateLimiter::new(tier);

    // First request with 5 tokens should succeed
    assert!(limiter.try_acquire(5).is_some(), "First request");

    // Second request with 5 tokens should succeed
    assert!(limiter.try_acquire(5).is_some(), "Second request");

    // Third request should fail (would exceed TPM)
    assert!(limiter.try_acquire(1).is_none(), "Should be TPM limited");
    Ok(())
}
