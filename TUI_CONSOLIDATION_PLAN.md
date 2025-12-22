# TUI Consolidation Plan

## Problem Statement

We have two parallel TUI implementations causing code duplication and architectural confusion:

1. **botticelli_tui** - Early implementation with direct tool integration
2. **botticelli_chat** - Later implementation with MCP host capabilities

This duplication leads to:
- Inconsistent behavior
- Duplicated logic
- Confusion about which codebase to use
- Difficulty maintaining both

## Architectural Vision

**Single Unified Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                     botticelli_chat                          │
│  (MCP Host + Conversation Management + Tool Orchestration)   │
│                                                              │
│  • McpHost (manages multiple McpClient connections)          │
│  • Conversation state management                             │
│  • LLM integration via trait interfaces                      │
│  • Tool execution flow                                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Implements traits from
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   botticelli_interface                       │
│                                                              │
│  • ChatHost trait - conversation management interface        │
│  • ToolExecutor trait - tool execution interface             │
│  • McpTransport trait - transport abstraction                │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Used by
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     botticelli_tui                           │
│           (Pure UI Layer - No Business Logic)                │
│                                                              │
│  • Ratatui-based terminal UI                                │
│  • Event handling (keyboard, mouse)                          │
│  • Screen rendering                                          │
│  • Uses ChatHost trait (doesn't know about internals)        │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Define Clean Trait Boundaries ✅ CURRENT

**Goal:** Extract trait interfaces that separate UI from business logic

**Tasks:**
1. ✅ Define `ChatHost` trait in `botticelli_interface`
   - `send_message(content: String) -> Result<Response>`
   - `get_conversation_history() -> Vec<Message>`
   - `list_available_tools() -> Vec<ToolDefinition>`
   - `get_conversation_status() -> Status`

2. ✅ Define `ToolExecutor` trait in `botticelli_interface`
   - `execute_tool(name: String, args: Value) -> Result<Value>`
   - Already exists, verify completeness

3. ✅ Define `McpTransport` trait in `botticelli_interface`
   - `connect() -> Result<Connection>`
   - `send_request(req: Request) -> Result<Response>`
   - Already defined, verify implementation

**Success Criteria:**
- Traits defined in `botticelli_interface`
- No dependency on concrete implementations
- Clear separation of concerns

### Phase 2: Implement Traits in botticelli_chat

**Goal:** Make `botticelli_chat` implement the trait interfaces

**Tasks:**
1. Implement `ChatHost` for the main chat coordinator
   - Wire up existing `McpHost` and LLM integration
   - Expose conversation management through trait
   - Handle tool execution flow

2. Ensure `McpHost` properly manages multiple `McpClient` connections
   - Each server gets its own client
   - Tool routing to correct server
   - Proper error handling and fallback

3. Complete tool execution loop
   - Detect tool calls in LLM responses
   - Execute via MCP
   - Return results to LLM
   - Continue until completion

**Success Criteria:**
- `botticelli_chat` compiles with trait implementations
- All tests pass
- Tool execution flow works end-to-end

### Phase 3: Refactor botticelli_tui to Use Traits

**Goal:** Remove all business logic from `botticelli_tui`, make it pure UI

**Tasks:**
1. Remove direct tool implementations from `botticelli_tui`
   - Delete `ExecuteCarouselTool` and similar
   - Remove MCP client initialization
   - Remove LLM integration

2. Refactor TUI to use `ChatHost` trait
   - Accept `impl ChatHost` as dependency
   - Call trait methods for all operations
   - Never directly touch MCP, tools, or LLM

3. Clean up state management
   - UI state only (scroll, selection, input)
   - No conversation state (owned by ChatHost)
   - No tool registry (owned by ChatHost)

**Success Criteria:**
- `botticelli_tui` has zero business logic
- All functionality works through traits
- No direct dependencies on MCP, tools, or LLM crates

### Phase 4: Integration and Testing

**Goal:** Wire everything together and validate

**Tasks:**
1. Update binary entry points
   - `botticelli-chat` binary creates ChatHost and TUI
   - Pass ChatHost implementation to TUI
   - Proper initialization and shutdown

2. Add comprehensive instrumentation
   - Trace conversation flow
   - Track tool execution
   - Monitor MCP connections
   - Log all errors with context

3. Test suite
   - Unit tests for each trait implementation
   - Integration tests for full flow
   - Manual testing of UI

**Success Criteria:**
- `just chat` launches successfully
- All tools visible and functional
- Conversation flow works end-to-end
- Logs show clear execution trace

### Phase 5: Cleanup and Documentation

**Goal:** Remove old code, update docs

**Tasks:**
1. Delete deprecated code
   - Old hardcoded tool registrations
   - Parallel implementations
   - Unused utility functions

2. Update documentation
   - Architecture diagrams
   - Developer guide
   - API documentation

3. Final audit
   - Run `just check-all`
   - Fix all warnings
   - Verify feature gates

**Success Criteria:**
- Zero compilation warnings
- All tests passing
- Documentation current
- Clean git history

## Key Principles

1. **Program to Interfaces, Not Implementations**
   - TUI knows nothing about MCP internals
   - Chat knows nothing about UI details
   - Traits enforce boundaries

2. **Single Responsibility**
   - TUI: Rendering and input
   - Chat: Conversation and orchestration
   - MCP: Tool transport

3. **No Code Duplication**
   - One place for each concern
   - Shared logic in libraries
   - DRY principle

4. **Observability First**
   - Instrument all boundaries
   - Log state transitions
   - Trace execution flow

## Migration Path

### Immediate (Phase 1)
- Define traits ✅
- Document boundaries ✅

### Short-term (Phases 2-3)
- Implement traits in chat
- Refactor TUI to use traits
- Remove duplication

### Long-term (Phases 4-5)
- Integration testing
- Documentation
- Cleanup

## Success Metrics

- **Code Quality:** Zero warnings, all tests pass
- **Architecture:** Clear separation, no circular dependencies
- **Functionality:** All features work as before
- **Maintainability:** Single codebase for each concern
- **Observability:** Complete execution traces

---

**Status:** Phase 1 in progress
**Last Updated:** 2025-12-22
**Tracking:** See PLANNING_INDEX.md
