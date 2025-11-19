use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat completion response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionResponse {
    /// Unique identifier for the completion
    pub id: String,
    /// Object type (always "chat.completion")
    pub object: String,
    /// Unix timestamp of when the completion was created
    pub created: i64,
    /// Model used for completion
    pub model: String,
    /// Generated completions
    pub choices: Vec<Choice>,
    /// Token usage statistics
    pub usage: Usage,
}

/// A completion choice
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Choice {
    /// Index of this choice
    pub index: u32,
    /// The generated message
    pub message: ChoiceMessage,
    /// Reason why generation finished
    pub finish_reason: String,
}

/// Message in a choice
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChoiceMessage {
    /// Role of the message (typically "assistant")
    pub role: String,
    /// Generated content
    pub content: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Streaming chat completion chunk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionChunk {
    /// Unique identifier
    pub id: String,
    /// Object type (always "chat.completion.chunk")
    pub object: String,
    /// Unix timestamp
    pub created: i64,
    /// Model used
    pub model: String,
    /// Delta choices
    pub choices: Vec<ChunkChoice>,
}

/// A choice in a streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChunkChoice {
    /// Index of this choice
    pub index: u32,
    /// Delta content
    pub delta: Delta,
    /// Finish reason (if complete)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Delta {
    /// Role (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Incremental content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}
