//! Trait definitions for the Botticelli LLM API library.
//!
//! This crate provides the core traits and capability traits that define
//! the Botticelli interface.

mod bot_server;
mod chat_host;
mod narrative;
mod provider;
mod registry;
mod registry_traits;
mod table_query_view;
mod table_view;
mod traits;
mod types;

pub use bot_server::{BotActor, BotResult, BotServer, BotServerConfig, BotState, BotStats};
pub use chat_host::{ChatHost, ChatMessage};
pub use narrative::{
    ActExecution, ActExecutionBuilder, ExecutionFilter, ExecutionStatus, ExecutionSummary,
    NarrativeExecution, NarrativeRepository,
};
pub use provider::{LlmProvider, ProviderError, ProviderErrorKind};
pub use registry::RegistryOperations;
pub use registry_traits::{
    DatabaseRegistryOperations, ElicitationRegistryOperations, NarrativeRegistryOperations,
    NarrativeStorageOperations,
};
pub use table_query_view::{
    TableCountView, TableCountViewBuilder, TableQueryView, TableQueryViewBuilder,
};
pub use table_view::{TableReference, TableView};
pub use traits::{
    Audio, BatchGeneration, BotticelliDriver, ContentRepository, DocumentProcessing, Embeddings,
    Health, JsonMode, Metadata, Streaming, TableQueryRegistry, TokenCounting, ToolCalling, Video,
    Vision,
};
pub use types::{
    Capabilities, FinishReason, HealthStatus, ModelMetadata, ModelMetadataBuilder, StreamChunk,
};
