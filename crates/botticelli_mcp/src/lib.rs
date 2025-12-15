//! Model Context Protocol (MCP) server for Botticelli.
//!
//! This crate provides an MCP server that exposes Botticelli's capabilities
//! as standardized tools and resources that LLMs can use.
//!
//! # Features
//!
//! - **Tools**: Functions LLMs can call (DB queries, narrative execution, etc.)
//! - **Resources**: Data sources LLMs can read (content, narratives, etc.)
//! - **Prompts**: Reusable prompt templates
//!
//! # Usage
//!
//! ```no_run
//! use botticelli_mcp::{BotticelliRouter, ByteTransport, Server, RouterService};
//! use tokio::io::{stdin, stdout};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let router = BotticelliRouter::builder()
//!         .name("botticelli")
//!         .version(env!("CARGO_PKG_VERSION"))
//!         .build();
//!     
//!     let server = Server::new(RouterService(router));
//!     let transport = ByteTransport::new(stdin(), stdout());
//!     server.run(transport).await?;
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod conversation;
mod elicitation;
mod resources;
mod server;
pub mod tools;

// PMCP migration - new implementation
mod pmcp_adapters;
mod pmcp_middleware;
mod pmcp_server;

#[cfg(feature = "streamable-http")]
mod pmcp_http_server;

#[cfg(feature = "http")]
pub mod http;

pub use conversation::{
    Attachment, ConversationSession, ConversationTurn, SessionState, ToolResult,
};
pub use elicitation::{
    ElicitationDialog, NarrativeElicitor, PartialAct, PartialNarrative, PartialNarrativeBuilder,
    RegistryOperations,
};
pub use resources::{McpResource, NarrativeResource, ResourceInfo, ResourceRegistry};
pub use server::{BotticelliRouter, BotticelliRouterBuilder};
pub use tools::{
    Act, ActMetrics, CreateNarrativeTool, EchoTool, ElicitActInput, ElicitActTool,
    ElicitMetadataInput, ElicitMetadataTool, ElicitationHelper, ExecuteNarrativeTool,
    ExecutionMetrics, ExportMetricsTool, FinalizeNarrativeInput, FinalizeNarrativeTool,
    GenerateTool, LlmSampler, McpTool, MetricsSummary, ModifyNarrativeTool, NarrativeHelper,
    NarrativeRegistry, PrometheusMetrics, QueryContentTool, SamplingCoordinator, SamplingError,
    SamplingErrorKind, SamplingHelper, SamplingResult, SaveNarrativeTool, ServerInfoTool,
    StartNarrativeInput, StartNarrativeTool, ToolDefinition, ToolRegistry, ValidateNarrativeTool,
};

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

// PMCP migration exports
pub use pmcp_server::run_pmcp_server;

#[cfg(feature = "streamable-http")]
pub use pmcp_http_server::run_pmcp_http_server;

// Re-export key mcp-server types for convenience
pub use mcp_server::router::RouterService;
pub use mcp_server::{ByteTransport, Router, Server};
