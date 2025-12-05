//! Token counting utilities for LLM backends.

use botticelli_error::{ModelsError, ModelsErrorKind, ModelsResult};

/// Create a tiktoken tokenizer for Claude models (using cl100k_base encoding).
pub fn claude_tokenizer() -> ModelsResult<tiktoken_rs::CoreBPE> {
    tiktoken_rs::cl100k_base().map_err(|e| {
        ModelsError::new(ModelsErrorKind::TokenCountingFailed(format!(
            "Failed to load tokenizer: {}",
            e
        )))
    })
}

/// Create a tiktoken tokenizer for GPT-based models (Groq, OpenAI).
pub fn gpt_tokenizer() -> ModelsResult<tiktoken_rs::CoreBPE> {
    tiktoken_rs::cl100k_base().map_err(|e| {
        ModelsError::new(ModelsErrorKind::TokenCountingFailed(format!(
            "Failed to load tokenizer: {}",
            e
        )))
    })
}

/// Count tokens using tiktoken (approximation for Claude, Groq).
pub fn count_tokens_tiktoken(text: &str, tokenizer: &tiktoken_rs::CoreBPE) -> usize {
    tokenizer.encode_with_special_tokens(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_tokenizer() {
        let tokenizer = claude_tokenizer().expect("Failed to load tokenizer");
        let count = count_tokens_tiktoken("Hello, world!", &tokenizer);
        assert!(count > 0);
        assert!(count < 10); // "Hello, world!" should be <10 tokens
    }

    #[test]
    fn test_gpt_tokenizer() {
        let tokenizer = gpt_tokenizer().expect("Failed to load tokenizer");
        let count = count_tokens_tiktoken("Hello, world!", &tokenizer);
        assert!(count > 0);
        assert!(count < 10);
    }
}
