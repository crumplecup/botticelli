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
//! use botticelli_mcp::BotticelliServer;
//! use rmcp::ServerHandler;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let server = BotticelliServer::builder().build();
//!     
//!     // Use rmcp's serve method with stdio transport
//!     let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
//!     server.serve((stdin, stdout)).await?;
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod conversation;
mod dialog_resource;
mod echo;
mod elicitation;
mod errors;
mod resources;
mod rmcp_server;
// Legacy modules - to be migrated or removed
// mod server;
mod server_info;
pub mod tools;
// mod transport;

#[cfg(feature = "http")]
pub mod http;

pub use conversation::{Attachment, ConversationSession, ConversationTurn, SessionState};
pub use dialog_resource::DialogResource;
pub use echo::{EchoParams, EchoResult};
pub use elicitation::{
    ElicitationDialog, NarrativeElicitor, PartialAct, PartialNarrative, PartialNarrativeBuilder,
};
pub use errors::ToolError;
pub use resources::{McpResource, NarrativeResource, ResourceInfo, ResourceRegistry};
pub use rmcp_server::{BotticelliServer, BotticelliServerBuilder};
// Legacy exports - commented out during rmcp migration
// pub use server::{BotticelliRouter, BotticelliRouterBuilder};
pub use server_info::ServerInfoResult;
pub use tools::{
    Act, ActMetrics, CreateNarrativeTool, EchoTool, ElicitActInput, ElicitActTool,
    ElicitBoolTool, ElicitMetadataInput, ElicitMetadataTool, ElicitNumberTool,
    ElicitSelectTool, ElicitTextTool, ElicitationHelper, ExecuteNarrativeTool, ExecutionMetrics,
    ExportMetricsTool, FinalizeNarrativeInput, FinalizeNarrativeTool, GenerateTool, LlmSampler,
    McpTool, MetricsSummary, ModifyNarrativeTool, NarrativeHelper, NarrativeRegistry,
    PrometheusMetrics, QueryContentTool, SamplingCoordinator, SamplingError, SamplingErrorKind,
    SamplingHelper, SamplingResult, SaveNarrativeTool, ServerInfoTool, StartNarrativeInput,
    StartNarrativeTool, ToolRegistry, ValidateNarrativeTool,
};
// Legacy transport exports - commented out during rmcp migration
// pub use transport::{
//     HttpTransport, InProcServerHandle, InProcTransport, McpTransport, McpTransportError,
// };

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
