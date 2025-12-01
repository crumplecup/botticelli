//! Test utilities for Botticelli tests.
//!
//! This module provides mock implementations and test helpers.

// Test utilities may not be used by all test files
#![allow(dead_code)]


pub mod mock_gemini;

#[allow(unused_imports)]
pub use mock_gemini::{MockBehavior, MockGeminiClient, MockResponse};

/// Creates a test GenerateRequest with the given prompt.
///
/// # Panics
/// Panics if the message or request cannot be built (test utility only).
#[cfg(feature = "gemini")]
pub fn create_test_request(
    prompt: &str,
    model: Option<String>,
    max_tokens: Option<u32>,
) -> GenerateRequest {
    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text(prompt.to_string())])
        .build()
        .expect("Test message should be valid");

    GenerateRequest::builder()
        .messages(vec![message])
        .model(model)
        .max_tokens(max_tokens)
        .build()
        .expect("Test request should be valid")
}
