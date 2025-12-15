//! MCP client for agentic orchestration and external server connections.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod approval;
mod context;
mod error;
mod external_client;
mod llm_adapter;
mod metrics;
mod retry;
pub mod schema;
mod tool_executor;
mod unified_client;

pub use approval::ApprovalHandler;
pub use context::ContextManager;
pub use error::{McpClientError, McpClientErrorKind, McpClientResult};
pub use external_client::{ExternalMcpClient, ExternalServerConfig};
pub use llm_adapter::LlmAdapter;
pub use metrics::McpClientMetrics;
pub use retry::{CircuitBreaker, CircuitState, RetryConfig};
pub use tool_executor::ToolDefinition;
pub use unified_client::{LlmBackend, ToolCall, UnifiedClientMetrics, UnifiedMcpClient, extract_tool_calls};
