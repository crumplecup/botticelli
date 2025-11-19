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
//! Import tier enums directly from their defining crate:
//! ```ignore
//! use botticelli_rate_limit::tiers::GeminiTier;
//! ```

mod config;
mod detector;
mod error;
mod limiter;
mod tier;
pub mod tiers;

pub use config::{BotticelliConfig, ModelTierConfig, ProviderConfig, TierConfig};
pub use detector::HeaderRateLimitDetector;
pub use error::{RateLimitError, RateLimitErrorKind};
pub use limiter::{RateLimiter, RateLimiterGuard};
pub use tier::Tier;
