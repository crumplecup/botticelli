//! Request and response types for LLM generation.

use crate::{Message, Output};
use serde::{Deserialize, Serialize};

/// Generic generation request (multimodal-safe).
///
/// # Examples
///
/// ```
/// use botticelli_core::{GenerateRequest, Message, Role, Input};
///
/// let request = GenerateRequest::builder()
///     .messages(vec![Message {
///         role: Role::User,
///         content: vec![Input::Text("Hello!".to_string())],
///     }])
///     .max_tokens(Some(100))
///     .temperature(Some(0.7))
///     .model(Some("gemini-2.0-flash-lite".to_string()))
///     .build()
///     .unwrap();
///
/// assert_eq!(request.messages().len(), 1);
/// assert_eq!(request.max_tokens(), &Some(100));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct GenerateRequest {
    /// The conversation messages to send
    messages: Vec<Message>,
    /// Maximum number of tokens to generate
    max_tokens: Option<u32>,
    /// Sampling temperature (0.0 to 1.0)
    temperature: Option<f32>,
    /// Model identifier to use
    model: Option<String>,
}

impl GenerateRequest {
    /// Creates a new builder for GenerateRequest.
    pub fn builder() -> GenerateRequestBuilder {
        GenerateRequestBuilder::default()
    }
}

/// Builder for GenerateRequest.
#[derive(Debug, Clone, Default)]
pub struct GenerateRequestBuilder {
    messages: Option<Vec<Message>>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    model: Option<String>,
}

impl GenerateRequestBuilder {
    /// Sets the messages.
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = Some(messages);
        self
    }

    /// Sets the max_tokens.
    pub fn max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the temperature.
    pub fn temperature(mut self, temperature: Option<f32>) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the model.
    pub fn model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    /// Builds the GenerateRequest.
    pub fn build(self) -> Result<GenerateRequest, String> {
        Ok(GenerateRequest {
            messages: self.messages.ok_or("messages is required")?,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            model: self.model,
        })
    }
}

/// The unified response object.
///
/// # Examples
///
/// ```
/// use botticelli_core::{GenerateResponse, Output};
///
/// let response = GenerateResponse {
///     outputs: vec![Output::Text("Hello! How can I help?".to_string())],
/// };
///
/// assert_eq!(response.outputs.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// The generated outputs from the model
    pub outputs: Vec<Output>,
}
