//! Token counting trait implementation for Gemini.

use botticelli_interface::TokenCounting;
use botticelli_error::{GeminiError, GeminiErrorKind};
use crate::GeminiClient;

impl TokenCounting for GeminiClient {
    fn count_tokens(&self, text: &str) -> Result<usize, Self::Error> {
        let tokenizer = crate::token_counting::gpt_tokenizer()
            .map_err(|e| GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string())))?;
        Ok(crate::token_counting::count_tokens_tiktoken(text, &tokenizer))
    }

    fn count_request_tokens(&self, req: &Self::Request) -> Result<usize, Self::Error> {
        // Convert request to text representation for counting
        let text = serde_json::to_string(req)
            .map_err(|e| GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string())))?;
        self.count_tokens(&text)
    }
}
