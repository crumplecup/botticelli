//! Rate limiting and usage tier management.
//!
//! This module provides rate limiting functionality to comply with LLM API quotas.
//! It supports multiple providers with different tier structures and automatically
//! detects limits from API response headers when possible.

pub mod config;
pub mod tier;
pub mod tiers;

pub use config::{BoticelliConfig, ProviderConfig, TierConfig};
pub use tier::Tier;

// Re-export provider-specific tier enums
#[cfg(feature = "gemini")]
pub use tiers::GeminiTier;
#[cfg(feature = "anthropic")]
pub use tiers::AnthropicTier;
pub use tiers::OpenAITier;
