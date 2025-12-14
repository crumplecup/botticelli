# Narrative Sampling Implementation Plan

## Quick Start: What to Do First

**Start Here:** Phase 1, Task 1 - Define the `LlmClient` trait in `botticelli_core`

```rust
// crates/botticelli_core/src/llm_client.rs
pub trait LlmClient {
    async fn generate_with_tools(
        &self,
        request: GenerateRequest,
        tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, LlmError>;
}
```

**Why this first:** Everything else depends on having a unified LLM interface. Without it, we can't implement the sampling loop.

**What success looks like:** You can call any LLM provider through the same interface and get tool calls back in a unified format.

---

## Current State Assessment

### What We Have

#### botticelli_mcp (Foundation Layer)
1. **Elicitation Traits** (`src/elicitation/`)
   - `ElicitationDialog` - UI abstraction for prompting users
   - `NarrativeElicitor` - Component elicitor interface
   - `PartialNarrative` / `PartialAct` - In-progress state representation
   - `SessionOrchestrator` - Coordinate elicitor sequence (STUB)

2. **MCP Tools** (`src/tools/elicitation/`)
   - `NarrativeRegistry` - Session state storage
   - `NarrativeHelper` - Static utility methods for text analysis
   - `CreateNarrativeSessionTool` - Initialize session with description analysis
   - `ElicitMetadataTool` - Set/update metadata (name, description, defaults)
   - `ElicitActTool` - Add/update acts with prompts
   - `FinalizeNarrativeTool` - Generate TOML from session state

3. **Sampling Coordinator** (`src/tools/sampling.rs`)
   - `SamplingCoordinator` - Orchestrates LLM-driven narrative generation
   - `LlmSampler` trait - Interface for LLM tool calling
   - `SamplingSession` / `Turn` / `ToolResponse` - Session state types
   - `SamplingHelper` - System prompt generation

4. **Session Manager** (`src/tools/sampling_session_manager.rs`)
   - `SamplingSessionManager` - Manages active sessions
   - Session lifecycle (create, get, list, remove)

#### botticelli_chat (Consumer Layer)
1. **Elicitor Implementations** (`src/elicitation/`)
   - `TuiElicitationDialog` - TUI adapter for user prompts
   - `MetadataElicitor` - Gather name/description/defaults
   - `InputElicitor` - Gather inputs for acts
   - `CarouselElicitor` - Gather carousels for acts
   - `ValidationElicitor` - Validate user-provided TOML
   - (These use TUI but are NOT integrated with MCP tools)

2. **Sampling Integration** (`src/sampling*.rs`)
   - `ChatLlmSampler` - STUB (unimplemented)
   - `SamplingIntegration` - Wires sampler to chat commands (partially done)

3. **Command Executor** (`src/executor.rs`)
   - Has `SamplingIntegration` reference
   - Has `current_narrative` state
   - NOT connected to sampling workflow

### What's Missing

#### Critical Gaps

1. **No LLM Client Infrastructure**
   - No unified trait for calling LLMs with tool support
   - Chat implementations use provider-specific clients directly
   - Need: Abstract `LlmClient` trait that handles tool calls
   - Location: Should be in `botticelli_core`

2. **ChatLlmSampler is a Stub**
   - Currently returns `unimplemented!()`
   - Needs: Full sampling loop implementation
   - Must: Extract tool calls from LLM responses
   - Must: Execute tools via registry
   - Must: Build multi-turn conversations

3. **MCP Tools Not Registered**
   - Tools exist but not exposed to LLM
   - Need: Registry of available tools with schemas
   - Need: Tool dispatcher that routes calls to implementations
   - Need: Integration with sampling coordinator

4. **No Tool Call Parsing**
   - LLM responses contain tool calls in provider-specific formats
   - Need: Extraction logic for each provider (Claude, OpenAI, etc.)
   - Need: Unified tool call representation

5. **Two Separate Workflows**
   - MCP tools work with JSON state
   - Chat elicitors work with TUI dialogs
   - NOT integrated - user can't switch between them
   - Need: Bridge or unified approach

6. **No Command Integration**
   - CommandExecutor has sampling reference but doesn't use it
   - Need: Commands like `narrative create <description>` to trigger sampling
   - Need: Commands to view/refine current narrative

#### Design Questions

1. **Should chat elicitors use MCP tools internally?**
   - Option A: Keep separate (simpler, less composable)
   - Option B: Chat elicitors call MCP tools (more complex, unified)
   - Recommendation: Keep separate for now, document boundary

2. **Where does tool execution happen?**
   - Option A: In SamplingCoordinator (centralized)
   - Option B: In ChatLlmSampler (decoupled)
   - Recommendation: ChatLlmSampler - it knows conversation context

3. **How to expose to Claude Desktop?**
   - MCP server needs to register these tools
   - Need: Integration in `botticelli_mcp/src/server.rs`

## Implementation Roadmap

### Phase 1: LLM Client Abstraction (Required Foundation)

**Goal:** Create unified interface for LLM calls with tool support

**Blocks:** Phase 2 (ChatLlmSampler), Phase 4 (CommandExecutor)

#### Task 1.1: Define Core Types

**File:** `crates/botticelli_core/src/llm_client.rs` (new)

**Action:** Create these exact type definitions:

```rust
use serde::{Deserialize, Serialize};

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,  // JSON Schema
}

/// Tool call from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,         // Unique ID for this call
    pub name: String,       // Tool name
    pub input: serde_json::Value,  // Arguments as JSON
}

/// Result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: serde_json::Value,
    pub is_error: bool,
}

/// Response that may contain tool calls
#[derive(Debug, Clone)]
pub enum GenerateResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
    TextAndToolCalls { text: String, tool_calls: Vec<ToolCall> },
}
```

**Acceptance:**
- [ ] Types compile
- [ ] All fields are public
- [ ] Derives include Serialize/Deserialize where needed
- [ ] `cargo check -p botticelli_core` passes

#### Task 1.2: Define LlmClient Trait

**File:** `crates/botticelli_core/src/llm_client.rs`

**Action:** Add this trait definition:

```rust
use async_trait::async_trait;
use crate::{GenerateRequest, LlmError};

#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate response with optional tool support
    async fn generate_with_tools(
        &self,
        request: GenerateRequest,
        tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, LlmError>;

    /// Get provider name for debugging
    fn provider_name(&self) -> &str;
}
```

**Acceptance:**
- [ ] Trait compiles
- [ ] Uses async_trait (add to Cargo.toml if needed)
- [ ] Error type exists in `botticelli_error`
- [ ] Empty tools slice means "no tool calling"

#### Task 1.3: Implement for Claude Provider

**File:** `crates/botticelli_core/src/providers/claude.rs`

**Action:** Implement trait for existing Claude client:

```rust
#[async_trait]
impl LlmClient for ClaudeClient {
    async fn generate_with_tools(
        &self,
        request: GenerateRequest,
        tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, LlmError> {
        // Convert ToolDefinition to Anthropic tool format
        let anthropic_tools: Vec<_> = tools.iter()
            .map(|t| /* convert to anthropic::Tool */)
            .collect();

        // Make API call
        let response = self.client.messages()
            .create(/* ... */)
            .await?;

        // Extract tool calls from response.content
        let tool_calls = response.content.iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => Some(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                }),
                _ => None,
            })
            .collect();

        if tool_calls.is_empty() {
            Ok(GenerateResponse::Text(extract_text(&response)))
        } else {
            Ok(GenerateResponse::ToolCalls(tool_calls))
        }
    }

    fn provider_name(&self) -> &str { "claude" }
}
```

**Acceptance:**
- [ ] Implementation compiles
- [ ] Test with actual API call (use `#[cfg_attr(not(feature = "api"), ignore)]`)
- [ ] Tool calls extracted correctly from Claude response
- [ ] Text-only responses work
- [ ] Errors propagate properly

#### Task 1.4: Add Test Infrastructure

**File:** `crates/botticelli_core/tests/llm_client_test.rs`

**Action:** Create test with mock LLM:

```rust
use botticelli_core::{LlmClient, ToolDefinition, GenerateResponse};

struct MockLlmClient {
    response: GenerateResponse,
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn generate_with_tools(
        &self,
        _request: GenerateRequest,
        _tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, LlmError> {
        Ok(self.response.clone())
    }

    fn provider_name(&self) -> &str { "mock" }
}

#[tokio::test]
async fn test_mock_returns_tool_calls() {
    let client = MockLlmClient {
        response: GenerateResponse::ToolCalls(vec![
            ToolCall {
                id: "call_1".to_string(),
                name: "test_tool".to_string(),
                input: json!({"arg": "value"}),
            }
        ]),
    };

    let result = client.generate_with_tools(
        GenerateRequest::default(),
        &[],
    ).await.unwrap();

    match result {
        GenerateResponse::ToolCalls(calls) => {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].name, "test_tool");
        }
        _ => panic!("Expected tool calls"),
    }
}
```

**Acceptance:**
- [ ] Mock client test passes
- [ ] Test demonstrates expected interface
- [ ] Can inject different responses for testing

#### Task 1.5: Update Exports

**File:** `crates/botticelli_core/src/lib.rs`

**Action:** Add these exports:

```rust
mod llm_client;

pub use llm_client::{
    LlmClient,
    ToolDefinition,
    ToolCall,
    ToolResult,
    GenerateResponse,
};
```

**Acceptance:**
- [ ] Other crates can `use botticelli_core::{LlmClient, ToolCall};`
- [ ] No module paths needed
- [ ] `just check` passes

### Phase 2: Complete ChatLlmSampler (Core Loop)

**Goal:** Implement full LLM sampling loop with tool execution

**Dependencies:** Requires Phase 1 complete, Phase 3 in parallel

#### Task 2.1: Add LlmClient to ChatLlmSampler

**File:** `crates/botticelli_chat/src/sampling.rs`

**Action:** Replace stub with real implementation:

```rust
use botticelli_core::{LlmClient, ToolDefinition};
use botticelli_mcp::ToolRegistry;
use std::sync::Arc;

pub struct ChatLlmSampler {
    client: Arc<dyn LlmClient>,
    registry: Arc<ToolRegistry>,
    max_turns: usize,  // Prevent infinite loops
}

impl ChatLlmSampler {
    pub fn new(
        client: Arc<dyn LlmClient>,
        registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            client,
            registry,
            max_turns: 20,  // Reasonable default
        }
    }
}
```

**Acceptance:**
- [ ] Struct compiles
- [ ] Dependencies added to Cargo.toml
- [ ] Client is thread-safe (Arc<dyn LlmClient>)

#### Task 2.2: Implement Sampling Loop

**File:** `crates/botticelli_chat/src/sampling.rs`

**Action:** Implement the core `sample()` method:

```rust
#[async_trait]
impl LlmSampler for ChatLlmSampler {
    async fn sample(
        &self,
        session: &mut SamplingSession,
    ) -> Result<SamplingResult, SamplingError> {
        let tools = self.registry.list_tools();
        let mut turn_count = 0;

        loop {
            turn_count += 1;
            if turn_count > self.max_turns {
                return Err(SamplingError::new(
                    SamplingErrorKind::MaxTurnsExceeded { max: self.max_turns }
                ));
            }

            // Build request from session history
            let request = build_request_from_session(session)?;

            // Call LLM with tools
            let response = self.client
                .generate_with_tools(request, &tools)
                .await?;

            match response {
                GenerateResponse::Text(text) => {
                    // LLM is done, return final text
                    session.add_turn(Turn::AssistantText { text: text.clone() });
                    return Ok(SamplingResult::Completed { final_text: text });
                }

                GenerateResponse::ToolCalls(calls) => {
                    // Execute each tool call
                    let mut results = Vec::new();
                    for call in &calls {
                        let result = self.registry
                            .execute(&call.name, &call.input)
                            .await?;

                        results.push(ToolResult {
                            tool_call_id: call.id.clone(),
                            output: result,
                            is_error: false,
                        });
                    }

                    // Add to history
                    session.add_turn(Turn::ToolCalls { calls: calls.clone() });
                    session.add_turn(Turn::ToolResults { results: results.clone() });

                    // Continue loop to get LLM's next response
                }

                GenerateResponse::TextAndToolCalls { text, tool_calls } => {
                    // Handle combined response
                    session.add_turn(Turn::AssistantText { text });
                    session.add_turn(Turn::ToolCalls { calls: tool_calls.clone() });

                    // Execute tools and continue
                    // ... (similar to above)
                }
            }
        }
    }
}
```

**Acceptance:**
- [ ] Loop terminates on text-only response
- [ ] Loop continues on tool calls
- [ ] Max turns limit works
- [ ] Tool results added to history
- [ ] Errors propagate with context

#### Task 2.3: Add Request Building

**File:** `crates/botticelli_chat/src/sampling.rs`

**Action:** Convert session history to LLM request:

```rust
fn build_request_from_session(
    session: &SamplingSession,
) -> Result<GenerateRequest, SamplingError> {
    let mut messages = Vec::new();

    // System prompt
    messages.push(Message::builder()
        .role(Role::System)
        .content(vec![Input::Text(session.system_prompt.clone())])
        .build()?);

    // User's initial prompt
    messages.push(Message::builder()
        .role(Role::User)
        .content(vec![Input::Text(session.initial_prompt.clone())])
        .build()?);

    // Add all turns
    for turn in &session.turns {
        match turn {
            Turn::AssistantText { text } => {
                messages.push(Message::builder()
                    .role(Role::Assistant)
                    .content(vec![Input::Text(text.clone())])
                    .build()?);
            }
            Turn::ToolCalls { calls } => {
                // Convert to assistant message with tool calls
                // Format depends on provider
            }
            Turn::ToolResults { results } => {
                // Convert to user message with tool results
                // Format depends on provider
            }
        }
    }

    GenerateRequest::builder()
        .messages(messages)
        .build()
}
```

**Acceptance:**
- [ ] System prompt included once at start
- [ ] All turns converted correctly
- [ ] Tool call format matches provider expectations
- [ ] Test with mock session history

#### Task 2.4: Add Error Handling

**File:** `crates/botticelli_chat/src/sampling_error.rs` (new)

**Action:** Define comprehensive error types:

```rust
use derive_more::{Display, Error};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum SamplingErrorKind {
    #[display("Max turns exceeded: {}", max)]
    MaxTurnsExceeded { max: usize },

    #[display("Tool execution failed: {} - {}", tool_name, reason)]
    ToolExecutionFailed { tool_name: String, reason: String },

    #[display("Unknown tool called: {}", name)]
    UnknownTool { name: String },

    #[display("Invalid tool input: {}", reason)]
    InvalidToolInput { reason: String },

    #[display("LLM error: {}", _0)]
    LlmError(String),
}

#[derive(Debug, Clone, Display, Error)]
#[display("Sampling: {} at {}:{}", kind, file, line)]
pub struct SamplingError {
    pub kind: SamplingErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl SamplingError {
    #[track_caller]
    pub fn new(kind: SamplingErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self { kind, line: loc.line(), file: loc.file() }
    }
}
```

**Acceptance:**
- [ ] All error scenarios covered
- [ ] Follows botticelli error conventions
- [ ] Uses derive_more (no manual Display/Error impl)
- [ ] Location tracking with #[track_caller]

#### Task 2.5: Add Sampling Tests

**File:** `crates/botticelli_chat/tests/sampling_test.rs`

**Action:** Test the complete sampling loop:

```rust
#[tokio::test]
async fn test_sampling_loop_with_single_tool_call() {
    // Setup mock LLM that returns tool call, then text
    let mock_client = MockLlmClient::new(vec![
        GenerateResponse::ToolCalls(vec![
            ToolCall {
                id: "1".to_string(),
                name: "create_narrative_session".to_string(),
                input: json!({"description": "test"}),
            }
        ]),
        GenerateResponse::Text("Done!".to_string()),
    ]);

    // Setup mock registry
    let registry = Arc::new(MockToolRegistry::new());

    let sampler = ChatLlmSampler::new(
        Arc::new(mock_client),
        registry,
    );

    let mut session = SamplingSession::new(
        "system prompt".to_string(),
        "user prompt".to_string(),
    );

    let result = sampler.sample(&mut session).await.unwrap();

    assert_eq!(session.turns.len(), 4);  // tool_call, tool_result, text
    assert!(matches!(result, SamplingResult::Completed { .. }));
}

#[tokio::test]
async fn test_max_turns_limit() {
    // Mock that always returns tool calls
    let mock_client = MockLlmClient::always_tool_calls();
    let registry = Arc::new(MockToolRegistry::new());

    let sampler = ChatLlmSampler::new(
        Arc::new(mock_client),
        registry,
    );

    let mut session = SamplingSession::new("sys".to_string(), "user".to_string());

    let result = sampler.sample(&mut session).await;

    assert!(matches!(
        result,
        Err(SamplingError { kind: SamplingErrorKind::MaxTurnsExceeded { .. }, .. })
    ));
}
```

**Acceptance:**
- [ ] Single turn test passes
- [ ] Multi-turn test passes
- [ ] Max turns limit test passes
- [ ] Tool execution error test passes
- [ ] Unknown tool test passes

### Phase 3: Tool Registry & Dispatcher (Infrastructure)

**Goal:** Make MCP tools callable from sampling loop

**Dependencies:** None (parallel with Phase 1-2)

#### Task 3.1: Define Tool Trait

**File:** `crates/botticelli_mcp/src/tools/registry.rs` (new)

**Action:** Create trait all tools must implement:

```rust
use async_trait::async_trait;
use serde_json::Value;
use botticelli_core::ToolDefinition;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must be unique)
    fn name(&self) -> &str;

    /// Human-readable description for LLM
    fn description(&self) -> &str;

    /// JSON Schema for input validation
    fn input_schema(&self) -> Value;

    /// Execute tool with JSON input, return JSON output
    async fn execute(&self, input: Value) -> Result<Value, ToolError>;

    /// Get full definition for LLM
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.input_schema(),
        }
    }
}
```

**Acceptance:**
- [ ] Trait compiles
- [ ] async_trait used
- [ ] Input/output is serde_json::Value
- [ ] Default definition() implementation works

#### Task 3.2: Create Tool Registry

**File:** `crates/botticelli_mcp/src/tools/registry.rs`

**Action:** Registry holds all registered tools:

```rust
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool (fails if name collision)
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<(), RegistryError> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(RegistryError::new(
                RegistryErrorKind::DuplicateTool { name }
            ));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Get all tool definitions for LLM
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .map(|tool| tool.definition())
            .collect()
    }

    /// Execute tool by name
    pub async fn execute(
        &self,
        name: &str,
        input: &Value,
    ) -> Result<Value, ToolError> {
        let tool = self.tools.get(name)
            .ok_or_else(|| ToolError::new(
                ToolErrorKind::UnknownTool { name: name.to_string() }
            ))?;

        tool.execute(input.clone()).await
    }
}
```

**Acceptance:**
- [ ] Can register tools
- [ ] Prevents duplicate names
- [ ] Lists all definitions
- [ ] Executes by name
- [ ] Returns error for unknown tool

#### Task 3.3: Implement Tool for Existing MCP Tools

**File:** `crates/botticelli_mcp/src/tools/elicitation/create_session.rs`

**Action:** Make CreateNarrativeSessionTool implement Tool trait:

```rust
use crate::Tool;

pub struct CreateNarrativeSessionTool {
    registry: Arc<RwLock<NarrativeRegistry>>,
}

#[async_trait]
impl Tool for CreateNarrativeSessionTool {
    fn name(&self) -> &str {
        "create_narrative_session"
    }

    fn description(&self) -> &str {
        "Initialize a new narrative creation session with description analysis"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Unique identifier for this session"
                },
                "description": {
                    "type": "string",
                    "description": "User's narrative description to analyze"
                }
            },
            "required": ["session_id", "description"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value, ToolError> {
        // Deserialize input
        #[derive(Deserialize)]
        struct Input {
            session_id: String,
            description: String,
        }

        let input: Input = serde_json::from_value(input)
            .map_err(|e| ToolError::new(
                ToolErrorKind::InvalidInput { reason: e.to_string() }
            ))?;

        // Execute existing logic
        let analyzed = NarrativeHelper::analyze_description(&input.description);

        let mut registry = self.registry.write().unwrap();
        registry.create_session(&input.session_id, analyzed);

        // Return success
        Ok(json!({
            "session_id": input.session_id,
            "status": "created",
            "analyzed_theme": analyzed.theme
        }))
    }
}
```

**Acceptance:**
- [ ] Tool trait implementation compiles
- [ ] Input schema matches expected format
- [ ] Executes existing MCP tool logic
- [ ] Returns structured JSON result
- [ ] Error handling converts to ToolError

#### Task 3.4: Register All Elicitation Tools

**File:** `crates/botticelli_mcp/src/tools/mod.rs`

**Action:** Create registry builder function:

```rust
use crate::{ToolRegistry, CreateNarrativeSessionTool, ElicitMetadataTool, /* ... */};

pub fn build_narrative_tools_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    let narrative_registry = Arc::new(RwLock::new(NarrativeRegistry::new()));

    // Register all elicitation tools
    registry.register(Arc::new(CreateNarrativeSessionTool::new(
        narrative_registry.clone()
    ))).expect("CreateNarrativeSessionTool");

    registry.register(Arc::new(ElicitMetadataTool::new(
        narrative_registry.clone()
    ))).expect("ElicitMetadataTool");

    registry.register(Arc::new(ElicitActTool::new(
        narrative_registry.clone()
    ))).expect("ElicitActTool");

    registry.register(Arc::new(FinalizeNarrativeTool::new(
        narrative_registry.clone()
    ))).expect("FinalizeNarrativeTool");

    registry
}
```

**Acceptance:**
- [ ] All tools share same NarrativeRegistry
- [ ] No duplicate names (panics if collision)
- [ ] Returns ready-to-use registry
- [ ] Test can call any tool via registry

#### Task 3.5: Add Registry Tests

**File:** `crates/botticelli_mcp/tests/tool_registry_test.rs`

**Action:** Test registry behavior:

```rust
#[tokio::test]
async fn test_registry_lists_all_tools() {
    let registry = build_narrative_tools_registry();
    let tools = registry.list_tools();

    assert_eq!(tools.len(), 4);
    assert!(tools.iter().any(|t| t.name == "create_narrative_session"));
    assert!(tools.iter().any(|t| t.name == "elicit_metadata"));
    assert!(tools.iter().any(|t| t.name == "elicit_act"));
    assert!(tools.iter().any(|t| t.name == "finalize_narrative"));
}

#[tokio::test]
async fn test_execute_tool_by_name() {
    let registry = build_narrative_tools_registry();

    let result = registry.execute(
        "create_narrative_session",
        &json!({
            "session_id": "test_123",
            "description": "A space adventure"
        })
    ).await.unwrap();

    assert_eq!(result["session_id"], "test_123");
    assert_eq!(result["status"], "created");
}

#[tokio::test]
async fn test_unknown_tool_error() {
    let registry = build_narrative_tools_registry();

    let result = registry.execute("unknown_tool", &json!({})).await;

    assert!(matches!(
        result,
        Err(ToolError { kind: ToolErrorKind::UnknownTool { .. }, .. })
    ));
}
```

**Acceptance:**
- [ ] Can list all tools
- [ ] Can execute by name
- [ ] Unknown tool returns error
- [ ] Tools share state (session persists between calls)

### Phase 4: Connect to CommandExecutor (User Access)

**Goal:** Add commands to trigger LLM sampling workflow

**Dependencies:** Phase 2 must be complete

#### Task 4.1: Add Command Enum Variants

**File:** `crates/botticelli_chat/src/command.rs`

**Action:** Add narrative commands:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // ... existing commands

    NarrativeGenerate { description: String },
    NarrativeRefine { feedback: String },
    NarrativeShow,
    NarrativeSave { path: PathBuf },
}
```

**Acceptance:**
- [ ] Enum variants compile
- [ ] Derives include PartialEq for testing

#### Task 4.2: Add Command Parsing

**File:** `crates/botticelli_chat/src/parser.rs`

**Action:** Parse narrative commands:

```rust
fn parse_narrative_command(parts: &[&str]) -> Result<Command, ParseError> {
    match parts.get(1) {
        Some(&"generate") => {
            let description = parts[2..].join(" ");
            if description.is_empty() {
                return Err(ParseError::new(
                    ParseErrorKind::MissingArgument {
                        command: "narrative generate".to_string(),
                        arg: "description".to_string(),
                    }
                ));
            }
            Ok(Command::NarrativeGenerate { description })
        }
        Some(&"refine") => {
            let feedback = parts[2..].join(" ");
            if feedback.is_empty() {
                return Err(ParseError::new(
                    ParseErrorKind::MissingArgument {
                        command: "narrative refine".to_string(),
                        arg: "feedback".to_string(),
                    }
                ));
            }
            Ok(Command::NarrativeRefine { feedback })
        }
        Some(&"show") => Ok(Command::NarrativeShow),
        Some(&"save") => {
            let path = parts.get(2)
                .ok_or_else(|| ParseError::new(
                    ParseErrorKind::MissingArgument {
                        command: "narrative save".to_string(),
                        arg: "path".to_string(),
                    }
                ))?;
            Ok(Command::NarrativeSave { path: PathBuf::from(path) })
        }
        _ => Err(ParseError::new(
            ParseErrorKind::UnknownCommand {
                command: format!("narrative {}", parts.get(1).unwrap_or(&""))
            }
        ))
    }
}
```

**Acceptance:**
- [ ] Parses "narrative generate A story about..."
- [ ] Parses "narrative refine Add more action"
- [ ] Parses "narrative show"
- [ ] Parses "narrative save /path/to/file.toml"
- [ ] Returns error for missing arguments

#### Task 4.3: Implement Command Execution

**File:** `crates/botticelli_chat/src/executor.rs`

**Action:** Handle narrative commands:

```rust
impl CommandExecutor {
    #[instrument(skip(self))]
    pub async fn execute(&mut self, command: Command) -> Result<String, ExecutorError> {
        match command {
            // ... existing commands

            Command::NarrativeGenerate { description } => {
                self.execute_narrative_generate(description).await
            }
            Command::NarrativeRefine { feedback } => {
                self.execute_narrative_refine(feedback).await
            }
            Command::NarrativeShow => {
                self.execute_narrative_show()
            }
            Command::NarrativeSave { path } => {
                self.execute_narrative_save(path).await
            }
        }
    }

    async fn execute_narrative_generate(
        &mut self,
        description: String,
    ) -> Result<String, ExecutorError> {
        info!(description = %description, "Starting narrative generation");

        // Create session
        let session_id = format!("session_{}", uuid::Uuid::new_v4());
        let mut session = SamplingSession::new(
            SamplingHelper::system_prompt(),
            format!("Create a narrative: {}", description),
        );

        // Run sampling loop
        let result = self.sampling
            .sampler()
            .sample(&mut session)
            .await?;

        match result {
            SamplingResult::Completed { final_text } => {
                info!("Narrative generation completed");

                // Parse TOML from final text
                let narrative: Narrative = toml::from_str(&final_text)
                    .map_err(|e| ExecutorError::new(
                        ExecutorErrorKind::InvalidToml { reason: e.to_string() }
                    ))?;

                self.current_narrative = Some(narrative.clone());

                Ok(format!(
                    "Generated narrative '{}' with {} acts. Use 'narrative show' to view or 'narrative save <path>' to save.",
                    narrative.name,
                    narrative.acts.len()
                ))
            }
        }
    }

    async fn execute_narrative_refine(
        &mut self,
        feedback: String,
    ) -> Result<String, ExecutorError> {
        let current = self.current_narrative.as_ref()
            .ok_or_else(|| ExecutorError::new(
                ExecutorErrorKind::NoCurrentNarrative
            ))?;

        // Build refinement prompt
        let current_toml = toml::to_string(current)?;
        let prompt = format!(
            "Refine this narrative:\n\n{}\n\nFeedback: {}",
            current_toml,
            feedback
        );

        // Create session with refinement context
        let mut session = SamplingSession::new(
            SamplingHelper::system_prompt(),
            prompt,
        );

        // Run sampling
        let result = self.sampling.sampler().sample(&mut session).await?;

        // Update current narrative
        // ... (similar to generate)

        Ok("Narrative refined. Use 'narrative show' to view changes.".to_string())
    }

    fn execute_narrative_show(&self) -> Result<String, ExecutorError> {
        let narrative = self.current_narrative.as_ref()
            .ok_or_else(|| ExecutorError::new(
                ExecutorErrorKind::NoCurrentNarrative
            ))?;

        let toml = toml::to_string_pretty(narrative)?;
        Ok(format!("Current narrative:\n\n{}", toml))
    }

    async fn execute_narrative_save(
        &self,
        path: PathBuf,
    ) -> Result<String, ExecutorError> {
        let narrative = self.current_narrative.as_ref()
            .ok_or_else(|| ExecutorError::new(
                ExecutorErrorKind::NoCurrentNarrative
            ))?;

        let toml = toml::to_string_pretty(narrative)?;
        tokio::fs::write(&path, toml).await?;

        Ok(format!("Saved narrative to {}", path.display()))
    }
}
```

**Acceptance:**
- [ ] Generate command creates narrative
- [ ] Current narrative stored
- [ ] Show displays TOML
- [ ] Save writes to file
- [ ] Refine updates existing narrative
- [ ] Error if no current narrative

#### Task 4.4: Add Executor Tests

**File:** `crates/botticelli_chat/tests/executor_narrative_test.rs`

**Action:** Test command execution:

```rust
#[tokio::test]
async fn test_narrative_generate_command() {
    let mut executor = test_executor_with_mock_sampling();

    let result = executor.execute(Command::NarrativeGenerate {
        description: "A space adventure".to_string(),
    }).await.unwrap();

    assert!(result.contains("Generated narrative"));
    assert!(executor.current_narrative.is_some());
}

#[tokio::test]
async fn test_narrative_show_before_generate() {
    let executor = test_executor();

    let result = executor.execute(Command::NarrativeShow).await;

    assert!(matches!(
        result,
        Err(ExecutorError { kind: ExecutorErrorKind::NoCurrentNarrative, .. })
    ));
}

#[tokio::test]
async fn test_narrative_save() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test.toml");

    let mut executor = test_executor_with_narrative();

    executor.execute(Command::NarrativeSave { path: path.clone() })
        .await
        .unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[narrative]"));
}
```

**Acceptance:**
- [ ] Generate creates narrative
- [ ] Show requires existing narrative
- [ ] Save writes valid TOML
- [ ] Refine updates narrative

### Phase 5: MCP Server Integration (Claude Desktop)

**Goal:** Expose elicitation tools to Claude Desktop

**Location:** `crates/botticelli_mcp/src/server.rs`

**Tasks:**
1. Register elicitation tools in MCP server
2. Wire tool execution to registry/dispatcher
3. Add tool schemas to capability advertisement
4. Test with Claude Desktop

**Dependencies:** Phase 3 must be complete

**Success Criteria:**
- Claude Desktop sees narrative tools
- Can create session, add acts, finalize
- TOML output is valid

**Files to Modify:**
- `crates/botticelli_mcp/src/server.rs`

### Phase 6: Testing & Documentation

**Goal:** Comprehensive tests and usage examples

**Tasks:**
1. **Unit Tests:**
   - Tool execution with mock inputs
   - Registry lookup/dispatch
   - Session state management
   - TOML generation

2. **Integration Tests:**
   - ChatLlmSampler with mock LLM
   - Full sampling loop (mock tool execution)
   - Command execution end-to-end

3. **Documentation:**
   - Update `NARRATIVE_SAMPLING_USAGE.md` with actual code examples
   - Add architecture diagram
   - Document tool schemas
   - Add troubleshooting guide

**Files to Create:**
- `tests/sampling_unit_test.rs`
- `tests/sampling_integration_test.rs`
- `NARRATIVE_SAMPLING_USAGE.md` (update)

## Next Immediate Steps

1. **Start Phase 1:** Create LLM client abstraction
   - This unblocks everything else
   - Can be tested independently
   - Clear success criteria

2. **Parallel:** Design tool registry structure (Phase 3)
   - Can proceed independently
   - Needs buy-in on architecture

3. **Document:** Create example of desired user interaction
   - "User types X, sees Y, system does Z"
   - Helps validate design

## Decisions Made

### LLM Client Design
- **Location:** `botticelli_core` (needed by multiple crates)
- **Streaming:** Not needed for tool calling (add later if needed)
- **Tool schemas:** Provider-agnostic JSON Schema, convert per-provider internally

### Tool Registry Ownership
- **Ownership:** Registry is created once, shared via Arc
- **State:** Tools hold Arc<RwLock<NarrativeRegistry>> for shared state
- **Coordinator:** SamplingCoordinator receives registry as dependency

### Error Handling Strategy
- **Invalid tool:** Return ToolError, add to conversation, let LLM recover
- **Execution failure:** Same - tell LLM it failed, let it retry or abort
- **No auto-retry:** LLM decides whether to retry (more flexible)
- **Max turns:** Hard limit to prevent infinite loops (default 20)

### Chat vs MCP Elicitors
- **Keep separate:** Different use cases (interactive TUI vs LLM-driven)
- **Both use same data structures:** PartialNarrative, etc.
- **No unification planned:** Document as two workflows

## Success Metrics

### MVP Acceptance Test

Run these exact commands to verify MVP is complete:

```bash
# 1. Start the chat application
cargo run -p botticelli_chat

# 2. Generate a narrative
> narrative generate A three-act space adventure with a hero's journey

# Expected: LLM makes tool calls, creates narrative
# Expected output: "Generated narrative 'X' with 3 acts..."

# 3. View the narrative
> narrative show

# Expected: Valid TOML output with [narrative], [[act]], etc.

# 4. Save to file
> narrative save /tmp/test_narrative.toml

# Expected: File created with valid TOML

# 5. Validate file externally
$ cat /tmp/test_narrative.toml
$ cargo run -p botticelli_narrative -- validate /tmp/test_narrative.toml

# Expected: No errors, narrative is valid
```

**MVP is complete when all 5 steps succeed without errors.**

### Full Feature Set (Post-MVP)

Additional capabilities to add later:
- Refinement with feedback (`narrative refine "add more conflict"`)
- Complex inputs/carousels in acts
- Error recovery (LLM handles tool failures gracefully)
- Claude Desktop integration (Phase 5)
- Comprehensive test coverage (Phase 6)
- Streaming progress updates

## Critical Path to MVP

**Must complete in order:**
1. Phase 1 (LLM Client) - blocks everything
2. Phase 3 (Tool Registry) - can be parallel with Phase 1
3. Phase 2 (ChatLlmSampler) - needs both Phase 1 and 3
4. Phase 4 (Commands) - needs Phase 2

**Can skip for MVP:**
- Phase 5 (MCP Server) - only needed for Claude Desktop
- Phase 6 (Documentation) - complete after MVP works

**Parallel work opportunities:**
- Phase 1 and Phase 3 can be done simultaneously
- Phase 5 can start once Phase 3 is done
- Tests can be written alongside implementation
