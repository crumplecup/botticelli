//! Trait definitions for the Botticelli LLM API library.
//!
//! This crate provides the core traits and capability traits that define
//! the Botticelli interface.

mod narrative;
mod table_view;
mod traits;
mod types;

pub use narrative::{
    ActExecution, ExecutionFilter, ExecutionStatus, ExecutionSummary, NarrativeExecution,
    NarrativeRepository,
};
pub use table_view::{TableReference, TableView};
pub use traits::{
    Audio, BatchGeneration, BotticelliDriver, ContentRepository, DocumentProcessing, Embeddings,
    Health, JsonMode, Metadata, Streaming, TableQueryRegistry, TokenCounting, ToolUse, Video,
    Vision,
};
pub use types::{
    FinishReason, HealthStatus, ModelMetadata, StreamChunk, ToolDefinition, ToolResult,
};
