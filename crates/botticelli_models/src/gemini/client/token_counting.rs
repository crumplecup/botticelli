//! Token counting trait implementation for Gemini.

use botticelli_interface::TokenCounting;
use botticelli_error::{GeminiError, GeminiErrorKind};
use crate::GeminiClient;

/// Create a tokenizer for Gemini (uses GPT encoding).
fn gemini_tokenizer() -> Result<tiktoken_rs::CoreBPE, GeminiError> {
    tiktoken_rs::cl100k_base().map_err(|e| {
        GeminiError::new(GeminiErrorKind::Tiktoken(e.to_string()))
    })
}

impl TokenCounting for GeminiClient {
    fn count_tokens(&self, text: &str) -> Result<usize, Self::Error> {
        let tokenizer = gemini_tokenizer()?;
        Ok(crate::token_counting::count_tokens_tiktoken(text, &tokenizer))
    }

    fn count_request_tokens(&self, req: &Self::Request) -> Result<usize, Self::Error> {
        // Convert request to text representation for counting
        let text = serde_json::to_string(req)
            .map_err(|e| GeminiError::new(GeminiErrorKind::Serialization(std::sync::Arc::new(e))))?;
        self.count_tokens(&text)
    }
}
