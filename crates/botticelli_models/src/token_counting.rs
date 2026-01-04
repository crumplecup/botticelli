//! Token counting utilities for LLM backends.

use botticelli_error::{ModelsError, ModelsErrorKind, ModelsResult};

/// Create a tiktoken tokenizer for Claude models (using cl100k_base encoding).
pub fn claude_tokenizer() -> ModelsResult<tiktoken_rs::CoreBPE> {
    tiktoken_rs::cl100k_base()
        .map_err(|e| ModelsError::new(ModelsErrorKind::Tiktoken(std::sync::Arc::new(e))))
}

/// Create a tiktoken tokenizer for GPT-based models (Groq, OpenAI).
pub fn gpt_tokenizer() -> ModelsResult<tiktoken_rs::CoreBPE> {
    tiktoken_rs::cl100k_base()
        .map_err(|e| ModelsError::new(ModelsErrorKind::Tiktoken(std::sync::Arc::new(e))))
}

/// Count tokens using tiktoken (approximation for Claude, Groq).
pub fn count_tokens_tiktoken(text: &str, tokenizer: &tiktoken_rs::CoreBPE) -> usize {
    tokenizer.encode_with_special_tokens(text).len()
}
