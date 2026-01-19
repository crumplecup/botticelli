//! Tiered Gemini client wrapper.

use botticelli_interface::Tier;
use gemini_rust::Gemini;
use rmcp::tool;

/// Couples a Gemini API client with its rate limiting tier.
///
/// This type wraps a `Gemini` client and a tier (implementing `Tier`) together,
/// enabling the `RateLimiter` to own both the client and its rate limit configuration.
/// This ensures that clients cannot be accessed without going through rate limiting.
///
/// The struct implements `Tier` by delegating all methods to the inner tier,
/// allowing it to be used anywhere a `Tier` is expected (e.g., in `RateLimiter`).
#[derive(Clone, derive_getters::Getters)]
pub struct TieredGemini<T: Tier> {
    /// The Gemini API client
    client: Gemini,
    /// The tier configuration for rate limiting
    tier: T,
}

impl<T: Tier> TieredGemini<T> {
    /// Creates a new tiered Gemini client.
    #[tool]
    pub fn new(client: Gemini, tier: T) -> Self {
        Self { client, tier }
    }
}

impl<T: Tier + std::fmt::Debug> std::fmt::Debug for TieredGemini<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TieredGemini")
            .field("tier", &self.tier)
            .finish_non_exhaustive()
    }
}

impl<T: Tier> Tier for TieredGemini<T> {
    fn rpm(&self) -> Option<u32> {
        self.tier.rpm()
    }

    fn tpm(&self) -> Option<u64> {
        self.tier.tpm()
    }

    fn rpd(&self) -> Option<u32> {
        self.tier.rpd()
    }

    fn max_concurrent(&self) -> Option<u32> {
        self.tier.max_concurrent()
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        self.tier.daily_quota_usd()
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        self.tier.cost_per_million_input_tokens()
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        self.tier.cost_per_million_output_tokens()
    }

    fn name(&self) -> &str {
        self.tier.name()
    }
}
