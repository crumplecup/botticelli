//! Provider-specific tier implementations.
//!
//! This module contains concrete tier enums for each LLM provider (Gemini, Anthropic, OpenAI)
//! with their actual rate limits and pricing. These enums implement the `Tier` trait and
//! provide type-safe tier selection for each provider.

use crate::Tier;

/// Gemini API usage tiers.
///
/// Based on [Gemini API pricing](https://ai.google.dev/pricing).
/// Rates verified from user dashboard as of 2025-01.
#[cfg(feature = "gemini")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeminiTier {
    /// Free tier: 10 RPM, 250K TPM, 250 RPD (Flash 2.0)
    Free,
    /// Pay-as-you-go: 360 RPM, 4M TPM, no daily limit
    PayAsYouGo,
}

#[cfg(feature = "gemini")]
impl Tier for GeminiTier {
    fn rpm(&self) -> Option<u32> {
        match self {
            GeminiTier::Free => Some(10),        // Free tier: 10 RPM (Flash 2.0)
            GeminiTier::PayAsYouGo => Some(360), // Paid: 360 RPM (6 per second)
        }
    }

    fn tpm(&self) -> Option<u64> {
        match self {
            GeminiTier::Free => Some(250_000), // 250K tokens/min (Flash 2.0)
            GeminiTier::PayAsYouGo => Some(4_000_000), // 4M tokens/min
        }
    }

    fn rpd(&self) -> Option<u32> {
        match self {
            GeminiTier::Free => Some(250),  // 250 requests/day (Flash 2.0)
            GeminiTier::PayAsYouGo => None, // No daily limit
        }
    }

    fn max_concurrent(&self) -> Option<u32> {
        Some(1) // Both tiers: 1 concurrent request
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        match self {
            GeminiTier::Free => Some(0.0),
            GeminiTier::PayAsYouGo => Some(0.075), // Gemini 2.0 Flash
        }
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        match self {
            GeminiTier::Free => Some(0.0),
            GeminiTier::PayAsYouGo => Some(0.30),
        }
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        None // No hard USD quota
    }

    fn name(&self) -> &str {
        match self {
            GeminiTier::Free => "Free",
            GeminiTier::PayAsYouGo => "Pay-as-you-go",
        }
    }
}

/// Anthropic API usage tiers.
///
/// Based on [Anthropic pricing](https://docs.anthropic.com/claude/docs/rate-limits).
/// Tiers are automatically assigned based on cumulative spend.
#[cfg(feature = "anthropic")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnthropicTier {
    /// Tier 1: Free/new accounts (5 RPM, 20K TPM)
    Tier1,
    /// Tier 2: $5+ paid (50 RPM, 40K TPM)
    Tier2,
    /// Tier 3: $40+ paid (1000 RPM, 80K TPM)
    Tier3,
    /// Tier 4: $200+ paid (2000 RPM, 160K TPM)
    Tier4,
}

#[cfg(feature = "anthropic")]
impl Tier for AnthropicTier {
    fn rpm(&self) -> Option<u32> {
        match self {
            AnthropicTier::Tier1 => Some(5),
            AnthropicTier::Tier2 => Some(50),
            AnthropicTier::Tier3 => Some(1000),
            AnthropicTier::Tier4 => Some(2000),
        }
    }

    fn tpm(&self) -> Option<u64> {
        match self {
            AnthropicTier::Tier1 => Some(20_000),
            AnthropicTier::Tier2 => Some(40_000),
            AnthropicTier::Tier3 => Some(80_000),
            AnthropicTier::Tier4 => Some(160_000),
        }
    }

    fn rpd(&self) -> Option<u32> {
        None // No daily limit, monthly budget instead
    }

    fn max_concurrent(&self) -> Option<u32> {
        Some(5) // All tiers
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        Some(3.0) // Claude 3.5 Sonnet (varies by model)
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        Some(15.0)
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        None // Monthly budget, not daily
    }

    fn name(&self) -> &str {
        match self {
            AnthropicTier::Tier1 => "Tier 1",
            AnthropicTier::Tier2 => "Tier 2",
            AnthropicTier::Tier3 => "Tier 3",
            AnthropicTier::Tier4 => "Tier 4",
        }
    }
}

/// OpenAI API usage tiers.
///
/// Based on [OpenAI usage tiers](https://platform.openai.com/docs/guides/rate-limits).
/// Tiers are automatically assigned based on cumulative spend and account age.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenAITier {
    /// Free tier: 3 RPM, 40K TPM, 200 RPD
    Free,
    /// Tier 1: $5+ paid (500 RPM, 200K TPM)
    Tier1,
    /// Tier 2: $50+ paid (5000 RPM, 2M TPM)
    Tier2,
    /// Tier 3: $100+ paid (10000 RPM, 10M TPM)
    Tier3,
    /// Tier 4: $250+ paid (10000 RPM, 30M TPM)
    Tier4,
    /// Tier 5: $1000+ paid (10000 RPM, 100M TPM)
    Tier5,
}

impl Tier for OpenAITier {
    fn rpm(&self) -> Option<u32> {
        match self {
            OpenAITier::Free => Some(3),
            OpenAITier::Tier1 => Some(500),
            OpenAITier::Tier2 => Some(5000),
            OpenAITier::Tier3 => Some(10000),
            OpenAITier::Tier4 => Some(10000),
            OpenAITier::Tier5 => Some(10000),
        }
    }

    fn tpm(&self) -> Option<u64> {
        match self {
            OpenAITier::Free => Some(40_000),
            OpenAITier::Tier1 => Some(200_000),
            OpenAITier::Tier2 => Some(2_000_000),
            OpenAITier::Tier3 => Some(10_000_000),
            OpenAITier::Tier4 => Some(30_000_000),
            OpenAITier::Tier5 => Some(100_000_000),
        }
    }

    fn rpd(&self) -> Option<u32> {
        match self {
            OpenAITier::Free => Some(200),
            _ => None,
        }
    }

    fn max_concurrent(&self) -> Option<u32> {
        Some(50) // Batch queue limit
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        Some(2.50) // GPT-4 Turbo (varies by model)
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        Some(10.0)
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        None
    }

    fn name(&self) -> &str {
        match self {
            OpenAITier::Free => "Free",
            OpenAITier::Tier1 => "Tier 1",
            OpenAITier::Tier2 => "Tier 2",
            OpenAITier::Tier3 => "Tier 3",
            OpenAITier::Tier4 => "Tier 4",
            OpenAITier::Tier5 => "Tier 5",
        }
    }
}
