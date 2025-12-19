# Architectural Gaps Resolution Plan

## Overview

This plan addresses critical architectural gaps preventing tool calling from working end-to-end in Botticelli. While Phase 0 successfully connected tools to UnifiedMcpClient, the underlying LLM drivers do not actually support tool calling.

**Status**: Tools are registered and orchestration loop exists, but drivers cannot execute tool calls.

---

## Current State Analysis

### ✅ What Works

1. **Tool Registration** (crates/botticelli_tui/src/state.rs:544-586)
   - Tools properly registered in UnifiedMcpClient.internal_registry
   - 5 internal tools available: echo, create_narrative, validate_narrative, list_narratives, load_narrative
   - Registry accessible via `mcp_client.internal_registry_mut()`

2. **Orchestration Loop** (crates/botticelli_mcp_client/src/unified_client.rs:315-414)
   - `execute_with_tracking` implements agentic loop
   - Calls `backend.generate_with_tools(&conversation, &all_tools)`
   - Extracts tool calls from responses
   - Executes tools and adds results to conversation
   - Iterates until completion or max_iterations

3. **Tool Execution** (crates/botticelli_mcp_client/src/unified_client.rs:200-280)
   - Routes to internal registry or external servers
   - Proper error handling and metrics tracking
   - Approval flow for sensitive operations

### ❌ Critical Gaps

#### Gap 1: No ToolUse Trait Implementation

**Location**: All driver implementations (anthropic, gemini, ollama, groq, huggingface)

**Problem**:
```rust
// crates/botticelli_interface/src/traits.rs:169-192
pub trait ToolUse: BotticelliDriver {
    async fn generate_with_tools(
        &self,
        req: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse>;
    // ...
}

// ❌ NO implementations exist!
// $ find crates/botticelli_models -name "*.rs" -exec grep -l "impl.*ToolUse" {} \;
// (no results)
```

**Impact**: LLMs cannot be told about available tools, so they cannot decide to call them.

---

#### Gap 2: TuiLlmBackend Ignores Tools Parameter

**Location**: crates/botticelli_tui/src/state.rs:37-106

**Problem**:
```rust
#[async_trait]
impl LlmBackend for TuiLlmBackend {
    async fn generate_with_tools(
        &self,
        messages: &[botticelli_core::Message],
        _tools: &[ToolDefinition],  // ❌ Ignored
    ) -> Result<String, Box<dyn std::error::Error>> {
        // NOTE: Current architecture limitation - BotticelliDriver trait doesn't expose tool definitions
        // We'll need to refactor this to properly support tool calling

        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .build()?;

        let response = self.driver.generate(&request).await?;  // ❌ No tools passed
```

**Impact**: Even with ToolUse implemented, TuiLlmBackend wouldn't use it.

---

#### Gap 3: No Tool Schema Conversion

**Problem**: ToolDefinition (MCP format) needs conversion to provider-specific schemas:

- **Anthropic**: `{ "name": "...", "description": "...", "input_schema": {...} }`
- **Gemini**: `{ "name": "...", "description": "...", "parameters": {...} }`
- **OpenAI**: `{ "type": "function", "function": {...} }`

**Impact**: Cannot pass tool definitions to APIs in correct format.

---

#### Gap 4: No Tool Call Parsing from API Responses

**Problem**: Need to extract tool calls from provider-specific response formats:

- **Anthropic**: `content[].type == "tool_use"` with `id`, `name`, `input`
- **Gemini**: `functionCall` with `name`, `args`
- **OpenAI**: `tool_calls[]` with `id`, `function.name`, `function.arguments`

**Current Workaround** (state.rs:56-96): TuiLlmBackend manually converts botticelli_core::Output::ToolCalls to Anthropic JSON format.

**Impact**: Fragile, provider-specific, duplicates conversion logic.

---

## Resolution Plan

### Phase 1: Implement ToolUse for AnthropicClient

**Goal**: Make AnthropicClient properly support tool calling.

#### Task 1.1: Add Anthropic Tool Schema Types

**File**: `crates/botticelli_models/src/anthropic/schema.rs` (new)

```rust
use serde::{Deserialize, Serialize};

/// Anthropic tool definition schema.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

impl AnthropicTool {
    /// Convert MCP ToolDefinition to Anthropic format.
    pub fn from_mcp_tool(tool: &botticelli_mcp_client::ToolDefinition) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.input_schema().clone(),
        }
    }
}

/// Anthropic tool use content block (in responses).
#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct AnthropicToolUse {
    #[serde(rename = "type")]
    type_: String,  // Always "tool_use"
    id: String,
    name: String,
    input: serde_json::Value,
}
```

**Success Criteria**:
- [ ] Types compile with proper derives
- [ ] Serde serialization/deserialization works
- [ ] Unit test: convert sample ToolDefinition to AnthropicTool
- [ ] Unit test: deserialize Anthropic API tool_use response

---

#### Task 1.2: Update AnthropicRequest for Tools

**File**: `crates/botticelli_models/src/anthropic/request.rs`

**Action**: Add optional `tools` field to AnthropicRequest:

```rust
#[derive(Debug, Clone, Serialize, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,  // ← NEW
}
```

**Success Criteria**:
- [ ] Field added with proper derives
- [ ] Serde skips field when None
- [ ] Unit test: serialize request with tools

---

#### Task 1.3: Update AnthropicResponse for Tool Calls

**File**: `crates/botticelli_models/src/anthropic/response.rs`

**Action**: Support mixed content blocks (text + tool_use):

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize, derive_getters::Getters)]
pub struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContentBlock>,  // ← Now enum instead of just text
    stop_reason: String,
    usage: Option<AnthropicUsage>,
}
```

**Success Criteria**:
- [ ] Enum compiles with serde tag
- [ ] Unit test: deserialize response with text only
- [ ] Unit test: deserialize response with tool_use
- [ ] Unit test: deserialize response with mixed content

---

#### Task 1.4: Implement ToolUse Trait for AnthropicClient

**File**: `crates/botticelli_models/src/anthropic/client.rs`

**Action**:

```rust
use botticelli_interface::ToolUse;
use botticelli_mcp_client::ToolDefinition;

#[async_trait::async_trait]
impl ToolUse for AnthropicClient {
    #[instrument(skip(self, req, tools), fields(tool_count = tools.len()))]
    async fn generate_with_tools(
        &self,
        req: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse> {
        debug!(tool_count = tools.len(), "Generating with tools");

        // Convert request to Anthropic format
        let mut anthropic_request = self.convert_request(req)?;

        // Convert tools to Anthropic format
        let anthropic_tools: Vec<AnthropicTool> = tools
            .iter()
            .map(|t| AnthropicTool::from_mcp_tool(t))
            .collect();

        anthropic_request.with_tools(Some(anthropic_tools));

        // Call API
        let response = self.generate_anthropic(&anthropic_request).await?;

        // Convert response (now handles tool calls)
        Self::convert_response_with_tools(&response)
    }

    fn max_tools(&self) -> usize {
        64  // Anthropic's documented limit
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true  // Anthropic supports multiple tool calls in one response
    }
}
```

**Success Criteria**:
- [ ] Compiles without errors
- [ ] #[instrument] span includes tool count
- [ ] Calls convert_request and convert_response_with_tools
- [ ] Returns proper BotticelliResult

---

#### Task 1.5: Implement convert_response_with_tools

**File**: `crates/botticelli_models/src/anthropic/client.rs`

**Action**:

```rust
impl AnthropicClient {
    /// Converts Anthropic response to Botticelli format (with tool call support).
    #[instrument(skip(response))]
    pub(crate) fn convert_response_with_tools(
        response: &AnthropicResponse,
    ) -> Result<GenerateResponse, ModelsError> {
        debug!("Converting AnthropicResponse to GenerateResponse with tool support");

        let mut outputs = Vec::new();

        for content_block in response.content() {
            match content_block {
                AnthropicContentBlock::Text { text } => {
                    outputs.push(Output::Text(text.clone()));
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    let tool_call = ToolCallBuilder::default()
                        .id(id.clone())
                        .name(name.clone())
                        .arguments(input.clone())
                        .build()
                        .map_err(|e| ModelsError::new(
                            AnthropicErrorKind::Parse(format!("Failed to build tool call: {}", e)).into()
                        ))?;
                    outputs.push(Output::ToolCalls(vec![tool_call]));
                }
            }
        }

        // Map stop_reason
        let stop_reason = match response.stop_reason().as_str() {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "tool_use" => StopReason::ToolUse,
            "stop_sequence" => StopReason::StopSequence,
            _ => StopReason::Other,
        };

        // Extract usage
        let usage = response.usage().as_ref().map(|u| {
            TokenUsageData::new(
                *u.input_tokens() as u64,
                *u.output_tokens() as u64,
                (*u.input_tokens() + *u.output_tokens()) as u64,
            )
        });

        GenerateResponse::builder()
            .outputs(outputs)
            .stop_reason(stop_reason)
            .usage(usage)
            .build()
            .map_err(|e| ModelsError::new(AnthropicErrorKind::Builder(e.to_string()).into()))
    }
}
```

**Success Criteria**:
- [ ] Handles text-only responses
- [ ] Handles tool_use responses
- [ ] Handles mixed content
- [ ] Proper error handling with ModelsError
- [ ] Unit test: convert text response
- [ ] Unit test: convert tool_use response
- [ ] Unit test: convert mixed response

---

#### Task 1.6: Integration Test

**File**: `crates/botticelli_models/tests/anthropic_tool_calling_test.rs` (new)

```rust
use botticelli_core::{GenerateRequest, Message, MessageBuilder, Role, Input};
use botticelli_interface::{BotticelliDriver, ToolUse};
use botticelli_mcp_client::ToolDefinition;
use botticelli_models::AnthropicClient;
use serde_json::json;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_anthropic_tool_calling() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");
    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    // Define simple echo tool
    let tool = ToolDefinition::new(
        "echo".to_string(),
        "Echoes back the input message".to_string(),
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to echo back"
                }
            },
            "required": ["message"]
        }),
    );

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Please use the echo tool to say 'Hello, tools!'".to_string())])
        .build()
        .expect("Valid message");

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .build()
        .expect("Valid request");

    let response = client.generate_with_tools(&request, &[tool]).await.expect("API call succeeded");

    // Should contain tool call
    let has_tool_call = response.outputs().iter().any(|output| matches!(output, botticelli_core::Output::ToolCalls(_)));
    assert!(has_tool_call, "Response should contain tool call");
}
```

**Success Criteria**:
- [ ] Test passes with real Anthropic API (feature = "api")
- [ ] Response contains ToolCalls output
- [ ] Uses <50 tokens (rate limit conservation)

---

### Phase 2: Fix TuiLlmBackend to Use ToolUse

**Goal**: Make TuiLlmBackend actually use the ToolUse trait.

#### Task 2.1: Check for ToolUse Support in TuiLlmBackend

**File**: `crates/botticelli_tui/src/state.rs`

**Action**: Replace the current implementation with trait-aware code:

```rust
use botticelli_interface::ToolUse;
use std::any::Any;

#[async_trait]
impl LlmBackend for TuiLlmBackend {
    #[instrument(skip(self, messages, tools), fields(tool_count = tools.len()))]
    async fn generate_with_tools(
        &self,
        messages: &[botticelli_core::Message],
        tools: &[ToolDefinition],
    ) -> Result<String, Box<dyn std::error::Error>> {
        use botticelli_core::Output;
        use serde_json::json;

        // Build request
        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .build()?;

        // Check if driver implements ToolUse trait
        let driver_any = &self.driver as &dyn Any;
        let response = if let Some(tool_driver) = driver_any.downcast_ref::<dyn ToolUse>() {
            // Driver supports tools - use generate_with_tools
            debug!("Driver supports ToolUse trait, using generate_with_tools");
            tool_driver.generate_with_tools(&request, tools).await?
        } else {
            // Fallback to basic generate
            debug!("Driver does not support ToolUse trait, falling back to generate");
            warn!("Tool calling requested but driver does not implement ToolUse trait");
            self.driver.generate(&request).await?
        };

        // Convert response to JSON format expected by extract_tool_calls
        let mut content_array = Vec::new();

        for output in response.outputs() {
            match output {
                Output::Text(text) => {
                    content_array.push(json!({
                        "type": "text",
                        "text": text
                    }));
                }
                Output::ToolCalls(calls) => {
                    for call in calls {
                        content_array.push(json!({
                            "type": "tool_use",
                            "id": call.id(),
                            "name": call.name(),
                            "input": call.arguments()
                        }));
                    }
                }
                _ => {
                    debug!("Skipping non-text/non-tool output");
                }
            }
        }

        Ok(serde_json::to_string(&json!({ "content": content_array }))?)
    }
}
```

**Problem with this approach**: `Any` downcasting doesn't work well with trait objects.

**Better approach**: Use a feature check method or wrapper type:

```rust
#[async_trait]
impl LlmBackend for TuiLlmBackend {
    async fn generate_with_tools(
        &self,
        messages: &[botticelli_core::Message],
        tools: &[ToolDefinition],
    ) -> Result<String, Box<dyn std::error::Error>> {
        use botticelli_core::Output;
        use serde_json::json;

        // Build request
        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .build()?;

        // Try to use tools if available (requires driver to implement ToolUse)
        // For now, we'll use a helper method that checks at compile time
        let response = self.try_generate_with_tools(&request, tools).await?;

        // Convert to JSON format for extract_tool_calls
        // ... (same conversion logic as before)
    }
}

impl TuiLlmBackend {
    /// Attempts to generate with tools if the driver supports it.
    ///
    /// This is a workaround for the lack of trait upcasting in Rust.
    async fn try_generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, Box<dyn std::error::Error>> {
        // For now, assume all drivers will eventually implement ToolUse
        // This will be a compile-time error if they don't
        // TODO: Make this more dynamic when trait upcasting stabilizes

        // Cast to Arc<dyn ToolUse> - this will fail at compile time if driver doesn't implement it
        // We'll need to update this when we add support for multiple driver types

        // TEMPORARY: Just use basic generate until we can properly detect ToolUse support
        warn!("ToolUse support detection not yet implemented, using basic generate");
        Ok(self.driver.generate(request).await?)
    }
}
```

**Actual Best Solution**: Change TuiLlmBackend to require ToolUse:

```rust
pub struct TuiLlmBackend {
    driver: Arc<dyn ToolUse>,  // ← Require ToolUse instead of just BotticelliDriver
}

impl TuiLlmBackend {
    pub fn new(driver: Arc<dyn ToolUse>) -> Self {
        Self { driver }
    }
}

#[async_trait]
impl LlmBackend for TuiLlmBackend {
    async fn generate_with_tools(
        &self,
        messages: &[botticelli_core::Message],
        tools: &[ToolDefinition],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .build()?;

        // Driver is guaranteed to support ToolUse
        let response = self.driver.generate_with_tools(&request, tools).await?;

        // Convert to JSON... (same as before)
    }
}
```

**Success Criteria**:
- [ ] TuiLlmBackend actually calls generate_with_tools on driver
- [ ] Compiles with proper trait bounds
- [ ] Unit test: verify tools are passed to driver

---

#### Task 2.2: Update AppState::with_mcp_integration

**File**: `crates/botticelli_tui/src/state.rs`

**Action**: Update to require ToolUse trait:

```rust
impl AppState {
    pub fn with_mcp_integration(driver: Arc<dyn ToolUse>) -> Self {  // ← Changed
        info!("Initializing AppState with MCP integration");

        let llm_backend = TuiLlmBackend::new(driver);
        // ... rest stays the same
    }
}
```

**Success Criteria**:
- [ ] Compiles with new trait bound
- [ ] Type signature enforces ToolUse requirement

---

#### Task 2.3: Update TuiApp and Binary Entry Points

**Files**:
- `crates/botticelli_tui/src/app.rs`
- `crates/botticelli_tui/src/tui.rs`
- `crates/botticelli_tui/src/bin/tui.rs`
- `crates/botticelli_tui/examples/chat_with_mcp.rs`

**Action**: Update all entry points to use `Arc<dyn ToolUse>`:

```rust
// app.rs
impl TuiApp {
    pub fn new(driver: Arc<dyn ToolUse>) -> TuiResult<Self> {  // ← Changed
        // ... rest stays the same
    }
}

// bin/tui.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...
    let driver: Arc<dyn ToolUse> = Arc::new(AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022"));
    let mut app = TuiApp::new(driver)?;
    app.run().await?;
    Ok(())
}
```

**Success Criteria**:
- [ ] All files compile
- [ ] Binary runs successfully
- [ ] No trait object errors

---

### Phase 3: Extend to Other Providers

**Goal**: Implement ToolUse for Gemini, OpenAI, Ollama, Groq.

#### Task 3.1: Implement ToolUse for GeminiClient

**Files**:
- `crates/botticelli_models/src/gemini/schema.rs` (new - tool schemas)
- `crates/botticelli_models/src/gemini/client.rs` (update)

**Pattern**: Follow same approach as Anthropic (Phase 1)

1. Define GeminiTool schema
2. Update GeminiRequest with tools field
3. Update GeminiResponse to handle functionCall
4. Implement ToolUse trait
5. Implement convert_response_with_tools
6. Add integration test

**Success Criteria**:
- [ ] GeminiClient implements ToolUse
- [ ] Integration test passes with real API
- [ ] Supports Gemini function calling format

---

#### Task 3.2: Implement ToolUse for OllamaClient (Optional)

**Note**: Ollama tool support varies by model. May need to check model capabilities first.

**Success Criteria**:
- [ ] Tool calling works with compatible Ollama models
- [ ] Graceful fallback for incompatible models

---

#### Task 3.3: Implement ToolUse for GroqClient (If Applicable)

**Success Criteria**:
- [ ] Groq tool calling supported (check API docs for availability)

---

### Phase 4: Testing and Validation

**Goal**: Ensure tool calling works end-to-end.

#### Task 4.1: End-to-End Integration Test

**File**: `crates/botticelli_tui/tests/tool_calling_integration_test.rs` (new)

```rust
//! Integration test for full tool calling flow in TUI.

use botticelli_mcp_client::{UnifiedMcpClient, LlmBackend, ToolDefinition};
use botticelli_models::AnthropicClient;
use botticelli_tui::TuiLlmBackend;
use botticelli_core::{Message, MessageBuilder, Role, Input};
use std::sync::Arc;
use serde_json::json;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_end_to_end_tool_calling() {
    // Setup
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");
    let driver = Arc::new(AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022"));
    let backend = TuiLlmBackend::new(driver);

    // Create UnifiedMcpClient with echo tool
    let mut mcp_client = UnifiedMcpClient::builder().max_iterations(5).build();
    let registry = mcp_client.internal_registry_mut();

    registry.register(
        "echo".to_string(),
        Arc::new(botticelli_mcp::tools::EchoTool),
    ).expect("Register echo tool");

    // Create conversation
    let messages = vec![
        MessageBuilder::default()
            .role(Role::User)
            .content(vec![Input::Text("Use the echo tool to say 'Integration test successful!'".to_string())])
            .build()
            .expect("Valid message"),
    ];

    // Execute with tracking
    let result = mcp_client.execute_with_tracking(&backend, messages).await.expect("Execution succeeded");

    // Verify
    assert!(result.iterations > 0, "Should have executed at least one iteration");
    assert!(!result.tool_calls.is_empty(), "Should have made tool calls");
    assert_eq!(result.tool_calls[0].tool_name, "echo", "Should have called echo tool");
    assert!(result.tool_calls[0].success, "Tool call should have succeeded");
}
```

**Success Criteria**:
- [ ] Test passes with real API
- [ ] Tool is actually called by LLM
- [ ] Result is properly tracked
- [ ] Uses <100 tokens total

---

#### Task 4.2: Manual Testing Checklist

**Scenarios**:
1. [ ] Run TUI with Anthropic driver → Claude can call narrative tools
2. [ ] Ask Claude to "list all narratives" → uses list_narratives tool
3. [ ] Ask Claude to "create a new narrative called test" → uses create_narrative tool
4. [ ] Verify tool results appear in conversation
5. [ ] Check logs show tool execution spans
6. [ ] Test error handling: invalid tool arguments
7. [ ] Test max_iterations limit

---

### Phase 5: Documentation and Cleanup

#### Task 5.1: Update Architecture Documentation

**Files**:
- `TUI_ARCHITECTURE_ANALYSIS.md` - mark Phase 0 gaps as resolved
- `README.md` - add tool calling usage examples
- `crates/botticelli_interface/README.md` - document ToolUse trait

**Success Criteria**:
- [ ] Documentation reflects current architecture
- [ ] Examples show how to implement ToolUse for new providers
- [ ] Known limitations documented

---

#### Task 5.2: Remove Workarounds and TODOs

**Action**: Search codebase for:
- `// TODO: Add proper tool schema conversion`
- `// NOTE: Current architecture limitation`
- `_tools` (underscore-prefixed parameters that should be used)

**Success Criteria**:
- [ ] All TODOs addressed or converted to GitHub issues
- [ ] No ignored parameters in tool-related code
- [ ] No architecture limitation comments remain

---

## Implementation Order

1. **Phase 1 (Anthropic ToolUse)** - 6 tasks - Core functionality
2. **Phase 2 (TuiLlmBackend)** - 3 tasks - Wire up the backend
3. **Phase 4.1 (E2E Test)** - 1 task - Validate it works
4. **Phase 3 (Other Providers)** - 3 tasks - Expand support
5. **Phase 4.2 (Manual Testing)** - 1 checklist - User validation
6. **Phase 5 (Documentation)** - 2 tasks - Polish

**Estimated Effort**: 5-7 focused sessions

---

## Success Metrics

### Functional Requirements
- [ ] AnthropicClient implements ToolUse trait
- [ ] TuiLlmBackend uses ToolUse.generate_with_tools
- [ ] Tool calling works end-to-end in TUI
- [ ] Integration tests passing
- [ ] Zero compilation errors/warnings

### Observability
- [ ] Tool call attempts visible in tracing spans
- [ ] Tool execution results logged
- [ ] Error messages clear and actionable

### Code Quality
- [ ] All public functions have #[instrument]
- [ ] Proper error types with derive_more
- [ ] No #[allow] directives
- [ ] Tests in tests/ directory, not inline

---

## Rollback Plan

If issues arise:
- **Phase 1 only**: Easy rollback (new trait impl, no breaking changes)
- **Phase 2+**: Temporarily keep old TuiLlmBackend alongside new one with feature flag
- **Emergency**: Revert to basic generate() without tool support

---

## Notes

- All implementations follow CLAUDE.md guidelines (builders, derive_more, tracing, etc.)
- Tool schema conversion uses standard JSON Schema format (no OpenAPI-specific features)
- Provider-specific formats handled in driver implementations (encapsulation)
- TUI remains provider-agnostic - only depends on ToolUse trait

---

## Related Documents

- `TUI_ARCHITECTURE_ANALYSIS.md` - Initial gap analysis (now partially outdated)
- `LLM_FALLBACK_IMPLEMENTATION_PLAN.md` - Deferred until tool calling works
- `MCP_CLIENT_MIGRATION_PLAN.md` - Context for UnifiedMcpClient design
- `CLAUDE.md` - Code standards and patterns
