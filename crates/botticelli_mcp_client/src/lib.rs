//! MCP client for agentic orchestration and external server connections.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod adapter_bridge;
mod approval;
mod client;
pub mod connection;
mod context;
mod error;
mod external_client;
mod llm_adapter;
mod metrics;
mod orchestrator;
mod retry;
pub mod schema;
mod tool_registry;
pub mod tools;
mod transport;

pub use adapter_bridge::DriverAdapter;
pub use approval::{ApprovalHandler, ApprovalManager, ApprovalPolicy, ConsoleApprovalHandler};
pub use context::ContextManager;
pub use error::{McpClientError, McpClientErrorKind, McpClientResult};
pub use external_client::{ExternalMcpClient, ExternalServerConfig};
pub use llm_adapter::{
    AnthropicAdapter, FinishReason, GeminiAdapter, GenerationConfig, GenerationResponse,
    GroqAdapter, LlmAdapter, Message, MessageRole, OllamaAdapter,
    ToolCall as LlmToolCall, ToolSchema as LlmToolSchema,
};

// Re-export TokenUsageData from core as TokenUsage for backward compatibility
pub use botticelli_core::TokenUsageData as TokenUsage;
pub use metrics::McpClientMetrics;
pub use orchestrator::{Orchestrator, tool_info_to_provider_schema, tool_info_to_schema};
pub use retry::{CircuitBreaker, CircuitState, RetryConfig, RetryState, retry_with_backoff};
pub use tool_registry::{ToolHandler, ToolRegistry};
pub use transport::{HttpTransport, McpTransport, StdioTransport};

// Re-export from botticelli_core for backward compatibility
pub use botticelli_core::ToolDefinition;
pub use tools::{
    CreateCarouselTool, CreateElicitationSessionTool, CreateNarrativeTool, ElicitActTool,
    ElicitMetadataTool, ElicitationRegistry, ExecuteCarouselTool, FinalizeElicitationTool,
    GenericRegistry, GetRegistryItemTool, ListNarrativesTool, ListRegistryKeysTool,
    LoadNarrativeTool, UpsertRegistryItemTool, ValidateNarrativeTool, register_internal_tools,
};

#[cfg(feature = "database")]
pub use tools::{
    CreateTableTool, InspectTableTool, QueryTableTool, TableExistsTool, register_database_tools,
};

// Discord tools temporarily disabled - need proper implementation
// #[cfg(feature = "discord")]
// pub use tools::{DiscordGetMessagesTool, DiscordSendMessageTool};
pub use client::{
    ExecutionResult, LlmBackend, McpHost, ToolCall, ToolCallRecord, UnifiedClientMetrics,
    extract_tool_calls,
};
