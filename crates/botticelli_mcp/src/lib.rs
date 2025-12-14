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

mod elicitation;
mod resources;
mod server;
pub mod tools;

#[cfg(feature = "http")]
pub mod http;

pub use elicitation::{
    ElicitationDialog, NarrativeElicitor, PartialAct, PartialNarrative, PartialNarrativeBuilder,
};
pub use resources::{McpResource, NarrativeResource, ResourceInfo, ResourceRegistry};
pub use server::{BotticelliRouter, BotticelliRouterBuilder};
pub use tools::{
    Act, ActMetrics, CreateNarrativeTool, EchoTool, ElicitActInput, ElicitActTool,
    ElicitMetadataInput, ElicitMetadataTool, ElicitationHelper, ExecuteNarrativeTool,
    ExecutionMetrics, ExportMetricsTool, FinalizeNarrativeInput, FinalizeNarrativeTool,
    GenerateTool, LlmSampler, McpTool, MetricsSummary, ModifyNarrativeTool, NarrativeHelper,
    NarrativeRegistry, PrometheusMetrics, QueryContentTool, SamplingCoordinator, SamplingHelper,
    SamplingSession, SaveNarrativeTool, ServerInfoTool, SessionState, StartNarrativeInput,
    StartNarrativeTool, ToolRegistry, ToolResponse, Turn, ValidateNarrativeTool,
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

// Re-export key mcp-server types for convenience
pub use mcp_server::router::RouterService;
pub use mcp_server::{ByteTransport, Router, Server};
