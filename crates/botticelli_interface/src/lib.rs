//! Trait definitions for the Botticelli LLM API library.
//!
//! This crate provides the core traits and capability traits that define
//! the Botticelli interface.

mod narrative;
mod traits;
mod types;

pub use narrative::{
    ActExecution, ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeExecution,
    NarrativeRepository,
};
pub use traits::{
    Audio, BatchGeneration, BotticelliDriver, DocumentProcessing, Embeddings, Health, JsonMode,
    Metadata, Streaming, TokenCounting, ToolUse, Video, Vision,
};
pub use types::{
    FinishReason, HealthStatus, ModelMetadata, StreamChunk, ToolDefinition, ToolResult,
};
