//! Rate limiting using token bucket algorithm.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

/// Rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct RateLimit {
    /// Maximum tokens (requests) allowed
    max_tokens: u32,
    /// Time window in seconds
    window_secs: u64,
    /// Burst allowance (extra tokens for spikes)
    burst: u32,
}

impl RateLimit {
    /// Create a new rate limit.
    #[tracing::instrument(fields(max_tokens, window_secs, burst))]
    pub fn new(max_tokens: u32, window_secs: u64, burst: u32) -> Self {
        tracing::debug!("Creating rate limit");
        Self {
            max_tokens,
            window_secs,
            burst,
        }
    }

    /// Create a rate limit with no burst.
    #[tracing::instrument(fields(max_tokens, window_secs))]
    pub fn strict(max_tokens: u32, window_secs: u64) -> Self {
        Self::new(max_tokens, window_secs, 0)
    }
}

/// Rate limit exceeded error details.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct RateLimitExceeded {
    /// Operation that exceeded limit
    operation: String,
    /// Current rate limit
    limit: RateLimit,
    /// Time until tokens are available
    retry_after: Duration,
}

/// Token bucket for a specific operation.
#[derive(Debug)]
struct TokenBucket {
    /// Rate limit configuration
    limit: RateLimit,
    /// Current available tokens
    tokens: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket.
    #[tracing::instrument(skip(limit), fields(max_tokens = limit.max_tokens, burst = limit.burst))]
    fn new(limit: RateLimit) -> Self {
        let max_tokens = (limit.max_tokens + limit.burst) as f64;
        tracing::debug!(initial_tokens = max_tokens, "Token bucket created");
        Self {
            limit,
            tokens: max_tokens,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time.
    #[tracing::instrument(skip(self), fields(current_tokens = self.tokens))]
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let elapsed_secs = elapsed.as_secs_f64();

        // Calculate tokens to add based on elapsed time
        let refill_rate = self.limit.max_tokens as f64 / self.limit.window_secs as f64;
        let tokens_to_add = elapsed_secs * refill_rate;

        let max_tokens = (self.limit.max_tokens + self.limit.burst) as f64;
        self.tokens = (self.tokens + tokens_to_add).min(max_tokens);
        self.last_refill = now;
        tracing::debug!(tokens_after_refill = self.tokens, "Tokens refilled");
    }

    /// Try to consume a token. Returns Ok if successful, Err with retry duration if not.
    #[tracing::instrument(skip(self), fields(available_tokens = self.tokens))]
    fn try_consume(&mut self) -> Result<(), Duration> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            tracing::debug!(remaining_tokens = self.tokens, "Token consumed");
            Ok(())
        } else {
            // Calculate time until a token is available
            let refill_rate = self.limit.max_tokens as f64 / self.limit.window_secs as f64;
            let tokens_needed = 1.0 - self.tokens;
            let secs_to_wait = tokens_needed / refill_rate;
            tracing::debug!(secs_to_wait, "Rate limit exceeded");
            Err(Duration::from_secs_f64(secs_to_wait))
        }
    }

    /// Get current token count.
    fn available_tokens(&mut self) -> u32 {
        self.refill();
        self.tokens.floor() as u32
    }
}

/// Rate limiter for tracking multiple operations.
///
/// Note: RateLimiter cannot be cloned because it contains stateful time-based
/// TokenBuckets. Cloning would split rate limit budgets and break rate limiting.
#[derive(Debug)]
pub struct RateLimiter {
    /// Rate limits by operation name
    limits: HashMap<String, RateLimit>,
    /// Token buckets by operation name
    buckets: HashMap<String, TokenBucket>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    #[tracing::instrument]
    pub fn new() -> Self {
        tracing::debug!("Creating rate limiter");
        Self {
            limits: HashMap::new(),
            buckets: HashMap::new(),
        }
    }

    /// Add a rate limit for an operation.
    #[tracing::instrument(skip(self, limit), fields(operation, max_tokens = limit.max_tokens))]
    pub fn add_limit(&mut self, operation: impl Into<String>, limit: RateLimit) {
        let operation = operation.into();
        tracing::debug!("Adding rate limit for operation");
        self.buckets
            .insert(operation.clone(), TokenBucket::new(limit.clone()));
        self.limits.insert(operation, limit);
    }

    /// Check if an operation can be executed.
    #[instrument(skip(self), fields(operation))]
    pub fn check(&mut self, operation: &str) -> SecurityResult<()> {
        debug!("Checking rate limit");

        // If no limit configured, allow
        let Some(bucket) = self.buckets.get_mut(operation) else {
            debug!("No rate limit configured for operation");
            return Ok(());
        };

        match bucket.try_consume() {
            Ok(()) => {
                let available = bucket.available_tokens();
                debug!(tokens_remaining = available, "Rate limit check passed");
                Ok(())
            }
            Err(retry_after) => {
                debug!(
                    retry_after_secs = retry_after.as_secs(),
                    "Rate limit exceeded"
                );
                let limit = self.limits.get(operation).unwrap().clone();
                Err(SecurityError::new(SecurityErrorKind::RateLimitExceeded {
                    operation: operation.to_string(),
                    reason: format!(
                        "Rate limit exceeded, retry after {} seconds",
                        retry_after.as_secs()
                    ),
                    limit: limit.max_tokens,
                    window_secs: limit.window_secs,
                }))
            }
        }
    }

    /// Get available tokens for an operation.
    #[tracing::instrument(skip(self), fields(operation))]
    pub fn available_tokens(&mut self, operation: &str) -> Option<u32> {
        self.buckets
            .get_mut(operation)
            .map(|b| b.available_tokens())
    }

    /// Get rate limit configuration for an operation.
    #[tracing::instrument(skip(self), fields(operation))]
    pub fn get_limit(&self, operation: &str) -> Option<&RateLimit> {
        self.limits.get(operation)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
