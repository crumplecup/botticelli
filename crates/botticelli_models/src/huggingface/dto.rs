//! HuggingFace Inference API data transfer objects.

use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

/// HuggingFace message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HuggingFaceRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// System message
    System,
}

/// HuggingFace message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Builder, Getters)]
#[builder(setter(into))]
pub struct HuggingFaceMessage {
    /// Message role
    role: HuggingFaceRole,
    /// Message content
    content: String,
}

impl HuggingFaceMessage {
    /// Creates a new builder for `HuggingFaceMessage`.
    pub fn builder() -> HuggingFaceMessageBuilder {
        HuggingFaceMessageBuilder::default()
    }
}

/// HuggingFace API request parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, Getters)]
#[builder(setter(into))]
pub struct HuggingFaceRequest {
    /// Model identifier
    model: String,
    /// Input text or messages
    inputs: String,
    /// Maximum tokens to generate
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_new_tokens: Option<usize>,
    /// Temperature for sampling
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Top-p sampling
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    /// Whether to stream the response
    #[builder(default = "false")]
    stream: bool,
}

impl HuggingFaceRequest {
    /// Creates a new builder for `HuggingFaceRequest`.
    pub fn builder() -> HuggingFaceRequestBuilder {
        HuggingFaceRequestBuilder::default()
    }
}

/// Token usage statistics from HuggingFace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Getters)]
pub struct HuggingFaceUsage {
    /// Input tokens consumed
    #[serde(default)]
    input_tokens: usize,
    /// Output tokens generated
    #[serde(default)]
    output_tokens: usize,
}

/// HuggingFace API response.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, Getters)]
#[builder(setter(into))]
pub struct HuggingFaceResponse {
    /// Generated text
    generated_text: String,
    /// Token usage statistics (if available)
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<HuggingFaceUsage>,
}

impl HuggingFaceResponse {
    /// Creates a new builder for `HuggingFaceResponse`.
    pub fn builder() -> HuggingFaceResponseBuilder {
        HuggingFaceResponseBuilder::default()
    }
}
