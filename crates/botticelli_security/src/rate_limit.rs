//! Rate limiting using token bucket algorithm.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

/// Rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum tokens (requests) allowed
    pub max_tokens: u32,
    /// Time window in seconds
    pub window_secs: u64,
    /// Burst allowance (extra tokens for spikes)
    pub burst: u32,
}

impl RateLimit {
    /// Create a new rate limit.
    pub fn new(max_tokens: u32, window_secs: u64, burst: u32) -> Self {
        Self {
            max_tokens,
            window_secs,
            burst,
        }
    }

    /// Create a rate limit with no burst.
    pub fn strict(max_tokens: u32, window_secs: u64) -> Self {
        Self::new(max_tokens, window_secs, 0)
    }
}

/// Rate limit exceeded error details.
#[derive(Debug, Clone)]
pub struct RateLimitExceeded {
    /// Operation that exceeded limit
    pub operation: String,
    /// Current rate limit
    pub limit: RateLimit,
    /// Time until tokens are available
    pub retry_after: Duration,
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
    fn new(limit: RateLimit) -> Self {
        let max_tokens = (limit.max_tokens + limit.burst) as f64;
        Self {
            limit,
            tokens: max_tokens,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time.
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
    }

    /// Try to consume a token. Returns Ok if successful, Err with retry duration if not.
    fn try_consume(&mut self) -> Result<(), Duration> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate time until a token is available
            let refill_rate = self.limit.max_tokens as f64 / self.limit.window_secs as f64;
            let tokens_needed = 1.0 - self.tokens;
            let secs_to_wait = tokens_needed / refill_rate;
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
pub struct RateLimiter {
    /// Rate limits by operation name
    limits: HashMap<String, RateLimit>,
    /// Token buckets by operation name
    buckets: HashMap<String, TokenBucket>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
            buckets: HashMap::new(),
        }
    }

    /// Add a rate limit for an operation.
    pub fn add_limit(&mut self, operation: impl Into<String>, limit: RateLimit) {
        let operation = operation.into();
        self.buckets.insert(operation.clone(), TokenBucket::new(limit.clone()));
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
                debug!(retry_after_secs = retry_after.as_secs(), "Rate limit exceeded");
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
    pub fn available_tokens(&mut self, operation: &str) -> Option<u32> {
        self.buckets.get_mut(operation).map(|b| b.available_tokens())
    }

    /// Get rate limit configuration for an operation.
    pub fn get_limit(&self, operation: &str) -> Option<&RateLimit> {
        self.limits.get(operation)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_rate_limit_allows_within_limit() {
        let mut limiter = RateLimiter::new();
        limiter.add_limit("test", RateLimit::strict(2, 1));

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
    }

    #[test]
    fn test_rate_limit_blocks_over_limit() {
        let mut limiter = RateLimiter::new();
        limiter.add_limit("test", RateLimit::strict(2, 1));

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_err()); // Should be blocked
    }

    #[test]
    fn test_rate_limit_refills() {
        let mut limiter = RateLimiter::new();
        limiter.add_limit("test", RateLimit::strict(1, 1)); // 1 per second

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_err()); // Blocked

        // Wait for refill
        thread::sleep(Duration::from_millis(1100));

        assert!(limiter.check("test").is_ok()); // Should work after refill
    }

    #[test]
    fn test_burst_allowance() {
        let mut limiter = RateLimiter::new();
        limiter.add_limit("test", RateLimit::new(2, 1, 1)); // 2/sec + 1 burst

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok()); // Burst token
        assert!(limiter.check("test").is_err()); // Should be blocked
    }

    #[test]
    fn test_no_limit_configured() {
        let mut limiter = RateLimiter::new();
        // No limit added for "test"

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok()); // Always allowed
    }

    #[test]
    fn test_available_tokens() {
        let mut limiter = RateLimiter::new();
        limiter.add_limit("test", RateLimit::strict(3, 1));

        assert_eq!(limiter.available_tokens("test"), Some(3));
        limiter.check("test").unwrap();
        assert_eq!(limiter.available_tokens("test"), Some(2));
        limiter.check("test").unwrap();
        assert_eq!(limiter.available_tokens("test"), Some(1));
    }
}
