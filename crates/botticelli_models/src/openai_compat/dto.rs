//! Data transfer objects for OpenAI-compatible APIs.

use derive_builder::Builder;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

/// A message in the OpenAI chat format.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct ChatMessage {
    /// Role: "system", "user", or "assistant"
    role: String,
    /// Message content
    content: String,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ChatToolCall>>,
}

impl ChatMessage {
    /// Creates a new ChatMessage.
    pub fn new(role: String, content: String, tool_calls: Option<Vec<ChatToolCall>>) -> Self {
        Self {
            role,
            content,
            tool_calls,
        }
    }
}

/// Tool call in OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct ChatToolCall {
    /// Unique ID for this tool call
    id: String,
    /// Type (always "function" for now)
    #[serde(rename = "type")]
    call_type: String,
    /// Function call details
    function: ChatFunction,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct ChatFunction {
    /// Function name
    name: String,
    /// Function arguments as JSON string
    arguments: String,
}

/// Tool definition in OpenAI format.
#[derive(Debug, Clone, Serialize, Getters)]
pub struct ChatTool {
    /// Type (always "function")
    #[serde(rename = "type")]
    tool_type: String,
    /// Function definition
    function: ChatFunctionDef,
}

impl ChatTool {
    /// Creates a new ChatTool.
    pub fn new(tool_type: String, function: ChatFunctionDef) -> Self {
        Self {
            tool_type,
            function,
        }
    }
}

/// Function definition.
#[derive(Debug, Clone, Serialize, Getters)]
pub struct ChatFunctionDef {
    /// Function name
    name: String,
    /// Function description
    description: String,
    /// Parameters schema
    parameters: serde_json::Value,
}

impl ChatFunctionDef {
    /// Creates a new ChatFunctionDef.
    pub fn new(name: String, description: String, parameters: serde_json::Value) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

/// OpenAI chat completion request.
#[derive(Debug, Clone, Serialize, Builder, Getters)]
#[builder(setter(into))]
pub struct ChatRequest {
    /// Model identifier
    model: String,
    /// Conversation messages
    messages: Vec<ChatMessage>,
    /// Maximum tokens to generate
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// Sampling temperature
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Enable streaming
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    /// Available tools for the model to call
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ChatTool>>,
}

impl ChatRequest {
    /// Creates a new builder for ChatRequest.
    pub fn builder() -> ChatRequestBuilder {
        ChatRequestBuilder::default()
    }
}

/// A choice in the OpenAI response.
///
/// Represents one possible completion from the model. In non-streaming mode,
/// typically contains a single choice with the complete response.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct ChatChoice {
    /// The message content returned by the model
    pub message: ChatMessage,

    /// Reason the model stopped generating
    ///
    /// Common values: "stop" (natural completion), "length" (max tokens reached),
    /// "content_filter" (filtered by safety systems).
    /// Deserialized from API response.
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Token usage statistics for a completion request.
///
/// Tracks token consumption for billing and rate limiting purposes.
/// All providers return slightly different formats, so fields are optional.
/// All fields are public as this is a DTO deserialized from API responses.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct ChatUsage {
    /// Number of tokens in the input prompt
    ///
    /// Used to calculate input costs and track rate limits.
    /// Deserialized from API response.
    #[serde(default)]
    pub prompt_tokens: Option<usize>,

    /// Number of tokens in the generated completion
    ///
    /// Used to calculate output costs (typically higher than input).
    /// Deserialized from API response.
    #[serde(default)]
    pub completion_tokens: Option<usize>,

    /// Total tokens used (prompt + completion)
    ///
    /// May differ slightly from sum due to provider-specific counting.
    /// Deserialized from API response.
    #[serde(default)]
    pub total_tokens: Option<usize>,
}

/// OpenAI chat completion response.
///
/// Returned by OpenAI-compatible APIs after a successful completion request.
/// Contains the generated text and metadata about token usage.
/// All fields are public as this is a DTO deserialized from API responses.
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct ChatResponse {
    /// One or more completion choices
    ///
    /// Non-streaming requests typically return a single choice.
    /// The `n` parameter in the request controls how many choices are returned.
    pub choices: Vec<ChatChoice>,

    /// Token usage statistics for this request
    ///
    /// Used for billing calculations and rate limit tracking.
    /// May be absent in streaming responses or error cases.
    /// Deserialized from API response.
    #[serde(default)]
    pub usage: Option<ChatUsage>,
}

