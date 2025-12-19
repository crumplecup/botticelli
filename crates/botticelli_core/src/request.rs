//! Request and response types for LLM generation.

use crate::{Message, Output, StopReason, ToolDefinition};
use serde::{Deserialize, Serialize};

/// Generic generation request (multimodal-safe).
///
/// # Examples
///
/// ```
/// use botticelli_core::{GenerateRequest, Message, Role, Input};
///
/// let message = Message::new(Role::User, vec![Input::Text("Hello!".to_string())]);
/// let request = GenerateRequest::new(vec![message]);
///
/// assert_eq!(request.messages().len(), 1);
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    derive_getters::Getters,
    derive_setters::Setters,
)]
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
    /// Available tools for the LLM to call
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

impl GenerateRequest {
    /// Creates a new GenerateRequest with the given messages.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            max_tokens: None,
            temperature: None,
            model: None,
            tools: None,
        }
    }

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
    tools: Option<Vec<ToolDefinition>>,
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

    /// Sets the available tools for the LLM to call.
    pub fn tools(mut self, tools: Option<Vec<ToolDefinition>>) -> Self {
        self.tools = tools;
        self
    }

    /// Builds the GenerateRequest.
    pub fn build(self) -> Result<GenerateRequest, String> {
        Ok(GenerateRequest {
            messages: self.messages.ok_or("messages is required")?,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            model: self.model,
            tools: self.tools,
        })
    }
}

/// The unified response object.
///
/// # Examples
///
/// ```
/// use botticelli_core::{GenerateResponse, Output, StopReason};
///
/// let response = GenerateResponse::builder()
///     .outputs(vec![Output::Text("Hello! How can I help?".to_string())])
///     .stop_reason(StopReason::EndTurn)
///     .build();
///
/// assert_eq!(response.outputs().len(), 1);
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    derive_builder::Builder,
    derive_getters::Getters,
)]
#[builder(setter(into))]
pub struct GenerateResponse {
    /// The generated outputs from the model
    outputs: Vec<Output>,
    /// Why the generation stopped (per MCP spec)
    stop_reason: StopReason,
    /// Token usage information (if available from provider)
    #[builder(default)]
    usage: Option<crate::TokenUsageData>,
}

impl GenerateResponse {
    /// Creates a builder for GenerateResponse.
    pub fn builder() -> GenerateResponseBuilder {
        GenerateResponseBuilder::default()
    }
}
