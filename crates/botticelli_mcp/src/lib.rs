//! Model Context Protocol (MCP) server for Botticelli.
//!
//! This crate provides an MCP server that exposes Botticelli's capabilities
//! as standardized tools and resources that LLMs can use.
//!
//! # Features
//!
//! - **Tools**: Functions LLMs can call (DB queries, narrative execution, etc.)
//! - **Resources**: Data sources LLMs can read (content, narratives, etc.)
//!
//! # Usage
//!
//! ```no_run
//! use botticelli_mcp::BotticelliServer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let service = rmcp::service::serve_server(
//!         BotticelliServer::new(),
//!         rmcp::transport::stdio(),
//!     )
//!     .await?;
//!     service.waiting().await.ok();
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod conversation;
mod resources;
mod server;
pub mod tools;
mod transport;

pub use conversation::{Attachment, ConversationSession, ConversationTurn, SessionState};
pub use resources::{McpResource, NarrativeResource, ResourceInfo, ResourceRegistry};
pub use server::BotticelliServer;
pub use tools::{
    Act, ActMetrics, CreateNarrativeTool, EchoTool, ElicitActInput, ElicitActTool,
    ElicitMetadataInput, ElicitMetadataTool, ElicitationHelper, ExecuteNarrativeTool,
    ExecutionMetrics, ExportMetricsTool, FinalizeNarrativeInput, FinalizeNarrativeTool,
    GenerateTool, LlmSampler, McpTool, MetricsSummary, ModifyNarrativeTool, NarrativeHelper,
    NarrativeRegistry, PrometheusMetrics, QueryContentTool, SamplingCoordinator, SamplingError,
    SamplingErrorKind, SamplingHelper, SamplingResult, SaveNarrativeTool, ServerInfoTool,
    StartNarrativeInput, StartNarrativeTool, ToolRegistry, ValidateNarrativeTool,
};
pub use transport::{HttpTransport, McpTransport, McpTransportError};

#[cfg(feature = "discord")]
pub use tools::{
    DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool, DiscordPostMessageTool,
};

// Export LLM tools based on features
#[cfg(feature = "anthropic")]
pub use tools::GenerateAnthropicTool;
#[cfg(feature = "gemini")]
pub use tools::GenerateGeminiTool;
#[cfg(feature = "groq")]
pub use tools::GenerateGroqTool;
#[cfg(feature = "huggingface")]
pub use tools::GenerateHuggingFaceTool;
#[cfg(feature = "ollama")]
pub use tools::GenerateOllamaTool;

#[cfg(feature = "database")]
pub use resources::ContentResource;
