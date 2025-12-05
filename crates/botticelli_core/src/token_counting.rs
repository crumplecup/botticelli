/// Token counting and cost calculation for LLM operations.
use std::sync::Arc;

use tiktoken_rs::CoreBPE;

/// Token usage statistics for a single LLM operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_getters::Getters)]
pub struct TokenUsage {
    /// Tokens in the prompt/input.
    prompt_tokens: usize,
    /// Tokens in the response/output.
    completion_tokens: usize,
    /// Total tokens (prompt + completion).
    total_tokens: usize,
}

impl TokenUsage {
    /// Create a new token usage record.
    pub fn new(prompt_tokens: usize, completion_tokens: usize) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }

    /// Calculate cost in USD based on pricing per million tokens.
    ///
    /// # Arguments
    ///
    /// * `prompt_price_per_million` - Cost per million prompt tokens in USD
    /// * `completion_price_per_million` - Cost per million completion tokens in USD
    pub fn calculate_cost(
        &self,
        prompt_price_per_million: f64,
        completion_price_per_million: f64,
    ) -> f64 {
        let prompt_cost = (self.prompt_tokens as f64 / 1_000_000.0) * prompt_price_per_million;
        let completion_cost =
            (self.completion_tokens as f64 / 1_000_000.0) * completion_price_per_million;
        prompt_cost + completion_cost
    }
}

/// Helper function to get a tokenizer by model name.
///
/// Returns an encoder for the specified model, or an error if the model
/// is not supported by tiktoken-rs.
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be loaded for the specified model.
pub fn get_tokenizer(model: &str) -> Result<Arc<CoreBPE>, Box<dyn std::error::Error + Send + Sync>> {
    tiktoken_rs::get_bpe_from_model(model)
        .map(Arc::new)
        .map_err(|e| format!("Failed to get tokenizer for {}: {}", model, e).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_new() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_token_usage_calculate_cost() {
        let usage = TokenUsage::new(1_000_000, 500_000);
        // $1 per million prompt, $2 per million completion
        let cost = usage.calculate_cost(1.0, 2.0);
        assert!((cost - 2.0).abs() < 0.001); // 1.0 + 1.0 = 2.0
    }

    #[test]
    fn test_get_tokenizer() {
        let encoder = get_tokenizer("gpt-4").expect("Should get encoder");
        let tokens = encoder.encode_with_special_tokens("Hello, world!");
        assert!(!tokens.is_empty());
    }
}
