//! Rate limiting and usage tier management.
//!
//! This module provides rate limiting functionality to comply with LLM API quotas.
//! It supports multiple providers with different tier structures and automatically
//! detects limits from API response headers when possible.

mod config;
mod detector;
mod limiter;
mod tier;
mod tiers;

pub use config::{BoticelliConfig, ProviderConfig, TierConfig};
pub use detector::HeaderRateLimitDetector;
pub use limiter::{RateLimiter, RateLimiterGuard};
pub use tier::Tier;

// Re-export provider-specific tier enums
#[cfg(feature = "anthropic")]
pub use tiers::AnthropicTier;
#[cfg(feature = "gemini")]
pub use tiers::GeminiTier;
pub use tiers::OpenAITier;

/// Trait for errors that can be classified as retryable or permanent.
///
/// This trait allows the RateLimiter to determine whether an error should
/// trigger a retry with exponential backoff, or fail immediately.
///
/// # Example
///
/// ```rust,ignore
/// impl RetryableError for MyError {
///     fn is_retryable(&self) -> bool {
///         match self {
///             MyError::NetworkTimeout => true,
///             MyError::ServerOverload => true,
///             MyError::RateLimit => true,
///             MyError::InvalidApiKey => false,
///             MyError::BadRequest => false,
///         }
///     }
/// }
/// ```
pub trait RetryableError {
    /// Returns true if this error should trigger a retry.
    ///
    /// Transient errors like 503 (service unavailable), 429 (rate limit),
    /// or network timeouts should return true. Permanent errors like 401
    /// (unauthorized) or 400 (bad request) should return false.
    fn is_retryable(&self) -> bool;
}
