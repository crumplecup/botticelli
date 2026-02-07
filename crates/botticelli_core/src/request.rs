//! Request and response types for LLM generation.

use crate::{Message, Output, StopReason};
use rmcp::tool;
use serde::{Deserialize, Serialize};

/// Generic generation request (multimodal-safe).
///
/// # Examples
///
/// ```
/// use botticelli_core::{GenerateRequest, Message, Role, Input};
///
/// let message = Message::new(Role::User, vec![Input::Text("Hello!".to_string())]);
/// let request = GenerateRequest::builder()
///     .messages(vec![message])
///     .build()
///     .expect("Valid request");
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
    derive_builder::Builder,
    schemars::JsonSchema,
    elicitation::Elicit,
)]
#[setters(prefix = "with_")]
#[builder(pattern = "owned", setter(into, strip_option))]
pub struct GenerateRequest {
    /// The conversation messages to send
    messages: Vec<Message>,
    /// Maximum number of tokens to generate
    #[builder(default, setter(strip_option))]
    max_tokens: Option<u32>,
    /// Sampling temperature (0.0 to 1.0)
    #[builder(default, setter(strip_option))]
    temperature: Option<f32>,
    /// Model identifier to use
    #[builder(default, setter(strip_option))]
    model: Option<String>,
}

impl GenerateRequest {
    /// Creates a builder for GenerateRequest.
    #[tool]
    pub fn builder() -> GenerateRequestBuilder {
        GenerateRequestBuilder::default()
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
///     .build()
///     .expect("Valid response");
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
    schemars::JsonSchema,
    elicitation::Elicit,
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
    #[tool]
    pub fn builder() -> GenerateResponseBuilder {
        GenerateResponseBuilder::default()
    }
}
