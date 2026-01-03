//! Rate limiting and usage tier management.
//!
//! This module provides rate limiting functionality to comply with LLM API quotas.
//! It supports multiple providers with different tier structures and automatically
//! detects limits from API response headers when possible.
//!
//! ## Provider Tiers
//!
//! Provider-specific tier enums are available behind feature flags:
//! - `GeminiTier` - Available with the `gemini` feature
//! - `AnthropicTier` - Available with the `anthropic` feature
//! - `OpenAITier` - Always available
//!
//! Import tier enums directly from the crate root:
//! ```ignore
//! use botticelli_rate_limit::{OpenAITier, GeminiTier};
//! ```

mod budget;
mod config;
mod detector;
mod error;
mod limiter;
mod tier;
mod tiers;

pub use budget::{Budget, BudgetRemaining};
pub use config::{
    BotticelliConfig, ModelTierConfig, ProviderConfig, RateLimitConfig, TierConfig,
    TierConfigBuilder,
};
pub use detector::HeaderRateLimitDetector;
pub use error::{RateLimitError, RateLimitErrorKind};
pub use limiter::{RateLimiter, RateLimiterGuard};
pub use tier::Tier;
#[cfg(feature = "anthropic")]
pub use tiers::AnthropicTier;
#[cfg(feature = "gemini")]
pub use tiers::GeminiTier;
pub use tiers::OpenAITier;
