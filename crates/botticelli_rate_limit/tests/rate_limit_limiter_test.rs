//! Tests for rate limiter implementation.

mod helpers;

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
async fn test_acquire_releases_on_drop() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing acquire releases on drop");

    let tier = create_test_tier(Some(100), Some(10000), None, Some(1))?;
    let limiter = Arc::new(RateLimiter::new(tier));

    tracing::debug!("Acquiring first guard");
    let guard1 = limiter.acquire(1).await?;

    tracing::debug!("Testing second acquire blocks");
    assert!(limiter.try_acquire(1).is_none());

    tracing::debug!("Dropping first guard");
    drop(guard1);

    tracing::debug!("Testing acquire succeeds after drop");
    assert!(
        limiter.try_acquire(1).is_some(),
        "Should acquire after drop"
    );

    tracing::info!("Acquire releases on drop test passed");
    Ok(())
}

#[tokio::test]
async fn test_rpm_limiting() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing RPM limiting");

    let tier = create_test_tier(Some(2), None, None, Some(10))?;
    let limiter = RateLimiter::new(tier);

    tracing::debug!("First two requests should succeed");
    assert!(limiter.try_acquire(1).is_some(), "First request");
    assert!(limiter.try_acquire(1).is_some(), "Second request");

    tracing::debug!("Third request should be rate limited");
    assert!(
        limiter.try_acquire(1).is_none(),
        "Third request should be rate limited"
    );

    tracing::info!("RPM limiting test passed");
    Ok(())
}

#[tokio::test]
async fn test_unlimited_tier() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing unlimited tier");

    let tier = create_test_tier(None, None, None, None)?;
    let limiter = RateLimiter::new(tier);

    tracing::debug!("Making 100 requests");
    for i in 0..100 {
        assert!(
            limiter.try_acquire(1).is_some(),
            "Request {} should not be limited",
            i
        );
    }

    tracing::info!("Unlimited tier test passed");
    Ok(())
}

#[tokio::test]
async fn test_tpm_limiting() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing TPM limiting");

    let tier = create_test_tier(None, Some(10), None, Some(10))?;
    let limiter = RateLimiter::new(tier);

    tracing::debug!("First request with 5 tokens");
    assert!(limiter.try_acquire(5).is_some(), "First request");

    tracing::debug!("Second request with 5 tokens");
    assert!(limiter.try_acquire(5).is_some(), "Second request");

    tracing::debug!("Third request should exceed TPM");
    assert!(limiter.try_acquire(1).is_none(), "Should be TPM limited");

    tracing::info!("TPM limiting test passed");
    Ok(())
}
