use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use async_trait::async_trait;
use serde_json::Value;

/// Represents a message in a conversation
#[derive(Debug, Clone)]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: String,
    /// Optional tool calls made by the assistant
    pub tool_calls: Vec<ToolCall>,
    /// Optional tool call results
    pub tool_results: Vec<ToolResult>,
}

/// Role of a message sender
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant/model message
    Assistant,
    /// System prompt
    System,
    /// Tool result message
    Tool,
}

/// A tool call requested by the LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments for the tool as JSON
    pub arguments: Value,
}

/// Result from executing a tool
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// ID of the tool call this is responding to
    pub tool_call_id: String,
    /// Result content as JSON
    pub content: Value,
    /// Whether the tool execution was successful
    pub is_error: bool,
}

/// Configuration for LLM generation
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature for sampling
    pub temperature: Option<f32>,
    /// Top-p for nucleus sampling
    pub top_p: Option<f32>,
    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(0.95),
            stop_sequences: Vec::new(),
        }
    }
}

/// Response from LLM generation
#[derive(Debug, Clone)]
pub struct GenerationResponse {
    /// Generated message
    pub message: Message,
    /// Usage statistics
    pub usage: TokenUsage,
    /// Finish reason
    pub finish_reason: FinishReason,
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Reason the model stopped generating
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// Model naturally finished
    Stop,
    /// Hit max tokens limit
    MaxTokens,
    /// Model requested tool calls
    ToolCalls,
    /// Content filtered
    ContentFilter,
    /// Error occurred
    Error,
}

/// Tool schema for LLM
#[derive(Debug, Clone)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON schema for tool parameters
    pub parameters: Value,
}

/// Adapter trait for different LLM providers
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Generate a response with optional tool calling
    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse>;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Check if the model supports tool calling
    fn supports_tools(&self) -> bool;

    /// Get maximum context window size
    fn max_context_tokens(&self) -> u32;
}

/// Adapter for Anthropic Claude models
pub struct AnthropicAdapter {
    model: String,
    api_key: String,
}

impl AnthropicAdapter {
    /// Gets the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl AnthropicAdapter {
    /// Create a new Anthropic adapter
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl LlmAdapter for AnthropicAdapter {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse> {
        // TODO: Implement Anthropic API call
        Err(McpClientError::new(McpClientErrorKind::LlmError(
            "Not yet implemented".to_string(),
        )))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u32 {
        200_000 // Claude 3.5 context window
    }
}

/// Adapter for Google Gemini models
pub struct GeminiAdapter {
    model: String,
    api_key: String,
}

impl GeminiAdapter {
    /// Gets the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl GeminiAdapter {
    /// Create a new Gemini adapter
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl LlmAdapter for GeminiAdapter {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse> {
        // TODO: Implement Gemini API call
        Err(McpClientError::new(McpClientErrorKind::LlmError(
            "Not yet implemented".to_string(),
        )))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u32 {
        1_000_000 // Gemini 1.5 Pro context window
    }
}

/// Adapter for OpenAI models via Groq
pub struct GroqAdapter {
    model: String,
    api_key: String,
}

impl GroqAdapter {
    /// Gets the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl GroqAdapter {
    /// Create a new Groq adapter
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl LlmAdapter for GroqAdapter {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse> {
        // TODO: Implement Groq API call
        Err(McpClientError::new(McpClientErrorKind::LlmError(
            "Not yet implemented".to_string(),
        )))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u32 {
        8192 // Typical for Groq models
    }
}

/// Adapter for Ollama local models
pub struct OllamaAdapter {
    model: String,
    base_url: String,
}

impl OllamaAdapter {
    /// Gets the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl OllamaAdapter {
    /// Create a new Ollama adapter
    pub fn new(model: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl LlmAdapter for OllamaAdapter {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: GenerationConfig,
    ) -> McpClientResult<GenerationResponse> {
        // TODO: Implement Ollama API call
        Err(McpClientError::new(McpClientErrorKind::LlmError(
            "Not yet implemented".to_string(),
        )))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        false // Most Ollama models don't support native tool calling
    }

    fn max_context_tokens(&self) -> u32 {
        4096 // Varies by model
    }
}
