//! Token counting and cost calculation for LLM operations.
use std::sync::Arc;

use tiktoken_rs::CoreBPE;

/// Helper function to get a tokenizer by model name.
///
/// Returns an encoder for the specified model, or an error if the model
/// is not supported by tiktoken-rs.
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be loaded for the specified model.
///
/// # Examples
///
/// ```
/// use botticelli_core::get_tokenizer;
///
/// let encoder = get_tokenizer("gpt-4").expect("Should get encoder");
/// let tokens = encoder.encode_with_special_tokens("Hello, world!");
/// assert!(!tokens.is_empty());
/// ```
pub fn get_tokenizer(model: &str) -> Result<Arc<CoreBPE>, String> {
    tiktoken_rs::get_bpe_from_model(model)
        .map(Arc::new)
        .map_err(|e| format!("Failed to get tokenizer for {}: {}", model, e))
}
