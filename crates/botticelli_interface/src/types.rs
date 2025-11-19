//! Core type definitions for the Botticelli interface.

use serde::{Deserialize, Serialize};

/// A single chunk from a streaming response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Incremental content (usually partial text).
    pub content: botticelli_core::Output,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Optional finish reason if final.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// Why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::EnumIter)]
pub enum FinishReason {
    /// Model completed naturally.
    Stop,
    /// Hit max_tokens limit.
    Length,
    /// Hit a stop sequence.
    StopSequence,
    /// Model requested tool/function call.
    ToolUse,
    /// Content was filtered.
    ContentFilter,
    /// Other/unknown reason.
    Other,
}

/// Definition of a tool/function that the model can call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Name of the tool/function
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: String,
    /// JSON Schema defining the parameters this tool accepts
    pub parameters: serde_json::Value,
}

/// Result of a tool execution to send back to the model.
///
/// After the model requests a tool call, your application executes it
/// and sends the result back using this structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID matching the tool call this is responding to
    pub id: String,
    /// The output from executing the tool (as JSON)
    pub output: serde_json::Value,
    /// Whether the tool execution resulted in an error
    pub is_error: bool,
}

/// Information about model capabilities and limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMetadata {
    /// Provider name (e.g., "anthropic", "openai")
    pub provider: &'static str,
    /// Model identifier (e.g., "claude-3-5-sonnet-20241022")
    pub model: String,
    /// Maximum input context tokens
    pub max_input_tokens: usize,
    /// Maximum output tokens per request
    pub max_output_tokens: usize,
    /// Supports streaming responses
    pub supports_streaming: bool,
    /// Supports image inputs (vision)
    pub supports_vision: bool,
    /// Supports audio inputs/outputs
    pub supports_audio: bool,
    /// Supports video inputs/outputs
    pub supports_video: bool,
    /// Supports document processing (PDF, etc.)
    pub supports_documents: bool,
    /// Supports function/tool calling
    pub supports_tool_use: bool,
    /// Supports structured JSON output mode
    pub supports_json_mode: bool,
    /// Supports vector embeddings
    pub supports_embeddings: bool,
    /// Supports batch processing
    pub supports_batch: bool,
}

/// Health status of the backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HealthStatus {
    /// System is fully operational
    Healthy,
    /// System is operational but with reduced performance
    Degraded {
        /// Description of the degradation
        message: String,
    },
    /// System is not operational
    Unhealthy {
        /// Description of the problem
        message: String,
    },
}
