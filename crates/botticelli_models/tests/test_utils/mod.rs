//! Test utilities for Botticelli tests.
//!
//! This module provides mock implementations and test helpers.

pub mod mock_gemini;

#[allow(unused_imports)]
pub use mock_gemini::{MockBehavior, MockGeminiClient, MockResponse};

use botticelli_core::{GenerateRequest, Input, MessageBuilder, Role};
use botticelli_error::BotticelliResult;

/// Creates a test request with a simple message.
pub fn create_test_request(
    content: &str,
    model: Option<String>,
    max_tokens: Option<u32>,
) -> BotticelliResult<GenerateRequest> {
    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text(content.to_string())])
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(
                botticelli_error::BotticelliErrorKind::Backend(
                    botticelli_error::BackendError::new(format!("Builder error: {}", e)),
                ),
            )
        })?;

    GenerateRequest::builder()
        .messages(vec![message])
        .model(model)
        .max_tokens(max_tokens)
        .build()
        .map_err(|e| {
            botticelli_error::BotticelliError::from(
                botticelli_error::BotticelliErrorKind::Backend(
                    botticelli_error::BackendError::new(format!("Builder error: {}", e)),
                ),
            )
        })
}
