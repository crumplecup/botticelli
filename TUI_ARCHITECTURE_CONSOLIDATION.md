# TUI Architecture Consolidation Plan

## Problem Statement

We have **two parallel TUI implementations** causing duplication and confusion:

1. **`botticelli_tui`** - Original TUI for narrative elicitation (older)
2. **`botticelli_chat`** - Chat-based TUI with MCP host capabilities (newer)

This creates:
- Duplicate MCP client/host code
- Confusion about which is the "real" implementation
- Competing tool registration patterns
- Difficult debugging due to unclear boundaries

## Architectural Vision

**Single Source of Truth Architecture:**

```
botticelli_interface (traits)
    ↓
botticelli_chat (MCP Host, LLM integration, business logic)
    ↓
botticelli_tui (UI layer only - implements display traits)
```

### Separation of Concerns

1. **`botticelli_interface`**: Define traits
   - `McpHost` trait (manages multiple MCP clients)
   - `ChatSession` trait (conversation management)
   - `ToolExecutor` trait (tool execution abstraction)

2. **`botticelli_chat`**: Business logic (trait-gated)
   - MCP host implementation
   - LLM provider integration with fallback
   - Tool execution loop
   - Session management
   - NO UI code

3. **`botticelli_tui`**: UI layer only
   - Renders chat interface using `ratatui`
   - Handles keyboard input
   - Displays messages, tool calls, results
   - Depends on `botticelli_chat` traits
   - NO business logic

## Implementation Phases

### Phase 1: Define Interface Traits ✅ (Partially done)

**Task 1.1**: Define `ChatHost` trait in `botticelli_interface`
```rust
pub trait ChatHost {
    fn send_message(&mut self, message: String) -> Result<Response>;
    fn list_tools(&self) -> Vec<ToolDefinition>;
    fn execute_tool(&mut self, call: ToolCall) -> Result<ToolResult>;
}
```

**Task 1.2**: Define `ChatSession` trait
```rust
pub trait ChatSession {
    fn add_message(&mut self, role: Role, content: String);
    fn get_history(&self) -> &[Message];
    fn clear(&mut self);
}
```

**Success Criteria**:
- Traits compile independently
- No concrete implementations in interface crate

---

### Phase 2: Consolidate Business Logic in `botticelli_chat`

**Task 2.1**: Move MCP host logic from `botticelli_tui` to `botticelli_chat`
- ✅ Already have `McpHost` in `botticelli_chat`
- ❌ Remove duplicate from `botticelli_tui`

**Task 2.2**: Implement `ChatHost` trait in `botticelli_chat`
```rust
impl ChatHost for McpHost {
    fn send_message(&mut self, message: String) -> Result<Response> {
        // LLM generation with tool calling loop
    }
}
```

**Task 2.3**: Move tool execution loop to `botticelli_chat`
- Currently fragmented between TUI and chat
- Centralize in chat with proper observability

**Success Criteria**:
- All business logic in `botticelli_chat`
- No MCP/LLM code in `botticelli_tui`
- Tests pass

---

### Phase 3: Refactor `botticelli_tui` to Pure UI

**Task 3.1**: Remove business logic from `botticelli_tui`
- Delete: MCP client code
- Delete: Tool registration
- Delete: LLM integration
- Keep: UI rendering, input handling

**Task 3.2**: Make `botticelli_tui` depend on `ChatHost` trait
```rust
pub struct ChatApp<H: ChatHost> {
    host: H,
    messages: Vec<DisplayMessage>,
    input: String,
}
```

**Task 3.3**: Update `botticelli_tui` binary to use `botticelli_chat`
```rust
let host = McpHost::new(config)?;
let app = ChatApp::new(host);
app.run()?;
```

**Success Criteria**:
- `botticelli_tui` has zero business logic
- All functionality works through traits
- Both TUI binaries work identically

---

### Phase 4: Eliminate Duplication

**Task 4.1**: Audit for duplicate code
- Tool definitions
- Message types
- Configuration loading

**Task 4.2**: Consolidate in appropriate crates
- Types → `botticelli_core`
- Config → `botticelli_chat`
- Traits → `botticelli_interface`

**Task 4.3**: Remove deprecated code
- Old MCP client implementations
- Hardcoded tool lists
- Parallel initialization paths

**Success Criteria**:
- Zero code duplication
- Single registration path for tools
- Clear separation of concerns

---

### Phase 5: Add Comprehensive Observability

**Task 5.1**: Instrument all public functions in `botticelli_chat`
```rust
#[tracing::instrument(skip(self))]
pub fn send_message(&mut self, message: String) -> Result<Response>
```

**Task 5.2**: Add chain of custody logging
- Tool discovery: "Discovered 32 tools from MCP server"
- Tool filtering: "Passing 32 tools to LLM"
- Tool calling: "LLM requested tool: create_actor"
- Tool execution: "Executing tool via MCP: create_actor"
- Tool result: "Tool returned success: {...}"

**Task 5.3**: Add span hierarchy
```
chat_session
  ├─ send_message
  │   ├─ discover_tools
  │   ├─ generate_with_tools (LLM)
  │   └─ execute_tool_loop
  │       ├─ execute_tool (MCP)
  │       └─ generate_with_results (LLM)
```

**Success Criteria**:
- Can trace entire flow from logs
- Know exactly where failures occur
- Performance metrics available

---

## Current Status

- ❌ Phase 1: Partially done (traits exist but incomplete)
- ❌ Phase 2: Major gaps (tool loop not implemented)
- ❌ Phase 3: Not started (TUI still has business logic)
- ❌ Phase 4: Not started (lots of duplication)
- ⚠️  Phase 5: Partial (some instrumentation exists)

## Next Steps

1. Complete Phase 2: Wire up tool execution loop in `botticelli_chat`
2. Add observability (Phase 5) to see what's broken
3. Run chat, read logs, fix gaps
4. Move to Phase 3: Refactor TUI to pure UI layer

---

## Benefits of This Architecture

1. **Single source of truth** - no parallel implementations
2. **Testability** - business logic separate from UI
3. **Flexibility** - can add web UI, CLI without duplicating logic
4. **Maintainability** - clear boundaries, trait-based design
5. **Observability** - instrumented business logic independent of UI
