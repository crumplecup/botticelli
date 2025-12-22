# TUI Architecture Consolidation Strategy

**Status**: Planning
**Created**: 2025-12-22
**Goal**: Eliminate parallel TUI implementations and establish clean trait-based architecture

## Current Problem

### Parallel Implementations
1. **botticelli_tui** - Early TUI implementation with embedded chat logic
2. **botticelli_chat** - Later TUI with MCP host capabilities
3. **Result**: Duplicated code, confusion, maintenance nightmare

### Architecture Issues
- Mixed concerns (UI + business logic)
- Duplicate MCP client initialization
- Unclear separation of responsibilities
- No trait boundaries between layers

## Target Architecture

### Layer 1: Interface Traits (botticelli_interface)
```
McpHost - Manages multiple MCP clients
McpTransport - Abstraction for HTTP/Stdio
ChatBackend - Trait for chat implementations
```

### Layer 2: Chat Backend (botticelli_chat)
```
ChatSession - Implements ChatBackend
- LLM provider management
- Tool execution loop
- MCP host integration
- Message history
- NO UI CODE
```

### Layer 3: TUI Frontend (botticelli_tui)
```
ChatTui - Ratatui-based UI
- Depends on ChatBackend trait
- Rendering only
- Event handling
- NO business logic
- NO MCP client code
```

## Implementation Plan

### Phase 1: Define Trait Boundaries
**Status**: Not Started

#### Task 1.1: Create ChatBackend Trait
- [ ] Define in `botticelli_interface`
- [ ] Methods: `send_message()`, `get_history()`, `list_tools()`, `execute_tool()`
- [ ] Document contract

#### Task 1.2: Create TuiRenderer Trait (Optional)
- [ ] Define rendering interface
- [ ] Allows multiple TUI implementations

**Success Criteria**: Traits compile, documented

---

### Phase 2: Refactor botticelli_chat
**Status**: Not Started

#### Task 2.1: Extract Business Logic
- [ ] Remove all ratatui dependencies
- [ ] Move UI code to botticelli_tui
- [ ] Keep: ChatSession, LLM provider, tool execution

#### Task 2.2: Implement ChatBackend Trait
- [ ] ChatSession implements ChatBackend
- [ ] All MCP host logic here
- [ ] Standalone, testable

#### Task 2.3: Add Comprehensive Instrumentation
- [ ] `#[tracing::instrument]` on all public functions
- [ ] Log tool discovery, execution, results
- [ ] Log LLM requests/responses

**Success Criteria**: botticelli_chat has no UI code, implements ChatBackend

---

### Phase 3: Refactor botticelli_tui
**Status**: Not Started

#### Task 3.1: Remove Duplicate Business Logic
- [ ] Delete embedded chat logic
- [ ] Delete MCP client initialization
- [ ] Keep only UI rendering

#### Task 3.2: Depend on ChatBackend Trait
- [ ] Accept `Box<dyn ChatBackend>` in constructor
- [ ] All business operations delegate to backend
- [ ] Pure UI layer

#### Task 3.3: Create TUI Binary
- [ ] `src/bin/tui.rs` or similar
- [ ] Initialize ChatSession from botticelli_chat
- [ ] Pass to ChatTui
- [ ] Run event loop

**Success Criteria**: botticelli_tui has no business logic, depends on trait

---

### Phase 4: Testing & Validation
**Status**: Not Started

#### Task 4.1: Unit Tests
- [ ] Test ChatSession independently (no UI)
- [ ] Test TUI rendering with mock backend
- [ ] Verify trait boundaries

#### Task 4.2: Integration Tests
- [ ] End-to-end tool execution
- [ ] LLM conversation flow
- [ ] Error handling

#### Task 4.3: Run just check-features
- [ ] Verify feature gates
- [ ] Fix any warnings

**Success Criteria**: All tests pass, zero warnings

---

### Phase 5: Documentation & Cleanup
**Status**: Not Started

#### Task 5.1: Update Documentation
- [ ] Architecture diagrams
- [ ] Usage examples
- [ ] API docs

#### Task 5.2: Remove Dead Code
- [ ] Delete unused modules
- [ ] Clean up imports
- [ ] Remove deprecated code

**Success Criteria**: Clean codebase, clear documentation

---

## File Structure (Target)

```
botticelli_interface/
  src/
    lib.rs
    chat_backend.rs     # NEW: ChatBackend trait
    mcp_transport.rs
    
botticelli_chat/
  src/
    lib.rs
    chat_session.rs     # Implements ChatBackend
    mcp_host.rs
    # NO UI CODE
    
botticelli_tui/
  src/
    lib.rs              # TUI components only
    chat_ui.rs          # Rendering logic
  bin/
    tui.rs              # Binary that wires chat + tui
    
crates/botticelli/
  src/
    bin/
      chat.rs           # User-facing binary
```

## Benefits

1. **Clear Separation**: UI vs business logic
2. **Testability**: Chat backend testable without UI
3. **Flexibility**: Multiple UI implementations possible
4. **Maintainability**: Single source of truth for each concern
5. **Observability**: Instrumentation at layer boundaries

## Risks & Mitigation

**Risk**: Breaking existing functionality during refactor
**Mitigation**: Incremental changes, commit after each working phase

**Risk**: Trait boundaries too rigid
**Mitigation**: Start with minimal traits, expand as needed

**Risk**: Feature gate complexity
**Mitigation**: Keep tracing instrumentation with full syntax

## Next Steps

1. Review this plan
2. Start Phase 1, Task 1.1
3. Commit after each task
4. Update this document with progress
