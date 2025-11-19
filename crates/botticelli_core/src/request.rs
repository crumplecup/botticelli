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
/// let request = GenerateRequest {
///     messages: vec![Message {
///         role: Role::User,
///         content: vec![Input::Text("Hello!".to_string())],
///     }],
///     max_tokens: Some(100),
///     temperature: Some(0.7),
///     model: Some("gemini-2.0-flash-lite".to_string()),
/// };
///
/// assert_eq!(request.messages.len(), 1);
/// assert_eq!(request.max_tokens, Some(100));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GenerateRequest {
    /// The conversation messages to send
    pub messages: Vec<Message>,
    /// Maximum number of tokens to generate
    pub max_tokens: Option<u32>,
    /// Sampling temperature (0.0 to 1.0)
    pub temperature: Option<f32>,
    /// Model identifier to use
    pub model: Option<String>,
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
