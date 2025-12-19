# Architectural Gaps Resolution Plan (v2)

## Overview

Fix tool calling by extending `GenerateRequest` with tools field. Each driver handles tools in its existing `generate()` method - no separate ToolUse trait needed.

**Key Insight**: `GenerateRequest` is already the abstraction layer. We don't need a parallel trait hierarchy.

---

## Current State Analysis

### ✅ What Works

1. **Tool Registration** - Tools connected to UnifiedMcpClient.internal_registry
2. **Orchestration Loop** - `execute_with_tracking` implements agentic workflow
3. **Tool Execution** - Registry routes to internal/external tools

### ❌ Critical Gaps

1. **GenerateRequest has no tools field** - Can't pass tool definitions to drivers
2. **Drivers don't handle tools** - No conversion logic exists
3. **No tool call parsing** - Drivers don't extract tool calls from API responses
4. **TuiLlmBackend workaround** - Manually converts Output::ToolCalls to JSON

---

## Resolution Strategy

### Core Principle

**Tools are just another optional field in GenerateRequest**, like `temperature` or `max_tokens`. Each driver:
1. Checks if `req.tools()` is `Some(...)`
2. Converts MCP ToolDefinition → provider-specific format
3. Sends to API with tools
4. Parses tool calls from response
5. Returns as `Output::ToolCalls` in GenerateResponse

No separate trait needed. Clean, consistent, scalable.

---

## Phase 1: Core Type Changes

### Task 1.1: Move ToolDefinition to botticelli_core

**Rationale**: ToolDefinition is now part of core request type, should live in core crate.

**File**: `crates/botticelli_core/src/tool_definition.rs` (new)

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP-compliant tool definition.
///
/// Describes a tool that can be called by the LLM during generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, derive_getters::Getters)]
pub struct ToolDefinition {
    /// Tool name (must be unique in the registry)
    name: String,
    /// Human-readable description
    description: String,
    /// JSON Schema for tool input parameters
    input_schema: Value,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(name: String, description: String, input_schema: Value) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }
}
```

**Success Criteria**:
- [ ] Types compile with all derives
- [ ] Re-exported from botticelli_core::lib
- [ ] Unit test: create and serialize ToolDefinition

---

### Task 1.2: Add tools Field to GenerateRequest

**File**: `crates/botticelli_core/src/request.rs`

**Action**:

```rust
use crate::ToolDefinition;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct GenerateRequest {
    messages: Vec<Message>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    model: Option<String>,
    tools: Option<Vec<ToolDefinition>>,  // ← NEW
}

impl GenerateRequestBuilder {
    // ... existing methods ...

    /// Sets the available tools for the LLM to call.
    pub fn tools(mut self, tools: Option<Vec<ToolDefinition>>) -> Self {
        self.tools = tools;
        self
    }

    pub fn build(self) -> Result<GenerateRequest, String> {
        Ok(GenerateRequest {
            messages: self.messages.ok_or("messages is required")?,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            model: self.model,
            tools: self.tools,  // ← NEW
        })
    }
}
```

**Success Criteria**:
- [ ] Compiles with proper derives
- [ ] Serde serialization includes/excludes tools appropriately
- [ ] Builder pattern works with tools
- [ ] Unit test: build request with tools

---

### Task 1.3: Update botticelli_core Exports

**File**: `crates/botticelli_core/src/lib.rs`

```rust
mod tool_definition;

pub use tool_definition::ToolDefinition;
```

**Success Criteria**:
- [ ] ToolDefinition accessible as `botticelli_core::ToolDefinition`
- [ ] No circular dependencies
- [ ] `just check botticelli_core` passes

---

### Task 1.4: Update botticelli_mcp_client to Re-export from Core

**File**: `crates/botticelli_mcp_client/src/lib.rs`

**Action**: Remove local ToolDefinition, re-export from core:

```rust
// Re-export from core for backward compatibility
pub use botticelli_core::ToolDefinition;
```

**Success Criteria**:
- [ ] No breaking changes in botticelli_mcp_client API
- [ ] All existing code compiles
- [ ] Tests pass

---

## Phase 2: Implement Tool Support in AnthropicClient

### Task 2.1: Add Anthropic Tool Schema Types

**File**: `crates/botticelli_models/src/anthropic/schema.rs` (new)

```rust
use botticelli_core::ToolDefinition;
use serde::{Deserialize, Serialize};

/// Anthropic tool definition format.
#[derive(Debug, Clone, Serialize, derive_getters::Getters)]
pub struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

impl AnthropicTool {
    /// Convert MCP ToolDefinition to Anthropic format.
    pub fn from_mcp(tool: &ToolDefinition) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.input_schema().clone(),
        }
    }
}
```

**File**: `crates/botticelli_models/src/anthropic/mod.rs`

```rust
mod schema;
pub use schema::AnthropicTool;
```

**Success Criteria**:
- [ ] Compiles with derives
- [ ] Serialization produces correct JSON
- [ ] Unit test: convert ToolDefinition → AnthropicTool

---

### Task 2.2: Update AnthropicRequest for Tools

**File**: `crates/botticelli_models/src/anthropic/request.rs`

```rust
use crate::AnthropicTool;

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
- [ ] Serde skips field when None
- [ ] Unit test: serialize with/without tools

---

### Task 2.3: Update AnthropicResponse for Tool Calls

**File**: `crates/botticelli_models/src/anthropic/response.rs`

```rust
/// Content block can be text or tool_use.
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
    content: Vec<AnthropicContentBlock>,  // ← Now enum
    stop_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<AnthropicUsage>,
}
```

**Success Criteria**:
- [ ] Deserializes text-only responses
- [ ] Deserializes tool_use responses
- [ ] Deserializes mixed content
- [ ] Unit test for each case

---

### Task 2.4: Update AnthropicClient::generate to Handle Tools

**File**: `crates/botticelli_models/src/anthropic/client.rs`

**Update convert_request**:

```rust
impl AnthropicClient {
    #[instrument(skip(self, request))]
    pub(crate) fn convert_request(
        &self,
        request: &GenerateRequest,
    ) -> Result<AnthropicRequest, ModelsError> {
        debug!("Converting GenerateRequest to AnthropicRequest");

        // Convert messages (existing logic)
        let messages: Result<Vec<AnthropicMessage>, ModelsError> = /* ... */;

        let mut builder = AnthropicRequestBuilder::default()
            .model(request.model().unwrap_or(&self.model))
            .messages(messages?)
            .max_tokens(request.max_tokens().unwrap_or(1024));

        if let Some(temp) = request.temperature() {
            builder = builder.temperature(*temp);
        }

        // ← NEW: Handle tools
        if let Some(tools) = request.tools() {
            let anthropic_tools = tools.iter()
                .map(AnthropicTool::from_mcp)
                .collect();
            builder = builder.tools(Some(anthropic_tools));
        }

        builder.build()
            .map_err(|e| ModelsError::new(AnthropicErrorKind::Builder(e.to_string()).into()))
    }
}
```

**Update convert_response**:

```rust
impl AnthropicClient {
    #[instrument(skip(response))]
    pub(crate) fn convert_response(
        response: &AnthropicResponse,
    ) -> Result<GenerateResponse, ModelsError> {
        debug!("Converting AnthropicResponse to GenerateResponse");

        let mut outputs = Vec::new();

        // ← NEW: Handle mixed content
        for block in response.content() {
            match block {
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

        // Usage
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
- [ ] Compiles without errors
- [ ] Handles requests with/without tools
- [ ] Parses responses with text/tool_use/mixed content
- [ ] Unit tests for all scenarios

---

### Task 2.5: Integration Test with Real API

**File**: `crates/botticelli_models/tests/anthropic_tool_calling_test.rs` (new)

```rust
use botticelli_core::{GenerateRequest, Message, MessageBuilder, Role, Input, ToolDefinition};
use botticelli_interface::BotticelliDriver;
use botticelli_models::AnthropicClient;
use serde_json::json;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_anthropic_tool_calling() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");
    let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");

    let tool = ToolDefinition::new(
        "echo".to_string(),
        "Echoes back the input message".to_string(),
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        }),
    );

    let message = MessageBuilder::default()
        .role(Role::User)
        .content(vec![Input::Text("Use the echo tool to say 'Hello, tools!'".to_string())])
        .build()
        .expect("Valid message");

    let request = GenerateRequest::builder()
        .messages(vec![message])
        .tools(Some(vec![tool]))  // ← Tools in request
        .build()
        .expect("Valid request");

    let response = client.generate(&request).await.expect("API call succeeded");

    // Verify tool call in outputs
    let has_tool_call = response.outputs().iter()
        .any(|o| matches!(o, botticelli_core::Output::ToolCalls(_)));
    assert!(has_tool_call, "Response should contain tool call");
}
```

**Success Criteria**:
- [ ] Test passes with real Anthropic API
- [ ] Uses <50 tokens (rate limit conservation)
- [ ] Run with `just test-api`

---

## Phase 3: Fix TuiLlmBackend

### Task 3.1: Simplify TuiLlmBackend

**File**: `crates/botticelli_tui/src/state.rs`

**Action**: Remove workaround, just pass tools in request:

```rust
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

        // Build request with tools
        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .tools(Some(tools.to_vec()))  // ← Just add them
            .build()?;

        // Driver handles tools automatically
        let response = self.driver.generate(&request).await?;

        // Convert to JSON format for extract_tool_calls
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

**Success Criteria**:
- [ ] Compiles without errors
- [ ] No ignored parameters
- [ ] Tools actually passed to driver
- [ ] Comment removed: "Current architecture limitation"

---

## Phase 4: Extend to Other Providers

### Task 4.1: Implement Tool Support in GeminiClient

**Files**:
- `crates/botticelli_models/src/gemini/schema.rs` (new - GeminiTool)
- `crates/botticelli_models/src/gemini/request.rs` (add tools field)
- `crates/botticelli_models/src/gemini/response.rs` (handle functionCall)
- `crates/botticelli_models/src/gemini/client.rs` (update convert methods)

**Pattern**: Same as Anthropic - convert to Gemini's function calling format.

**Success Criteria**:
- [ ] GeminiClient handles tools in generate()
- [ ] Integration test passes with real API
- [ ] Gemini function format: `{ "name": "...", "parameters": {...} }`

---

### Task 4.2: Implement Tool Support in OllamaClient (Optional)

**Note**: Tool support varies by model in Ollama. Should check model capabilities.

**Approach**:
- Check if model supports tools
- If yes, convert and send
- If no, log warning and ignore tools field

**Success Criteria**:
- [ ] Works with tool-capable models
- [ ] Graceful fallback for non-capable models

---

### Task 4.3: Document Provider Tool Support

**File**: `crates/botticelli_models/README.md`

**Add section**:

```markdown
## Tool Calling Support

| Provider | Status | Format |
|----------|--------|--------|
| Anthropic | ✅ Full | Anthropic tools |
| Gemini | ✅ Full | Function calling |
| OpenAI | 🚧 Planned | Function calling |
| Ollama | ⚠️ Model-dependent | Varies |
| Groq | 🚧 Check API | TBD |
| HuggingFace | ❌ Not supported | N/A |

### Usage

```rust
let tool = ToolDefinition::new(
    "get_weather".to_string(),
    "Get weather for a location".to_string(),
    json!({ /* JSON schema */ }),
);

let request = GenerateRequest::builder()
    .messages(vec![message])
    .tools(Some(vec![tool]))  // ← Add tools
    .build()?;

let response = driver.generate(&request).await?;
```
```

**Success Criteria**:
- [ ] Documentation complete
- [ ] Examples clear
- [ ] Tool support status accurate

---

## Phase 5: Testing and Validation

### Task 5.1: End-to-End Integration Test

**File**: `crates/botticelli_tui/tests/tool_calling_integration_test.rs` (new)

```rust
use botticelli_core::{GenerateRequest, Message, MessageBuilder, Role, Input, ToolDefinition};
use botticelli_mcp_client::{UnifiedMcpClient, LlmBackend};
use botticelli_models::AnthropicClient;
use botticelli_tui::TuiLlmBackend;
use std::sync::Arc;
use serde_json::json;

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_end_to_end_tool_calling() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY");
    let driver = Arc::new(AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022"));
    let backend = TuiLlmBackend::new(driver);

    let mut mcp_client = UnifiedMcpClient::builder().max_iterations(5).build();
    let registry = mcp_client.internal_registry_mut();

    registry.register(
        "echo".to_string(),
        Arc::new(botticelli_mcp::tools::EchoTool),
    ).expect("Register echo");

    let messages = vec![
        MessageBuilder::default()
            .role(Role::User)
            .content(vec![Input::Text("Use echo to say 'Integration test works!'".to_string())])
            .build()
            .expect("Valid message"),
    ];

    let result = mcp_client.execute_with_tracking(&backend, messages).await
        .expect("Execution succeeded");

    assert!(result.iterations > 0);
    assert!(!result.tool_calls.is_empty());
    assert_eq!(result.tool_calls[0].tool_name, "echo");
    assert!(result.tool_calls[0].success);
}
```

**Success Criteria**:
- [ ] Test passes with real API
- [ ] Tool actually called by LLM
- [ ] Result tracked correctly
- [ ] Uses <100 tokens

---

### Task 5.2: Manual Testing Checklist

**Scenarios**:
1. [ ] Run TUI with Anthropic → Claude can call narrative tools
2. [ ] Ask "list all narratives" → uses list_narratives tool
3. [ ] Ask "create narrative called test" → uses create_narrative tool
4. [ ] Verify tool results in conversation
5. [ ] Check logs show tool execution
6. [ ] Test error handling: invalid arguments
7. [ ] Test max_iterations limit

---

## Phase 6: Cleanup and Documentation

### Task 6.1: Remove ToolUse Trait

**File**: `crates/botticelli_interface/src/traits.rs`

**Action**: Delete ToolUse trait (lines 169-192):

```rust
// ❌ DELETE THIS:
pub trait ToolUse: BotticelliDriver {
    async fn generate_with_tools(...) -> ...;
    fn max_tools(&self) -> usize { 128 }
    fn supports_parallel_tool_calls(&self) -> bool { false }
}
```

**Rationale**: No longer needed - tools are in GenerateRequest.

**Success Criteria**:
- [ ] Trait removed
- [ ] No references to ToolUse remain
- [ ] All code compiles

---

### Task 6.2: Update Architecture Documentation

**File**: `TUI_ARCHITECTURE_ANALYSIS.md`

**Action**: Update Gap 2 section:

```markdown
#### Gap 2: ToolUse Trait Not Implemented

**Status**: ✅ RESOLVED

**Solution**: Added tools field to GenerateRequest instead of separate trait.

Each driver handles tools in its generate() method:
- Check if req.tools() is Some(...)
- Convert to provider format
- Parse tool calls from response
- Return as Output::ToolCalls

**Drivers with tool support**:
- ✅ AnthropicClient
- ✅ GeminiClient
- ⚠️ OllamaClient (model-dependent)
```

**File**: `README.md`

Add tool calling example to main README.

**Success Criteria**:
- [ ] Documentation accurate
- [ ] Examples clear
- [ ] Architecture described correctly

---

### Task 6.3: Remove Workarounds and TODOs

**Action**: Search and clean up:

```bash
grep -r "TODO.*tool" crates/
grep -r "architecture limitation" crates/
grep -r "_tools.*/" crates/  # Underscore-prefixed unused params
```

**Success Criteria**:
- [ ] All TODOs addressed or filed as issues
- [ ] No ignored tool parameters
- [ ] No "architecture limitation" comments

---

## Implementation Order

1. **Phase 1** (Core Types) - 4 tasks - Foundation
2. **Phase 2** (Anthropic) - 5 tasks - First provider
3. **Phase 3** (TuiLlmBackend) - 1 task - Wire it up
4. **Phase 5.1** (E2E Test) - 1 task - Validate
5. **Phase 4** (Other Providers) - 3 tasks - Expand
6. **Phase 5.2** (Manual Test) - 1 checklist - User validation
7. **Phase 6** (Cleanup) - 3 tasks - Polish

**Estimated Effort**: 4-6 focused sessions

---

## Success Metrics

### Functional
- [ ] ToolDefinition in botticelli_core
- [ ] GenerateRequest has tools field
- [ ] AnthropicClient handles tools in generate()
- [ ] TuiLlmBackend passes tools in request
- [ ] Tool calling works end-to-end
- [ ] Integration tests passing

### Code Quality
- [ ] No separate ToolUse trait
- [ ] No ignored parameters
- [ ] All #[instrument] in place
- [ ] Proper error handling
- [ ] Tests in tests/ directory

### Observability
- [ ] Tool call attempts traced
- [ ] Tool execution logged
- [ ] Clear error messages

---

## Benefits of This Approach

1. **Simpler** - No trait hierarchy, just one field
2. **Consistent** - Tools work like temperature/max_tokens
3. **Scalable** - Each driver owns conversion logic
4. **Backward compatible** - Tools are optional
5. **Extensible** - Easy to add new providers

---

## Comparison with Previous Plan

| Aspect | V1 (ToolUse Trait) | V2 (GenerateRequest) |
|--------|-------------------|---------------------|
| Trait count | 2 (BotticelliDriver + ToolUse) | 1 (BotticelliDriver) |
| Abstraction | Parallel trait hierarchy | Single request type |
| Per-provider code | Duplicate trait impl | Just format conversion |
| Type safety | Compile-time trait bounds | Runtime Option check |
| Flexibility | Rigid trait contract | Graceful degradation |
| Complexity | Higher | Lower |

**Winner**: V2 - Simpler, more flexible, better Rust idioms.

---

## Notes

- Follows CLAUDE.md: builders, derive_more, tracing, tests in tests/
- ToolDefinition uses JSON Schema (MCP standard)
- Each provider owns format conversion (encapsulation)
- Graceful degradation: drivers without tools can ignore field or error
- No breaking changes to existing code

---

## Related Documents

- `ARCHITECTURAL_GAPS_RESOLUTION_PLAN.md` - Original plan (superseded by v2)
- `TUI_ARCHITECTURE_ANALYSIS.md` - Initial gap analysis
- `LLM_FALLBACK_IMPLEMENTATION_PLAN.md` - Deferred until tools work
