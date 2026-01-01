//! Streaming response types.

use crate::Output;
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
)]
#[builder(setter(into))]
pub struct StreamChunk {
    /// Incremental content (usually partial text).
    content: Output,
    /// Whether this is the final chunk.
    is_final: bool,
    /// Optional finish reason if final.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default)]
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
