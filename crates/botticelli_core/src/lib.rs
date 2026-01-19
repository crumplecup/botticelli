//! Core data types for the Botticelli LLM API library.
//!
//! This crate provides the foundation data types used across all Botticelli interfaces.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bot_server;
mod budget;
mod health;
mod input;
mod media;
mod message;
mod metadata;
mod narrative;
mod observability;
mod output;
mod request;
mod role;
mod stream_chunk;
mod token_counting;
mod token_usage;
mod tool_definition;
mod tool_result;

pub use bot_server::{BotServerConfig, BotState, BotStats};
pub use budget::{BudgetConfig, BudgetConfigBuilder, BudgetConfigBuilderError, default_multiplier_tool};
pub use health::HealthStatus;
pub use input::{HistoryRetention, Input, TableFormat};
pub use media::MediaSource;
pub use message::{Message, MessageBuilder};
pub use metadata::{Capabilities, ModelMetadata, ModelMetadataBuilder};
pub use narrative::{
    ActExecution, ActExecutionBuilder, ExecutionFilter, ExecutionStatus, ExecutionSummary,
    NarrativeExecution,
};
pub use observability::{
    ExporterBackend, ObservabilityConfig, init_observability, init_observability_with_config,
    shutdown_observability,
};
pub use output::{Output, StopReason, ToolCall, ToolCallBuilder};
pub use request::{
    GenerateRequest, GenerateRequestBuilder, GenerateRequestBuilderError, GenerateResponse,
    GenerateResponseBuilder, GenerateResponseBuilderError,
};
pub use role::Role;
pub use stream_chunk::{FinishReason, StreamChunk, StreamChunkBuilder};
pub use token_counting::get_tokenizer;
pub use token_usage::{TokenUsageData, TokenUsageDataBuilder};
pub use tool_definition::ToolDefinition;
pub use tool_result::ToolResult;
