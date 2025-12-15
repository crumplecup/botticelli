//! MCP client for agentic orchestration and external server connections.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod adapter_bridge;
mod approval;
mod context;
mod error;
mod external_client;
mod llm_adapter;
mod metrics;
mod orchestrator;
mod retry;
pub mod schema;
mod tool_executor;
mod tool_registry;
pub mod tools;
mod unified_client;

pub use adapter_bridge::DriverAdapter;
pub use approval::ApprovalHandler;
pub use context::ContextManager;
pub use error::{McpClientError, McpClientErrorKind, McpClientResult};
pub use external_client::{ExternalMcpClient, ExternalServerConfig};
pub use llm_adapter::{
    AnthropicAdapter, FinishReason, GeminiAdapter, GenerationConfig, GenerationResponse,
    GroqAdapter, LlmAdapter, Message, MessageRole, OllamaAdapter, TokenUsage, ToolCall as LlmToolCall,
    ToolResult, ToolSchema as LlmToolSchema,
};
pub use metrics::McpClientMetrics;
pub use orchestrator::{tool_info_to_provider_schema, tool_info_to_schema, Orchestrator};
pub use retry::{CircuitBreaker, CircuitState, RetryConfig};
pub use tool_executor::ToolDefinition;
pub use tool_registry::{ToolHandler, ToolRegistry};
pub use unified_client::{
    ExecutionResult, LlmBackend, ToolCall, ToolCallRecord, UnifiedClientMetrics,
    UnifiedMcpClient, extract_tool_calls,
};
