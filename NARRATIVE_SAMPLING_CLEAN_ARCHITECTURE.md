# Narrative Sampling: Clean Architecture Implementation

## Status

**Last Updated:** 2024-12-14

### Completed
- ✅ **Phase 1 Complete** (2024-12-14)
  - Provider abstraction layer
  - Conversation modeling
  - Old types removed and replaced
  - All tests passing (55 tests)
- ✅ **Task 2.1 Complete** (2024-12-14)
  - LlmSampler trait refactored with generate(), sample(), execute_tools()
  - Default implementation provided
  - Error types added
  - All tests passing (14 tests)

### In Progress
- 🔄 **Phase 4: Testing & Polish**
  - Task 4.4: Documentation (not started)

### Completed (Latest First)
- ✅ **Task 4.3 Complete** (2024-12-14)
  - Comprehensive error handling audit completed
  - All error types follow project standards (derive_more)
  - Zero critical issues, 6 minor cosmetic issues (non-blocking)
  - Location tracking verified working
  - No panics in production code
  - Good error test coverage
  - Grade: A- (Excellent)
- ✅ **Task 4.2 Complete + MCP Compliance Fix** (2024-12-14)
  - Added StopReason enum per MCP specification
  - Updated GenerateResponse to require stop_reason field
  - Updated Anthropic and OpenAI providers to map stop reasons
  - Added 6 comprehensive integration tests
  - Tests cover full sampling loops, tool execution, multi-turn conversations, error handling
  - All tests passing (26 tests total)
- ✅ **Task 4.1 Started** (2024-12-14)
  - Added 6 comprehensive tests for ChatLlmSampler with mock providers
  - Tests cover text responses, tool calls, tool execution, request building
  - Updated Input variant handling in database conversions
  - All tests passing (20 tests total)
- ✅ **Phase 3 Complete** (2024-12-14)
- ✅ **Task 3.3 Complete** (2024-12-14)
  - Added LLM provider to ServiceContainer with lazy initialization
  - Provider created based on ChatConfig.initial_model
  - Wired through SamplingIntegration via lazy PlaceholderProvider
  - Supports Gemini and Claude models
- ✅ **Task 3.2 Complete** (2024-12-14)
  - Added Input::ToolCall and Input::ToolResult variants
  - Updated history retention to handle new variants
- ✅ **Task 3.1 Complete** (2024-12-14)
  - Implemented ChatLlmSampler with generate() and execute_tools()
  - Builds requests from ConversationSession
  - Calls LLM provider and tool registry
  - All tests passing (17 tests)
- ✅ **Task 2.3 Complete** (2024-12-14)
  - Enhanced ToolRegistry with tool_definitions() method
- ✅ **Task 2.2 Complete** (2024-12-14)
  - Updated SamplingCoordinator to use ToolRegistry
  - All tests passing (21 tests)
- ✅ **Task 2.1 Complete** (2024-12-14)
  - LlmSampler trait refactored with generate(), sample(), execute_tools()
  - Default implementation provided
  - Error types added
  - All tests passing (14 tests)
- ✅ **Phase 1 Complete** (2024-12-14)
  - Provider abstraction layer
  - Conversation modeling
  - Old types removed and replaced
  - All tests passing (55 tests)

### Not Started
- ⏳ **Phase 4: Testing & Polish**

---

## Vision: What We're Building

A flexible, observable, testable LLM sampling system where:
- Any LLM provider can be used interchangeably
- Tool execution is transparent and debuggable
- Multi-turn conversations are cleanly modeled
- Components are independently testable
- Extensions (streaming, resumption) are natural additions

---

## Target Architecture

### Layer 1: Provider Abstraction (botticelli_core)

**Create a unified interface for all LLM providers:**

```rust
// crates/botticelli_core/src/provider.rs

use crate::{GenerateRequest, GenerateResponse};
use async_trait::async_trait;

/// Unified interface for LLM providers.
///
/// All provider clients (Anthropic, OpenAI, Gemini, etc.) implement this trait,
/// allowing them to be used interchangeably.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a response from the provider.
    async fn generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError>;

    /// Get the provider name for logging/debugging.
    fn provider_name(&self) -> &str;

    /// Get the default model name.
    fn default_model(&self) -> &str;

    /// Check if this provider supports tool calling.
    fn supports_tools(&self) -> bool {
        true  // Most modern providers do
    }
}

/// Errors from provider operations.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Provider {}: {} at {}:{}", provider, kind, file, line)]
pub struct ProviderError {
    pub provider: String,
    pub kind: ProviderErrorKind,
    pub line: u32,
    pub file: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ProviderErrorKind {
    #[display("API error: {}", _0)]
    ApiError(String),

    #[display("Authentication failed")]
    AuthenticationFailed,

    #[display("Rate limit exceeded")]
    RateLimitExceeded,

    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),

    #[display("Response parsing failed: {}", _0)]
    ParsingError(String),
}

impl ProviderError {
    #[track_caller]
    pub fn new(provider: impl Into<String>, kind: ProviderErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            provider: provider.into(),
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}
```

**Refactor GenerateResponse to cleanly separate content from control flow:**

```rust
// crates/botticelli_core/src/response.rs

/// Response from LLM generation.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct GenerateResponse {
    /// Content generated by the model
    content: Vec<ContentBlock>,

    /// Tool calls requested by the model (if any)
    tool_calls: Vec<ToolCall>,

    /// Why generation stopped
    stop_reason: StopReason,

    /// Token usage information
    usage: Option<TokenUsageData>,

    /// Provider-specific metadata
    metadata: ResponseMetadata,
}

/// A block of content in the response.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    /// Text content
    Text(String),

    /// Image content
    Image { mime: String, data: Vec<u8> },

    /// Audio content
    Audio { mime: String, data: Vec<u8> },

    /// Structured JSON
    Json(serde_json::Value),

    /// Note: NO ToolCalls variant - those are separate
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StopReason {
    /// Model naturally ended its response
    EndTurn,

    /// Hit max tokens limit
    MaxTokens,

    /// Model wants to use tools
    ToolUse,

    /// Hit stop sequence
    StopSequence,

    /// Provider-specific reason
    Other,
}

/// Provider-specific metadata.
#[derive(Debug, Clone, Default)]
pub struct ResponseMetadata {
    /// Response ID from provider
    pub id: Option<String>,

    /// Model that generated the response
    pub model: Option<String>,

    /// Additional provider-specific data
    pub extra: serde_json::Value,
}

impl GenerateResponse {
    /// Get all text content concatenated.
    pub fn text(&self) -> String {
        self.content.iter()
            .filter_map(|block| match block {
                ContentBlock::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if response has any tool calls.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Check if response has any content.
    pub fn has_content(&self) -> bool {
        !self.content.is_empty()
    }

    /// Create builder for GenerateResponse.
    pub fn builder() -> GenerateResponseBuilder {
        GenerateResponseBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct GenerateResponseBuilder {
    content: Vec<ContentBlock>,
    tool_calls: Vec<ToolCall>,
    stop_reason: Option<StopReason>,
    usage: Option<TokenUsageData>,
    metadata: ResponseMetadata,
}

impl GenerateResponseBuilder {
    pub fn content(mut self, blocks: Vec<ContentBlock>) -> Self {
        self.content = blocks;
        self
    }

    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.content.push(ContentBlock::Text(text.into()));
        self
    }

    pub fn tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        self.tool_calls = calls;
        self
    }

    pub fn stop_reason(mut self, reason: StopReason) -> Self {
        self.stop_reason = Some(reason);
        self
    }

    pub fn usage(mut self, usage: TokenUsageData) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn build(self) -> GenerateResponse {
        GenerateResponse {
            content: self.content,
            tool_calls: self.tool_calls,
            stop_reason: self.stop_reason.unwrap_or(StopReason::EndTurn),
            usage: self.usage,
            metadata: self.metadata,
        }
    }
}
```

**Migration Strategy for GenerateResponse:**

The current `GenerateResponse` has `outputs: Vec<Output>` where `Output` is an enum. We need to:

1. Keep old `Output` enum for backwards compatibility (deprecated)
2. Add new fields to `GenerateResponse`: `content`, `tool_calls`, `stop_reason`
3. Implement conversion: old `outputs` → new fields
4. Gradually migrate code to use new fields
5. Remove old fields in v2.0

```rust
// Backwards compatibility implementation
impl GenerateResponse {
    /// DEPRECATED: Use content() and tool_calls() instead.
    #[deprecated(since = "0.8.0", note = "Use content() and tool_calls()")]
    pub fn outputs(&self) -> Vec<Output> {
        let mut outputs = Vec::new();

        // Convert content blocks to Output
        for block in &self.content {
            outputs.push(match block {
                ContentBlock::Text(s) => Output::Text(s.clone()),
                ContentBlock::Image { mime, data } => Output::Image {
                    mime: Some(mime.clone()),
                    data: data.clone(),
                },
                // ... other conversions
            });
        }

        // Add tool calls as Output variant
        if !self.tool_calls.is_empty() {
            outputs.push(Output::ToolCalls(self.tool_calls.clone()));
        }

        outputs
    }
}
```

---

### Layer 2: Conversation Modeling (botticelli_mcp)

**Clean separation of conversation elements:**

```rust
// crates/botticelli_mcp/src/conversation.rs

use botticelli_core::{GenerateRequest, GenerateResponse, ToolCall};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A multi-turn conversation session.
#[derive(Debug, Clone)]
pub struct ConversationSession {
    /// Unique session ID
    pub id: String,

    /// System prompt (context for all turns)
    pub system_prompt: String,

    /// Conversation turns in order
    pub turns: Vec<ConversationTurn>,

    /// Current session state
    pub state: SessionState,

    /// Maximum turns allowed (prevent infinite loops)
    pub max_turns: usize,
}

impl ConversationSession {
    /// Create a new conversation session.
    pub fn new(system_prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            system_prompt: system_prompt.into(),
            turns: Vec::new(),
            state: SessionState::Active,
            max_turns: 50,
        }
    }

    /// Add a turn to the conversation.
    pub fn add_turn(&mut self, turn: ConversationTurn) {
        self.turns.push(turn);

        // Check if we've exceeded max turns
        if self.turns.len() >= self.max_turns {
            self.state = SessionState::MaxTurnsExceeded;
        }
    }

    /// Get the number of turns.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Check if session is still active.
    pub fn is_active(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
}

/// A single turn in the conversation.
///
/// Each turn represents one discrete action:
/// - User sends a message
/// - Assistant responds with text
/// - Assistant calls tools
/// - Tool results are provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationTurn {
    /// User sent a message
    UserMessage {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attachments: Option<Vec<Attachment>>,
    },

    /// Assistant responded with text content
    AssistantMessage {
        content: String,
    },

    /// Assistant called one or more tools
    AssistantToolCalls {
        /// The tool calls being made
        calls: Vec<ToolCall>,

        /// Optional thinking/reasoning text before tool use
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },

    /// Results from tool execution
    ToolResults {
        /// Results indexed by tool_call_id
        results: Vec<ToolResult>,
    },
}

/// Result from executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this responds to
    pub tool_call_id: String,

    /// Output from the tool
    pub output: serde_json::Value,

    /// Whether this was an error
    pub is_error: bool,

    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Attachment to a user message (image, file, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// State of a conversation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is active and accepting turns
    Active,

    /// Session completed successfully
    Completed,

    /// Session failed with error
    Failed,

    /// Session exceeded maximum turns
    MaxTurnsExceeded,
}
```

---

### Layer 3: Enhanced Sampling Traits (botticelli_mcp)

**Refactor LlmSampler to provide both low and high-level APIs:**

```rust
// crates/botticelli_mcp/src/sampling.rs

use crate::{ConversationSession, ConversationTurn, ToolResult};
use botticelli_core::{GenerateRequest, GenerateResponse, LlmProvider};
use async_trait::async_trait;

/// Trait for LLM sampling with tool support.
///
/// Provides both low-level (single generation) and high-level (full session)
/// interfaces for maximum flexibility.
#[async_trait]
pub trait LlmSampler: Send + Sync {
    /// Low-level: Generate a single response with optional tools.
    ///
    /// This is the core primitive. The high-level `sample()` method
    /// is built on top of this by calling it in a loop.
    async fn generate(
        &self,
        session: &ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError>;

    /// High-level: Run a complete sampling session.
    ///
    /// Starts with the initial session state and runs until:
    /// - The model stops calling tools (returns text)
    /// - Maximum turns reached
    /// - Error occurs
    ///
    /// Default implementation uses generate() in a loop, but can be
    /// overridden for custom behavior (streaming, custom termination, etc.)
    async fn sample(
        &self,
        session: &mut ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<SamplingResult, SamplingError> {
        while session.is_active() {
            // Generate next response
            let response = self.generate(session, available_tools).await?;

            // Process response
            match response.stop_reason() {
                StopReason::ToolUse if response.has_tool_calls() => {
                    // Model wants to use tools
                    let thinking = if response.has_content() {
                        Some(response.text())
                    } else {
                        None
                    };

                    session.add_turn(ConversationTurn::AssistantToolCalls {
                        calls: response.tool_calls().clone(),
                        thinking,
                    });

                    // Execute tools
                    let results = self.execute_tools(response.tool_calls()).await?;
                    session.add_turn(ConversationTurn::ToolResults { results });

                    // Continue loop for next turn
                }

                _ => {
                    // Model is done (no tool calls)
                    if response.has_content() {
                        session.add_turn(ConversationTurn::AssistantMessage {
                            content: response.text(),
                        });
                    }

                    session.state = SessionState::Completed;
                    return Ok(SamplingResult::Completed {
                        final_response: response.text(),
                    });
                }
            }
        }

        // Session ended without completion
        Err(SamplingError::new(SamplingErrorKind::MaxTurnsExceeded {
            max: session.max_turns,
        }))
    }

    /// Execute tool calls and return results.
    ///
    /// Default implementation returns errors - must be overridden
    /// to provide actual tool execution.
    async fn execute_tools(
        &self,
        _calls: &[ToolCall],
    ) -> Result<Vec<ToolResult>, SamplingError> {
        Err(SamplingError::new(SamplingErrorKind::NoToolRegistry))
    }
}

/// Result of a sampling session.
#[derive(Debug, Clone)]
pub enum SamplingResult {
    /// Session completed successfully with final text
    Completed {
        final_response: String,
    },
}

/// Tool definition for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// JSON Schema for input validation
    pub input_schema: serde_json::Value,
}

/// Errors from sampling operations.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Sampling: {} at {}:{}", kind, file, line)]
pub struct SamplingError {
    pub kind: SamplingErrorKind,
    pub line: u32,
    pub file: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum SamplingErrorKind {
    #[display("Max turns exceeded: {}", max)]
    MaxTurnsExceeded { max: usize },

    #[display("Tool execution failed: {} - {}", tool_name, reason)]
    ToolExecutionFailed { tool_name: String, reason: String },

    #[display("Unknown tool: {}", name)]
    UnknownTool { name: String },

    #[display("Provider error: {}", _0)]
    ProviderError(String),

    #[display("No tool registry configured")]
    NoToolRegistry,

    #[display("Request building failed: {}", _0)]
    RequestBuildingFailed(String),
}

impl SamplingError {
    #[track_caller]
    pub fn new(kind: SamplingErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}
```

---

### Layer 4: Enhanced Tool Registry (botticelli_mcp)

**Add execution and tool definition methods:**

```rust
// crates/botticelli_mcp/src/tools/registry.rs (additions)

use crate::ToolDefinition;
use botticelli_error::{McpError, McpResult};

impl ToolRegistry {
    /// Execute a tool by name with JSON input.
    ///
    /// This is the primary way to invoke tools during sampling.
    #[instrument(skip(self, input), fields(tool = name))]
    pub async fn execute(
        &self,
        name: &str,
        input: &serde_json::Value,
    ) -> McpResult<serde_json::Value> {
        let tool = self.get(name)
            .ok_or_else(|| McpError::tool_not_found(name))?;

        debug!(tool = name, "Executing tool");

        let start = std::time::Instant::now();
        let result = tool.execute(input.clone()).await;
        let duration = start.elapsed();

        // Record metrics if available
        if let Some(metrics) = &self.metrics {
            metrics.record_tool_execution(name, duration, result.is_ok());
        }

        match result {
            Ok(output) => {
                debug!(tool = name, duration_ms = duration.as_millis(), "Tool executed successfully");
                Ok(output)
            }
            Err(e) => {
                error!(tool = name, error = %e, "Tool execution failed");
                Err(e)
            }
        }
    }

    /// Get tool definitions for LLM function calling.
    ///
    /// Converts all registered tools into the format expected by LLMs.
    #[instrument(skip(self))]
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        let definitions: Vec<_> = self.tools.values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
            })
            .collect();

        debug!(count = definitions.len(), "Generated tool definitions");
        definitions
    }

    /// Execute multiple tool calls in parallel.
    ///
    /// Returns results in the same order as calls.
    #[instrument(skip(self, calls))]
    pub async fn execute_batch(
        &self,
        calls: &[ToolCall],
    ) -> Vec<McpResult<serde_json::Value>> {
        use futures::future::join_all;

        let futures = calls.iter()
            .map(|call| self.execute(&call.name(), call.arguments()));

        join_all(futures).await
    }
}
```

---

### Layer 5: Concrete Implementation (botticelli_chat)

**Complete ChatLlmSampler implementation:**

```rust
// crates/botticelli_chat/src/sampling.rs

use botticelli_core::LlmProvider;
use botticelli_mcp::{
    ConversationSession, ConversationTurn, LlmSampler, SamplingError,
    SamplingErrorKind, ToolDefinition, ToolRegistry, ToolResult,
};
use std::sync::Arc;
use tracing::{debug, instrument};

/// LLM sampler implementation for chat system.
pub struct ChatLlmSampler {
    /// LLM provider (Anthropic, OpenAI, etc.)
    provider: Arc<dyn LlmProvider>,

    /// Tool registry for execution
    tool_registry: Arc<ToolRegistry>,
}

impl ChatLlmSampler {
    /// Create new sampler with provider and tool registry.
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            provider,
            tool_registry,
        }
    }

    /// Build GenerateRequest from conversation session.
    fn build_request(
        &self,
        session: &ConversationSession,
    ) -> Result<GenerateRequest, SamplingError> {
        let mut messages = Vec::new();

        // Add all conversation turns as messages
        for turn in &session.turns {
            match turn {
                ConversationTurn::UserMessage { content, .. } => {
                    messages.push(Message::builder()
                        .role(Role::User)
                        .content(vec![Input::Text(content.clone())])
                        .build()
                        .map_err(|e| SamplingError::new(
                            SamplingErrorKind::RequestBuildingFailed(e.to_string())
                        ))?);
                }

                ConversationTurn::AssistantMessage { content } => {
                    messages.push(Message::builder()
                        .role(Role::Assistant)
                        .content(vec![Input::Text(content.clone())])
                        .build()
                        .map_err(|e| SamplingError::new(
                            SamplingErrorKind::RequestBuildingFailed(e.to_string())
                        ))?);
                }

                ConversationTurn::AssistantToolCalls { calls, thinking } => {
                    // Add thinking text if present
                    let mut content = Vec::new();
                    if let Some(text) = thinking {
                        content.push(Input::Text(text.clone()));
                    }

                    // Add tool call blocks
                    // Note: This will need provider-specific formatting
                    // For now, we'll use a generic approach
                    for call in calls {
                        content.push(Input::ToolCall {
                            id: call.id().clone(),
                            name: call.name().clone(),
                            arguments: call.arguments().clone(),
                        });
                    }

                    messages.push(Message::builder()
                        .role(Role::Assistant)
                        .content(content)
                        .build()
                        .map_err(|e| SamplingError::new(
                            SamplingErrorKind::RequestBuildingFailed(e.to_string())
                        ))?);
                }

                ConversationTurn::ToolResults { results } => {
                    // Tool results go back as user messages
                    let content: Vec<_> = results.iter()
                        .map(|result| Input::ToolResult {
                            tool_call_id: result.tool_call_id.clone(),
                            content: result.output.to_string(),
                            is_error: result.is_error,
                        })
                        .collect();

                    messages.push(Message::builder()
                        .role(Role::User)
                        .content(content)
                        .build()
                        .map_err(|e| SamplingError::new(
                            SamplingErrorKind::RequestBuildingFailed(e.to_string())
                        ))?);
                }
            }
        }

        // Build final request
        GenerateRequest::builder()
            .messages(messages)
            .build()
            .map_err(|e| SamplingError::new(
                SamplingErrorKind::RequestBuildingFailed(e)
            ))
    }
}

#[async_trait::async_trait]
impl LlmSampler for ChatLlmSampler {
    #[instrument(skip(self, session, available_tools))]
    async fn generate(
        &self,
        session: &ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError> {
        debug!(
            turn_count = session.turn_count(),
            tool_count = available_tools.len(),
            "Generating next response"
        );

        // Build request from session
        let mut request = self.build_request(session)?;

        // Add system prompt if not already present
        // (implementation detail: some providers need it in request, others in messages)

        // TODO: Add tools to request
        // request = request.with_tools(available_tools);

        // Call provider
        let response = self.provider
            .generate(&request)
            .await
            .map_err(|e| SamplingError::new(
                SamplingErrorKind::ProviderError(e.to_string())
            ))?;

        debug!(
            has_content = response.has_content(),
            has_tools = response.has_tool_calls(),
            stop_reason = ?response.stop_reason(),
            "Received response"
        );

        Ok(response)
    }

    #[instrument(skip(self, calls))]
    async fn execute_tools(
        &self,
        calls: &[ToolCall],
    ) -> Result<Vec<ToolResult>, SamplingError> {
        debug!(count = calls.len(), "Executing tool calls");

        let mut results = Vec::new();

        for call in calls {
            let output = self.tool_registry
                .execute(call.name(), call.arguments())
                .await;

            let result = match output {
                Ok(value) => {
                    debug!(tool = call.name(), "Tool executed successfully");
                    ToolResult {
                        tool_call_id: call.id().clone(),
                        output: value,
                        is_error: false,
                        error_message: None,
                    }
                }
                Err(e) => {
                    error!(tool = call.name(), error = %e, "Tool execution failed");
                    ToolResult {
                        tool_call_id: call.id().clone(),
                        output: serde_json::json!(null),
                        is_error: true,
                        error_message: Some(e.to_string()),
                    }
                }
            };

            results.push(result);
        }

        Ok(results)
    }
}
```

---

## Implementation Phases

### Phase 1: Foundation Refactoring (Week 1)

**Goal:** Establish clean abstractions without breaking existing code

#### Task 1.1: Add LlmProvider Trait
- **File:** `crates/botticelli_core/src/provider.rs` (new)
- **Action:** Create trait and error types (code above)
- **Status:** ✅ COMPLETE (2024-12-14)
- **Acceptance:**
  - [x] Trait compiles
  - [x] Error types follow project conventions
  - [x] Exports added to lib.rs

#### Task 1.2: Implement LlmProvider for Anthropic
- **File:** `crates/botticelli_models/src/anthropic/provider_impl.rs` (new)
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Implemented LlmProvider trait, made conversion methods pub(crate)
  ```rust
  use botticelli_core::{LlmProvider, GenerateRequest, GenerateResponse, ProviderError};
  use async_trait::async_trait;

  #[async_trait]
  impl LlmProvider for AnthropicClient {
      async fn generate(
          &self,
          request: &GenerateRequest,
      ) -> Result<GenerateResponse, ProviderError> {
          // Convert GenerateRequest -> AnthropicRequest
          let anthropic_req = convert_request(request)?;

          // Call existing generate_anthropic method
          let anthropic_resp = self.generate_anthropic(&anthropic_req)
              .await
              .map_err(|e| ProviderError::new("anthropic",
                  ProviderErrorKind::ApiError(e.to_string())))?;

          // Convert AnthropicResponse -> GenerateResponse
          let response = convert_response(anthropic_resp)?;

          Ok(response)
      }

      fn provider_name(&self) -> &str { "anthropic" }
      fn default_model(&self) -> &str { &self.model }
  }

  fn convert_request(req: &GenerateRequest) -> Result<AnthropicRequest, ProviderError> {
      // Extract system prompt from first message if Role::System
      // Convert messages to Anthropic format
      // ... implementation
  }

  fn convert_response(resp: AnthropicResponse) -> Result<GenerateResponse, ProviderError> {
      let mut content = Vec::new();
      let mut tool_calls = Vec::new();

      // Parse response.content (Vec<AnthropicContent>)
      for block in resp.content() {
          match block {
              AnthropicContent::Text { text } => {
                  content.push(ContentBlock::Text(text.clone()));
              }
              AnthropicContent::ToolUse { id, name, input } => {
                  tool_calls.push(ToolCall::new(
                      id.clone(),
                      name.clone(),
                      input.clone(),
                  ));
              }
              // ... other content types
          }
      }

      let stop_reason = match resp.stop_reason() {
          Some("end_turn") => StopReason::EndTurn,
          Some("max_tokens") => StopReason::MaxTokens,
          Some("tool_use") => StopReason::ToolUse,
          _ => StopReason::Other,
      };

      Ok(GenerateResponse::builder()
          .content(content)
          .tool_calls(tool_calls)
          .stop_reason(stop_reason)
          .metadata(ResponseMetadata {
              id: Some(resp.id().clone()),
              model: Some(resp.model().clone()),
              ..Default::default()
          })
          .build())
  }
  ```
- **Acceptance:**
  - [x] Compiles without errors
  - [x] Test created (feature-gated)
  - [x] Uses existing conversion methods
  - [x] All tests passing

#### Task 1.3: Implement LlmProvider for OpenAI-Compatible
- **File:** `crates/botticelli_models/src/openai_compat/provider_impl.rs` (new)
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Implemented LlmProvider for OpenAICompatibleClient
- **Acceptance:**
  - [x] Compiles without errors
  - [x] Tests created for Groq and HuggingFace
  - [x] All tests passing

#### Task 1.4: Add ConversationSession Types
- **File:** `crates/botticelli_mcp/src/conversation.rs` (new)
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Added ConversationSession, ConversationTurn, SessionState, ToolResult, Attachment
- **Refactored:** Deleted old SamplingSession, Turn, ToolResponse types
- **Acceptance:**
  - [x] All types compile
  - [x] Serialization works
  - [x] Basic tests pass (7 tests)
  - [x] Old types removed
  - [x] All code updated to use new types

---

### Phase 2: Sampling Refactor (Week 2)

#### Task 2.1: Refactor LlmSampler Trait
- **File:** `crates/botticelli_mcp/src/tools/sampling.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Added generate() method, provided default sample() implementation with tool execution loop
- **Refactored:** Replaced old sample(system_prompt, user_message) signature with sample(session, tools)
- **Added:** SamplingError, SamplingErrorKind, SamplingResult, ToolDefinition types
- **Acceptance:**
  - [x] Trait compiles with generate(), sample(), execute_tools()
  - [x] Default implementation works (tool execution loop)
  - [x] SamplingCoordinator updated
  - [x] SamplingSessionManager updated
  - [x] All tests passing (14 tests)

#### Task 2.2: Update SamplingCoordinator
- **File:** `crates/botticelli_mcp/src/tools/sampling.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** 
  - Updated SamplingCoordinator to use ToolRegistry instead of NarrativeRegistry
  - Changed constructor to require Arc<ToolRegistry>
  - Updated generate_narrative() and refine_narrative() to call tool_registry.tool_definitions()
  - Added tool_registry() accessor
  - Updated botticelli_chat to create ToolRegistry when instantiating SamplingCoordinator
- **Acceptance:**
  - [x] Compiles
  - [x] Uses ToolRegistry
  - [x] Calls tool_registry.tool_definitions()
  - [x] All tests passing (21 tests)

#### Task 2.3: Enhance ToolRegistry
- **File:** `crates/botticelli_mcp/src/tools/mod.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Added tool_definitions() method to convert registered tools to ToolDefinition format
- **Acceptance:**
  - [x] Method compiles
  - [x] Converts McpTool to ToolDefinition
  - [x] Used by SamplingCoordinator
  ```rust
  pub struct SamplingCoordinator {
      sampler: Arc<dyn LlmSampler>,
      tool_registry: Arc<ToolRegistry>,
  }

  impl SamplingCoordinator {
      pub fn new(
          sampler: Arc<dyn LlmSampler>,
          tool_registry: Arc<ToolRegistry>,
      ) -> Self {
          Self { sampler, tool_registry }
      }

      #[instrument(skip(self))]
      pub async fn generate_narrative(
          &self,
          description: String,
      ) -> Result<PartialNarrative, SamplingError> {
          // Create session
          let mut session = ConversationSession::new(
              SamplingHelper::narrative_generation_prompt()
          );

          session.add_turn(ConversationTurn::UserMessage {
              content: description,
              attachments: None,
          });

          // Get available tools
          let tools = self.tool_registry.tool_definitions();

          // Run sampling
          let result = self.sampler.sample(&mut session, &tools).await?;

          // Extract narrative from final response
          // (either parse TOML from text, or get from tool results)
          let narrative = self.extract_narrative_from_session(&session)?;

          Ok(narrative)
      }
  }
  ```
- **Acceptance:**
  - [x] Compiles
  - [x] Uses ToolRegistry with tool_definitions()
  - [x] Calls sampler.sample()

#### Task 2.3: Enhance ToolRegistry  
- **File:** `crates/botticelli_mcp/src/tools/mod.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Added tool_definitions() method (execute() already existed)
- **Acceptance:**
  - [x] Method compiles
  - [x] Converts McpTool to ToolDefinition
  - [x] Used by SamplingCoordinator

---

### Phase 3: Complete Implementation (Week 3)

#### Task 3.1: Implement ChatLlmSampler
- **File:** `crates/botticelli_chat/src/sampling.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Fully implemented ChatLlmSampler with LLM provider and tool registry
  - Added provider and tool_registry fields
  - Implemented generate() to build requests from ConversationSession and call provider
  - Implemented execute_tools() to call tool registry and return results
  - Added build_request() helper to convert sessions to GenerateRequest
- **Acceptance:**
  - [x] Implements both generate() and execute_tools()
  - [x] Builds requests from sessions correctly
  - [x] Handles tool calls
  - [x] Compiles and tests pass

#### Task 3.2: Add Input Variants for Tool Support
- **File:** `crates/botticelli_core/src/input.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Added Input::ToolCall and Input::ToolResult variants
  - Added ToolCall variant with id, name, arguments
  - Added ToolResult variant with tool_call_id, content, is_error
  - Updated botticelli_narrative to handle new variants in history retention
- **Acceptance:**
  - [x] Variants added
  - [x] Serialization works
  - [x] History retention handles new variants
  - [x] All tests pass

#### Task 3.3: Wire to CommandExecutor
- **File:** `crates/botticelli_chat/src/services.rs`, `sampling_integration.rs`
- **Status:** ✅ COMPLETE (2024-12-14)
- **Action:** Wired LLM provider through ServiceContainer
  - Added llm_provider field to ServiceContainer with lazy initialization
  - Implemented init_llm_provider() to create provider based on ChatConfig.initial_model
  - Supports Gemini (via GOOGLE_API_KEY env) and Claude (via ANTHROPIC_API_KEY env)
  - Updated PlaceholderProvider to lazily delegate to real provider from services
  - Provider initialization deferred until first actual generate() call
- **Acceptance:**
  - [x] Provider initialized from config
  - [x] Wired through SamplingIntegration
  - [x] Supports multiple providers (Gemini, Claude)
  - [x] All tests passing
  ```rust
  impl CommandExecutor {
      pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
          let tool_registry = Arc::new(build_narrative_tool_registry());
          let sampler = Arc::new(ChatLlmSampler::new(
              provider,
              tool_registry.clone(),
          ));
          let coordinator = Arc::new(SamplingCoordinator::new(
              sampler,
              tool_registry,
          ));

          // ... rest of initialization
      }

      async fn execute_narrative_generate(
          &mut self,
          description: String,
      ) -> Result<String, ExecutorError> {
          let narrative = self.coordinator
              .generate_narrative(description)
              .await?;

          self.current_narrative = Some(narrative.clone());

          // Convert to TOML
          let toml = narrative_to_toml(&narrative)?;

          Ok(format!("Generated narrative. Use 'narrative show' to view."))
      }
  }
  ```
- **Acceptance:**
  - [ ] Commands work end-to-end
  - [ ] Narrative generated
  - [ ] TOML output valid

---

### Phase 4: Testing & Polish (Week 4)

#### Task 4.1: Comprehensive Unit Tests
- **Files:** Various test files
- **Coverage:**
  - [ ] Provider implementations
  - [ ] Conversation session
  - [ ] Sampler with mocks
  - [ ] Tool registry
  - [ ] Command executor

#### Task 4.2: Integration Tests
- **File:** `crates/botticelli_chat/tests/sampling_end_to_end_test.rs`
- **Test Flow:**
  1. Create mock provider that returns tool calls
  2. Create session
  3. Run sampling
  4. Verify tools executed
  5. Verify final narrative generated

#### Task 4.3: Error Handling Audit
- [ ] All error paths covered
- [ ] Error messages are helpful
- [ ] Location tracking works
- [ ] No panics in production code

#### Task 4.4: Documentation
- [ ] All public items documented
- [ ] Examples in doc comments
- [ ] Architecture diagram
- [ ] Usage guide

---

## Migration Strategy

### Backwards Compatibility

1. **GenerateResponse:** Keep old `outputs()` method, add new fields
2. **LlmSampler:** Keep old signature, add new method
3. **Deprecation warnings:** Add to old methods
4. **Version:** Mark as 0.8.0, remove deprecated in 1.0.0

### Testing During Migration

1. Run existing tests at each step
2. Add new tests for new functionality
3. Feature flag new code path initially
4. Gradual rollout

---

## Success Criteria

### MVP Complete When:
1. [ ] User can run: `narrative generate "space adventure"`
2. [ ] LLM makes tool calls
3. [ ] Tools execute and return results
4. [ ] LLM generates valid TOML
5. [ ] User can save to file
6. [ ] All tests pass
7. [ ] Zero clippy warnings

### Architecture Quality:
1. [ ] Any provider can be swapped
2. [ ] Conversation turns are cleanly modeled
3. [ ] Tool calls separated from content
4. [ ] Observable intermediate states
5. [ ] Independently testable components
6. [ ] Clear error messages

---

## Next Steps

1. Review this architecture - any concerns or changes?
2. I'll create detailed task breakdowns for Phase 1
3. We'll implement phase by phase with testing at each step
4. Commit after each completed phase

**Ready to proceed with Phase 1?**
