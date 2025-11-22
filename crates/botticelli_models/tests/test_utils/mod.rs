//! Test utilities for Botticelli tests.
//!
//! This module provides mock implementations and test helpers.

use botticelli_core::{GenerateRequest, GenerateRequestBuilder, Message, Role};

pub mod mock_gemini;

#[allow(unused_imports)]
pub use mock_gemini::{MockBehavior, MockGeminiClient, MockResponse};

/// Helper to create a test GenerateRequest using the builder pattern.
pub fn create_test_request(
    prompt: &str,
    model: Option<String>,
    max_tokens: Option<u32>,
) -> GenerateRequest {
    let message = botticelli_core::MessageBuilder::default()
        .role(Role::User)
        .content(vec![botticelli_core::Input::Text(prompt.to_string())])
        .build()
        .expect("Failed to build message");
    
    GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(max_tokens)
        .temperature(None)
        .model(model)
        .build()
        .expect("Failed to build test request")
}
