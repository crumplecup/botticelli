//! Model Context Protocol (MCP) server for Botticelli.
//!
//! This crate provides an MCP server that exposes Botticelli's capabilities
//! as standardized tools and resources that LLMs can use.
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
    Act, ActMetrics, ExecutionMetrics, LlmSampler, McpTool, MetricsSummary, NarrativeHelper,
    PrometheusMetrics, SamplingCoordinator, SamplingError, SamplingErrorKind, SamplingHelper,
    SamplingResult, ToolRegistry,
};
pub use transport::{HttpTransport, McpTransport, McpTransportError};

#[cfg(feature = "database")]
pub use resources::ContentResource;
