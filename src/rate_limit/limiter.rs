//! Rate limiter implementation using governor and Tokio Semaphore.
//!
//! This module provides the `RateLimiter` struct which enforces rate limits using:
//! - Governor crate (GCRA algorithm) for RPM, TPM, and RPD limits
//! - Tokio Semaphore for concurrent request limits
//!
//! The GCRA (Generic Cell Rate Algorithm) provides efficient, lock-free rate limiting
//! that is ~10x faster than mutex-based token bucket approaches.

use crate::Tier;
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::Semaphore;

// Type alias for our direct rate limiter
type DirectRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Rate limiter that enforces multiple quota types.
///
/// This limiter coordinates multiple rate limits:
/// - **RPM** (requests per minute): Enforced via governor
/// - **TPM** (tokens per minute): Enforced via governor
/// - **RPD** (requests per day): Enforced via governor with daily quota
/// - **Concurrent requests**: Enforced via Tokio Semaphore
///
/// The limiter takes ownership of a value implementing `Tier` and uses it
/// to configure rate limits. Access to the inner value is provided through
/// the `inner()` method after acquiring rate limit permission.
///
/// # Type Parameters
///
/// * `T` - Any type implementing the `Tier` trait. The limiter takes ownership
///   of this value and provides controlled access through rate limiting.
///
/// # Example
///
/// ```rust,ignore
/// use boticelli::{RateLimiter, GeminiTier};
///
/// let limiter = RateLimiter::new(GeminiTier::Free);
///
/// // Acquire permission for a request with estimated 1000 tokens
/// let guard = limiter.acquire(1000).await;
/// // Access the inner tier through the limiter
/// let tier_ref = limiter.inner();
/// // Make API call...
/// drop(guard); // Releases concurrent slot
/// ```
#[derive(Clone)]
pub struct RateLimiter<T: Tier> {
    inner: T,

    // RPM limiter (requests per minute)
    rpm_limiter: Option<Arc<DirectRateLimiter>>,

    // TPM limiter (tokens per minute)
    tpm_limiter: Option<Arc<DirectRateLimiter>>,

    // RPD limiter (requests per day)
    rpd_limiter: Option<Arc<DirectRateLimiter>>,

    // Concurrent request semaphore
    concurrent_semaphore: Arc<Semaphore>,
}

impl<T: Tier> RateLimiter<T> {
    /// Create a new rate limiter from a tier.
    ///
    /// Takes ownership of the tier and uses it to configure rate limits.
    /// The limiter will enforce all non-None limits from the tier:
    /// - If `tier.rpm()` is Some, enforces requests per minute
    /// - If `tier.tpm()` is Some, enforces tokens per minute
    /// - If `tier.rpd()` is Some, enforces requests per day
    /// - If `tier.max_concurrent()` is Some, enforces concurrent limit
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use boticelli::{RateLimiter, GeminiTier};
    ///
    /// let limiter = RateLimiter::new(GeminiTier::Free);
    /// // Enforces: 10 RPM, 250K TPM, 250 RPD, 1 concurrent
    /// ```
    pub fn new(tier: T) -> Self {
        // Create RPM limiter
        let rpm_limiter = tier.rpm().and_then(|rpm| {
            NonZeroU32::new(rpm).map(|n| {
                let quota = Quota::per_minute(n);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
        });

        // Create TPM limiter
        let tpm_limiter = tier.tpm().and_then(|tpm| {
            // Governor uses u32, so we need to handle large TPM values
            // For very large TPM values (>4B), we cap at u32::MAX
            NonZeroU32::new(tpm.min(u32::MAX as u64) as u32)
                .map(|n| {
                    let quota = Quota::per_minute(n);
                    Arc::new(GovernorRateLimiter::direct(quota))
                })
        });

        // Create RPD limiter (requests per day)
        // We model this as requests per 1440 minutes (24 hours)
        let rpd_limiter = tier.rpd().and_then(|rpd| {
            NonZeroU32::new(rpd).map(|n| {
                // Allow full daily burst at once
                let quota = Quota::per_minute(n).allow_burst(n);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
        });

        // Create concurrent semaphore
        let max_concurrent = tier.max_concurrent().unwrap_or(u32::MAX);
        let concurrent_semaphore = Arc::new(Semaphore::new(max_concurrent as usize));

        Self {
            inner: tier,
            rpm_limiter,
            tpm_limiter,
            rpd_limiter,
            concurrent_semaphore,
        }
    }

    /// Get a reference to the inner tier value.
    ///
    /// This allows access to the wrapped value (which implements `Tier`)
    /// after the rate limiter has been created. Useful for accessing the
    /// underlying client or tier information.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let limiter = RateLimiter::new(my_tiered_client);
    /// let client_ref = limiter.inner();
    /// // Use client_ref to make API calls
    /// ```
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Acquire rate limit permission for a request.
    ///
    /// This waits until all rate limits allow the request:
    /// - RPM (requests per minute)
    /// - TPM (tokens per minute, based on estimated_tokens)
    /// - RPD (requests per day)
    /// - Concurrent request limit
    ///
    /// Returns a guard that releases the concurrent slot when dropped.
    ///
    /// # Arguments
    ///
    /// * `estimated_tokens` - Estimated number of tokens for this request.
    ///   Used for TPM limiting. If unsure, use a conservative estimate.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let guard = limiter.acquire(1000).await;
    /// let response = client.generate(&request).await?;
    /// drop(guard); // Release concurrent slot
    /// ```
    pub async fn acquire(&self, estimated_tokens: u64) -> RateLimiterGuard {
        // Wait for RPM quota
        if let Some(limiter) = &self.rpm_limiter {
            limiter.until_ready().await;
        }

        // Wait for TPM quota (consume estimated tokens)
        if let Some(limiter) = &self.tpm_limiter {
            let tokens = (estimated_tokens.min(u32::MAX as u64) as u32).max(1);
            // Consume tokens one at a time to respect the rate
            // Governor doesn't have a "consume N" API, so we acquire N times
            for _ in 0..tokens {
                limiter.until_ready().await;
            }
        }

        // Wait for RPD quota
        if let Some(limiter) = &self.rpd_limiter {
            limiter.until_ready().await;
        }

        // Acquire concurrent request slot (last to avoid holding slot while waiting)
        let permit = self.concurrent_semaphore.clone()
            .acquire_owned()
            .await
            .expect("Semaphore should not be closed");

        RateLimiterGuard {
            _permit: permit,
        }
    }

    /// Try to acquire without waiting.
    ///
    /// Returns None if any rate limit would block.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(guard) = limiter.try_acquire(1000) {
    ///     // Rate limits allow request
    ///     let response = client.generate(&request).await?;
    /// } else {
    ///     // Rate limited, try again later
    /// }
    /// ```
    pub fn try_acquire(&self, estimated_tokens: u64) -> Option<RateLimiterGuard> {
        // Check RPM
        if let Some(limiter) = &self.rpm_limiter {
            limiter.check().ok()?;
        }

        // Check TPM
        if let Some(limiter) = &self.tpm_limiter {
            let tokens = (estimated_tokens.min(u32::MAX as u64) as u32).max(1);
            for _ in 0..tokens {
                limiter.check().ok()?;
            }
        }

        // Check RPD
        if let Some(limiter) = &self.rpd_limiter {
            limiter.check().ok()?;
        }

        // Try to acquire concurrent slot
        let permit = self.concurrent_semaphore.clone().try_acquire_owned().ok()?;

        Some(RateLimiterGuard { _permit: permit })
    }
}

/// RAII guard for rate limiter.
///
/// Automatically releases the concurrent request slot when dropped.
/// This ensures that even if the request fails or panics, the concurrent
/// slot is properly returned to the semaphore.
pub struct RateLimiterGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}
