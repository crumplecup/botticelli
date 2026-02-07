//! Model metadata and capability types.

use serde::{Deserialize, Serialize};

/// Information about model capabilities and limits.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_builder::Builder,
    schemars::JsonSchema,
    elicitation::Elicit,
)]
pub struct ModelMetadata {
    /// Provider name (e.g., "anthropic", "openai")
    provider: String,
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
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_setters::Setters,
    schemars::JsonSchema,
    elicitation::Elicit,
)]
#[setters(prefix = "with_")]
pub struct Capabilities {
    /// Supports streaming responses
    streaming: bool,
    /// Supports function/tool calling
    tool_calling: bool,
    /// Supports image inputs (vision)
    vision: bool,
    /// Supports audio inputs/outputs
    audio: bool,
    /// Supports video inputs/outputs
    video: bool,
    /// Supports vector embeddings
    embeddings: bool,
    /// Supports structured JSON output mode
    json_mode: bool,
    /// Supports batch processing
    batch_generation: bool,
}

impl From<&ModelMetadata> for Capabilities {
    fn from(metadata: &ModelMetadata) -> Self {
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
