# TODO/Landmine Audit - Botticelli Codebase

**Date:** 2025-12-15  
**Status:** Comprehensive audit of all TODOs, FIXMEs, and potential landmines

## Summary

Total items identified: ~100+
- **Critical (Blockers)**: 12
- **High Priority**: 28
- **Medium Priority**: 35
- **Low Priority (Nice-to-have)**: 25+

---

## 🚨 CRITICAL - Blockers for Production

### 1. MCP Client Transport Layer (BLOCKER)

**Location:** `crates/botticelli_mcp_client/src/transport.rs:53-60`

```rust
// TODO: Implement actual stdio communication
todo!("Implement stdio transport send")
// TODO: Implement graceful shutdown
todo!("Implement stdio transport close")
```

**Impact:** MCP client cannot communicate with external servers - entire external MCP integration is non-functional.

**Priority:** P0 - Must fix immediately  
**Effort:** Medium (2-4 hours)  
**Dependencies:** pmcp integration we just completed

---

### 2. LLM Adapter Implementations (BLOCKER)

**Location:** `crates/botticelli_mcp_client/src/llm_adapter.rs`

```rust
Line 177: // TODO: Implement Anthropic API call
Line 227: // TODO: Implement Gemini API call
Line 277: // TODO: Implement Groq API call
Line 327: // TODO: Implement Ollama API call
```

**Impact:** MCP client cannot make any LLM calls - agentic loop is broken.

**Priority:** P0 - Must fix immediately  
**Effort:** Large (8-12 hours for all 4)  
**Dependencies:** Need to adapt existing `botticelli_models` clients

**Note:** We already have working LLM clients in `botticelli_models` - need to wire them into the adapter.

---

### 3. Tool Call Parsing (BLOCKER)

**Location:** `crates/botticelli_mcp_client/src/client.rs:103`

```rust
// TODO: Implement actual tool call parsing
```

**Impact:** Cannot extract and execute tool calls from LLM responses.

**Priority:** P0 - Must fix immediately  
**Effort:** Medium (3-5 hours)  
**Dependencies:** LLM response format understanding

---

### 4. Tool Execution Integration (BLOCKER)

**Location:** `crates/botticelli_mcp_client/src/tool_executor.rs:45`

```rust
// TODO: Actual tool execution will integrate with MCP server
```

**Impact:** Cannot execute tools from external MCP servers.

**Priority:** P0 - Must fix immediately  
**Effort:** Medium (4-6 hours)  
**Dependencies:** Transport layer completion

---

## 🔴 HIGH PRIORITY - Core Functionality

### 5. Security Checks in Discord Commands

**Location:** `crates/botticelli_social/src/discord/commands.rs` (11 occurrences)

Lines: 1571, 1693, 1833, 2552, 2939, 3096, 3173, 3248, 3355, 3421, 3497

```rust
// TODO: Security check
```

**Impact:** Discord commands lack permission validation - **SECURITY RISK**.

**Priority:** P1 - Before any Discord deployment  
**Effort:** Medium (6-8 hours for all)  
**Risk:** Unauthorized users could execute privileged commands

---

### 6. Gemini WebSocket Handshake Failures

**Location:** `crates/botticelli_models/tests/gemini_live_*_test.rs`

Multiple test files with:
```rust
#[ignore = "TODO: Fix WebSocket handshake failure"]
```

**Impact:** Gemini live streaming API is broken - cannot use real-time features.

**Priority:** P1 - If we want Gemini streaming support  
**Effort:** Large (8-16 hours, requires debugging WebSocket protocol)  
**Status:** 8+ tests disabled

---

### 7. Context Summarization Missing

**Location:** `crates/botticelli_mcp_client/src/context.rs:116`

```rust
// TODO: Use LLM to summarize old messages when history gets too long
```

**Impact:** Context window will eventually overflow, causing failures in long conversations.

**Priority:** P1 - Before production use  
**Effort:** Medium (4-6 hours)  
**Mitigation:** Currently has basic truncation, but loses context quality

---

### 8. Token Usage & Cost Tracking Incomplete

**Location:** Multiple files in `botticelli_narrative`, `botticelli_database`

```rust
token_usage: None, // TODO: Load from database once schema is updated
estimated_cost_usd: None, // TODO: Calculate from token_usage + model pricing
total_cost_usd: None, // TODO: Calculate from total_token_usage + model pricing
```

**Impact:** Cannot track API costs or token usage accurately.

**Priority:** P1 - Before production (cost management)  
**Effort:** Medium (6-8 hours)  
**Files:** 
- `crates/botticelli_database/src/narrative_conversions.rs:215, 256`
- `crates/botticelli_narrative/src/executor.rs:452, 657, 824, 881`

---

## 🟡 MEDIUM PRIORITY - Important but Not Blocking

### 9. TUI Widget Stubs

**Location:** `crates/botticelli_chat/src/tui/widgets/`

```rust
// tab_bar.rs:24 - TODO: Implement rendering
// status_bar.rs:24 - TODO: Implement rendering  
// modal.rs:24 - TODO: Implement rendering
```

**Impact:** TUI missing visual polish and some UI features.

**Priority:** P2 - UX improvement  
**Effort:** Medium (4-6 hours total)

---

### 10. TUI Event Handlers Stubbed

**Location:** `crates/botticelli_chat/src/tui/events.rs`

```rust
Line 125: // TODO: Launch external editor
Line 133: // TODO: Execute narrative
Line 159: // TODO: Confirm and delete
Line 167: // TODO: Launch creation wizard
Line 175: // TODO: Refresh narratives from disk
```

**Impact:** Several TUI features non-functional.

**Priority:** P2 - UX completeness  
**Effort:** Medium (6-8 hours)

---

### 11. MCP Sampling Tools Incomplete

**Location:** `crates/botticelli_mcp/src/tools/`

```rust
// sampling.rs:52 - TODO: Extract narrative from session after LLM tool calling
// sampling.rs:89 - TODO: Apply refinements from LLM tool calling
```

**Impact:** MCP sampling tools don't fully integrate with LLM feedback.

**Priority:** P2 - Enhanced AI collaboration  
**Effort:** Medium (4-6 hours)

---

### 12. Narrative Test Files Missing

**Location:** `crates/botticelli_social/tests/`

```rust
#[ignore = "TODO: Replace narrative source - test files don't exist in expected location"]
```

**Files:**
- `discord_command_test.rs:142`
- `state_integration_test.rs:29`
- `discord_write_operations_test.rs:11`
- `discord_state_test.rs:37`

**Impact:** Cannot run integration tests for Discord narrative execution.

**Priority:** P2 - Test coverage  
**Effort:** Small (2-3 hours to create test narratives)

---

### 13. Chat Executor Stubs

**Location:** `crates/botticelli_chat/src/executor.rs`

```rust
Line 288: // For MVP, create a simple bot config stub
Line 596: // TODO: Load from file path
Line 634: // TODO: This should be replaced with actual generated content from MCP
```

**Impact:** Chat commands partially stubbed.

**Priority:** P2 - Chat functionality  
**Effort:** Small-Medium (3-5 hours)

---

## 🟢 LOW PRIORITY - Nice-to-Have

### 14. Error Recovery Tests Unimplemented

**Location:** `crates/botticelli_narrative/tests/error_recovery_test.rs`

```rust
// TODO: Test handling of missing acts
// TODO: Test empty TOC handling
// TODO: Test circular reference handling
```

**Impact:** Edge case testing incomplete.

**Priority:** P3 - Quality improvement  
**Effort:** Small (2-3 hours)

---

### 15. Parser Improvements

**Location:** `crates/botticelli_chat/tests/parser_test.rs`

```rust
Line 70: // TODO: Improve parser for bot creation commands
Line 83: // TODO: Improve parser for social scheduling commands
```

**Impact:** Parser could be more robust.

**Priority:** P3 - UX polish  
**Effort:** Small-Medium (3-4 hours)

---

### 16. Database Schema Edge Case Tests

**Location:** `crates/botticelli_database/tests/schema_edge_cases_test.rs:3`

```rust
//! TODO: Implement these tests once schema inference functions are exposed
```

**Impact:** Schema validation testing incomplete.

**Priority:** P3 - Quality  
**Effort:** Small (2-3 hours)

---

### 17. TUI State Handlers

**Location:** `crates/botticelli_tui/src/state.rs`

```rust
Line 196: // TODO: Send message to LLM and handle response
Line 208: // TODO: Handle settings input
Line 236: // TODO: Implement mouse handling
Line 242: // TODO: Implement resize handling
Line 248: // TODO: Implement periodic updates
```

**Impact:** TUI missing some event handling.

**Priority:** P3 - UX enhancement  
**Effort:** Small-Medium (4-5 hours)

---

### 18. MCP CLI Command Skeleton

**Location:** `crates/botticelli/src/cli/mcp.rs:13-15`

```rust
// TODO: Implement MCP server connection and tool discovery
// TODO: Create LLM backend adapter
// TODO: Execute agentic loop
```

**Impact:** CLI command exists but does nothing.

**Priority:** P3 - CLI completeness  
**Effort:** Large (8-12 hours) - but this overlaps with fixing the MCP client issues

---

### 19. Discord Actor Server Stubs

**Location:** `crates/botticelli_actor/src/discord_server.rs`

```rust
Line 227: // TODO: Get token from environment or config for authentication
Line 271: // TODO: Implement proper execution with database connection
```

**Impact:** Actor server needs proper config integration.

**Priority:** P3 - Architecture cleanup  
**Effort:** Small (2-3 hours)

---

### 20. Minor Implementation TODOs

Various small items:
- `botticelli_mcp/src/server.rs:116` - Resource list pre-computation (optimization)
- `botticelli_models/src/anthropic/client.rs:211` - Make timeout configurable
- `botticelli_models/src/gemini/client.rs:873` - Cache temporary value
- `botticelli_chat/src/tui/tabs/*` - Various UI action handlers
- `botticelli_interface/README.md` - Example code placeholders

**Priority:** P3-P4 - Polish  
**Effort:** Small (1-2 hours each)

---

## 🎯 Recommended Action Plan

### Phase 1: MCP Client Critical Path (P0 - 1-2 days)
1. ✅ Fix transport layer (`transport.rs` - stdio implementation)
2. ✅ Wire existing LLM clients into adapter (`llm_adapter.rs`)
3. ✅ Implement tool call parsing (`client.rs`)
4. ✅ Complete tool executor integration (`tool_executor.rs`)

**Outcome:** Functional MCP client for external servers

---

### Phase 2: Security & Safety (P1 - 1 day)
1. ✅ Implement Discord command security checks (11 locations)
2. ✅ Add context summarization to prevent overflow
3. ✅ Implement token usage and cost tracking

**Outcome:** Safe for production use, cost-aware

---

### Phase 3: Test Coverage (P2 - 1 day)
1. ✅ Create missing narrative test files
2. ✅ Implement error recovery tests
3. ✅ Add schema edge case tests

**Outcome:** Better test coverage and confidence

---

### Phase 4: TUI Completion (P2 - 1-2 days)
1. ✅ Complete widget implementations
2. ✅ Wire up event handlers
3. ✅ Complete chat executor stubs

**Outcome:** Fully functional TUI

---

### Phase 5: Gemini Streaming (P1/P2 - 2-3 days)
1. ✅ Debug WebSocket handshake
2. ✅ Re-enable 8+ streaming tests
3. ✅ Validate live API functionality

**Outcome:** Gemini streaming support (if needed)

---

### Phase 6: Polish (P3 - Ongoing)
Address low-priority items as needed for features

---

## 🔥 Immediate Next Steps

Based on our current focus on MCP client external server capability:

1. **START HERE:** Phase 1 - MCP Client Critical Path
   - These are the 4 blocking TODOs preventing external MCP server integration
   - All are in `botticelli_mcp_client` crate
   - Should take 1-2 focused days

2. **THEN:** Phase 2 Security - Discord commands
   - Before any production Discord deployment
   - 11 security check locations

3. **PARALLEL:** Document remaining items in PLANNING_TRACKER.md

---

## Notes

- Many "stub" implementations exist for feature-gated code (e.g., when no LLM backends enabled) - these are intentional, not bugs
- Some TODOs are aspirational improvements, not blockers
- The MCP client work is the most critical for your "self-driving" goal
- Security checks in Discord are critical before any production use

---

## Tracking

Add this document to `PLANNING_INDEX.md` for visibility.

Update status as items are completed.
