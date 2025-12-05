//! MCP client for tool-enabled LLM interactions.
//!
//! This crate provides a client that connects LLM backends with MCP tools,
//! enabling autonomous agent behavior.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod approval;
mod client;
mod context;
mod error;
mod llm_adapter;
mod metrics;
mod retry;
mod schema;
mod tool_executor;

pub use approval::{ApprovalHandler, ApprovalManager, ApprovalPolicy, ConsoleApprovalHandler};
pub use client::{LlmBackend, McpClient};
pub use context::ContextManager;
pub use error::{McpClientError, McpClientErrorKind, McpClientResult};
pub use llm_adapter::{
    AnthropicAdapter, FinishReason, GenerationConfig, GenerationResponse, GeminiAdapter,
    GroqAdapter, LlmAdapter, Message, MessageRole, OllamaAdapter, ToolCall, ToolResult,
    TokenUsage,
};
pub use metrics::McpClientMetrics;
pub use retry::{retry_with_backoff, CircuitBreaker, CircuitState, RetryConfig};
pub use schema::{
    AnthropicToolSchema, GeminiToolSchema, GroqToolSchema, HuggingFaceToolSchema,
    OllamaToolSchema, OpenAIToolSchema, ToolSchema, ToolSchemaConverter,
};
pub use tool_executor::ToolDefinition;
