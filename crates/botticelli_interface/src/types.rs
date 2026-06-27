//! Core type definitions for the Botticelli interface.

use serde::{Deserialize, Serialize};

/// A single chunk from a streaming response.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    derive_builder::Builder,
    derive_getters::Getters,
    derive_new::new,
)]
#[builder(setter(into))]
pub struct StreamChunk {
    /// Incremental content (usually partial text).
    content: botticelli_core::Output,
    /// Whether this is the final chunk (stream is exhausted).
    is_final: bool,
    /// Whether this chunk is reasoning/thinking content (not part of the final answer).
    ///
    /// Models like DeepSeek-R1 and Qwen3 emit chain-of-thought tokens before the
    /// answer.  Thinking chunks should be displayed differently (e.g. dimmed) and
    /// are not included in the canonical response.
    #[builder(default)]
    #[new(default)]
    is_thinking: bool,
    /// Optional finish reason if final.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    #[new(default)]
    finish_reason: Option<FinishReason>,
}

impl StreamChunk {
    /// Creates a builder for StreamChunk.
    pub fn builder() -> StreamChunkBuilder {
        StreamChunkBuilder::default()
    }
}

/// Why generation stopped.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    strum::EnumIter,
)]
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

/// Result of a tool execution to send back to the model.
///
/// After the model requests a tool call, your application executes it
/// and sends the result back using this structure.
///
/// **Note**: This type has been moved to `botticelli_core`.
/// Import from there: `use botticelli_core::ToolResult;`
///
/// Information about model capabilities and limits.
#[derive(Debug, Clone, PartialEq, Eq, derive_getters::Getters, derive_builder::Builder)]
pub struct ModelMetadata {
    /// Provider name (e.g., "anthropic", "openai")
    provider: &'static str,
    /// Model identifier (e.g., "claude-3-5-sonnet-20241022")
    model: String,
    /// Maximum input context tokens
    max_input_tokens: usize,
    /// Maximum output tokens per request
    max_output_tokens: usize,
    /// Supports streaming responses
    supports_streaming: bool,
    /// Supports image inputs (vision)
    supports_vision: bool,
    /// Supports audio inputs/outputs
    supports_audio: bool,
    /// Supports video inputs/outputs
    supports_video: bool,
    /// Supports document processing (PDF, etc.)
    supports_documents: bool,
    /// Supports function/tool calling
    supports_tool_use: bool,
    /// Supports structured JSON output mode
    supports_json_mode: bool,
    /// Supports vector embeddings
    supports_embeddings: bool,
    /// Supports batch processing
    supports_batch: bool,
}

/// Provider capability flags for runtime discovery.
///
/// Lightweight struct for querying what features a provider supports
/// without needing full ModelMetadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Capabilities {
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports function/tool calling
    pub tool_calling: bool,
    /// Supports image inputs (vision)
    pub vision: bool,
    /// Supports audio inputs/outputs
    pub audio: bool,
    /// Supports video inputs/outputs
    pub video: bool,
    /// Supports vector embeddings
    pub embeddings: bool,
    /// Supports structured JSON output mode
    pub json_mode: bool,
    /// Supports batch processing
    pub batch_generation: bool,
}

impl Capabilities {
    /// Create capabilities from ModelMetadata.
    pub fn from_metadata(metadata: &ModelMetadata) -> Self {
        Self {
            streaming: *metadata.supports_streaming(),
            tool_calling: *metadata.supports_tool_use(),
            vision: *metadata.supports_vision(),
            audio: *metadata.supports_audio(),
            video: *metadata.supports_video(),
            embeddings: *metadata.supports_embeddings(),
            json_mode: *metadata.supports_json_mode(),
            batch_generation: *metadata.supports_batch(),
        }
    }
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
