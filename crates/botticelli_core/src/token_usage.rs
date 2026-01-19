//! Token usage tracking for LLM requests.

use rmcp::tool;
use serde::{Deserialize, Serialize};

/// Token usage information for a completed generation.
///
/// This provides a unified view of token consumption across different
/// LLM providers, which may report tokens differently.
///
/// # Examples
///
/// ```
/// use botticelli_core::TokenUsageData;
///
/// let usage = TokenUsageData::new(150, 50, 200);
/// assert_eq!(*usage.input_tokens(), 150);
/// assert_eq!(*usage.output_tokens(), 50);
/// assert_eq!(*usage.total_tokens(), 200);
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Default,
    derive_getters::Getters,
    derive_builder::Builder,
    elicitation::Elicit,
)]
pub struct TokenUsageData {
    /// Number of tokens in the input/prompt.
    #[builder(default)]
    input_tokens: u64,
    /// Number of tokens in the generated output.
    #[builder(default)]
    output_tokens: u64,
    /// Total tokens consumed (may differ from input + output due to provider accounting).
    #[builder(default)]
    total_tokens: u64,
}

impl TokenUsageData {
    /// Creates new token usage data.
    #[tool]
    pub fn new(input_tokens: u64, output_tokens: u64, total_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens,
        }
    }

    /// Creates a builder for TokenUsageData.
    #[tool]
    pub fn builder() -> TokenUsageDataBuilder {
        TokenUsageDataBuilder::default()
    }

    /// Calculate cost in USD based on pricing per million tokens.
    ///
    /// # Arguments
    ///
    /// * `prompt_price_per_million` - Cost per million prompt tokens in USD
    /// * `completion_price_per_million` - Cost per million completion tokens in USD
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_core::TokenUsageData;
    ///
    /// let usage = TokenUsageData::new(1_000_000, 500_000, 1_500_000);
    /// // $1 per million input, $2 per million output
    /// let cost = usage.calculate_cost(1.0, 2.0);
    /// assert!((cost - 2.0).abs() < 0.001); // 1.0 + 1.0 = 2.0
    /// ```
    #[tool]
    pub fn calculate_cost(
        &self,
        prompt_price_per_million: f64,
        completion_price_per_million: f64,
    ) -> f64 {
        let prompt_cost = (self.input_tokens as f64 / 1_000_000.0) * prompt_price_per_million;
        let completion_cost =
            (self.output_tokens as f64 / 1_000_000.0) * completion_price_per_million;
        prompt_cost + completion_cost
    }
}
