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
#[cfg(feature = "gemini")]
pub use tiers::GeminiTier;
#[cfg(feature = "anthropic")]
pub use tiers::AnthropicTier;
pub use tiers::OpenAITier;
