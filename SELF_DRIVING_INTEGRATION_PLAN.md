# Self-Driving LLM Orchestration: Integration Plan

**Status**: In Progress  
**Created**: 2025-12-15  
**Last Updated**: 2025-12-15

## Executive Summary

This plan addresses the critical gaps in Botticelli's self-driving orchestration pipeline and establishes boundary-based integration testing to ensure reliable data flow through the system.

## Current State Analysis

### ✅ What We Have (Functional)

1. **MCP Server Infrastructure** (`botticelli_mcp`)
   - Tool registration system
   - Request handling
   - SSE/stdio transport

2. **Internal Tools** (`botticelli_mcp_client`)
   - Narrative generation tools
   - Database operation tools
   - Tool registry and execution
   - Approval system
   - Retry with backoff

3. **Core Capabilities**
   - Narrative elicitation system (Q&A based)
   - Narrative generation with sampling
   - LLM client abstraction (multiple providers)
   - PostgreSQL storage
   - Discord bot interface
   - TUI interface

4. **Observability**
   - Distributed tracing (Jaeger)
   - Metrics (Prometheus)
   - Structured logging

### ❌ Critical Gaps

1. **No MCP Integration Layer**
   - Tools exist but aren't exposed via MCP server
   - LLMs can't discover/call Botticelli capabilities
   - No unified orchestration interface

2. **Missing LLM Orchestrator**
   - No component to route LLM requests through MCP
   - No multi-step workflow coordination
   - No context management across tool calls

3. **Incomplete Elicitation → Narrative Pipeline**
   - Elicitation gathers requirements
   - No automated handoff to narrative generation
   - Manual intervention required

4. **No External Tool Integration**
   - Can't leverage ecosystem MCP servers (filesystem, etc.)
   - `ExternalMcpClient` implemented but not integrated

5. **Missing End-to-End Tests**
   - No integration tests for pipeline boundaries
   - Can't verify data flow between segments

---

## Phase 1: MCP Tool Integration (Priority: CRITICAL)

**Goal**: Expose internal tools via MCP server so LLMs can orchestrate Botticelli

### Step 1.1: Register Internal Tools with MCP Server

**Tasks**:
1. Create `McpToolBridge` in `botticelli_mcp`
2. Convert `NarrativeTool` trait methods to MCP tool definitions
3. Register tools with `pmcp::Server` on startup
4. Map MCP requests → `InternalToolExecutor` calls

**Files**:
- `crates/botticelli_mcp/src/tool_bridge.rs` (new)
- `crates/botticelli_mcp/src/lib.rs` (export)
- `crates/botticelli_mcp/src/server.rs` (integrate)

**Success Criteria**:
- [ ] MCP server exposes `tools/list` with narrative tools
- [ ] MCP `tools/call` executes internal narrative generation
- [ ] Integration test: MCP client → tool execution → success response

**Integration Test** (`tests/mcp_tool_bridge_test.rs`):
```rust
// Test: MCP Server ↔ Internal Tool Executor
// Verifies: Tool registration and execution boundary
#[tokio::test]
async fn test_mcp_exposes_internal_tools() {
    let server = create_test_mcp_server().await;
    let response = server.list_tools().await.unwrap();
    assert!(response.tools.iter().any(|t| t.name == "generate_narrative"));
}

#[tokio::test]
async fn test_mcp_executes_internal_tool() {
    let server = create_test_mcp_server().await;
    let result = server.call_tool("generate_narrative", json!({
        "prompt": "test narrative",
        "config": {...}
    })).await.unwrap();
    assert!(result.content.contains("narrative"));
}
```

---

## Phase 2: LLM Orchestrator (Priority: HIGH)

**Goal**: Component that routes LLM requests through MCP tools with context management

### Step 2.1: Create Orchestrator Core

**Tasks**:
1. Create `botticelli_orchestrator` crate
2. Implement `LlmOrchestrator` with conversation state
3. Add tool call routing through `UnifiedMcpClient`
4. Integrate with LLM client abstraction

**Files**:
- `crates/botticelli_orchestrator/` (new crate)
- `crates/botticelli_orchestrator/src/orchestrator.rs`
- `crates/botticelli_orchestrator/src/conversation.rs`

**Success Criteria**:
- [ ] Orchestrator maintains conversation context
- [ ] Routes tool calls to appropriate MCP tools
- [ ] Handles multi-turn LLM interactions
- [ ] Aggregates tool results back to LLM

**Integration Test** (`tests/orchestrator_mcp_test.rs`):
```rust
// Test: LLM Orchestrator ↔ MCP Client
// Verifies: Tool call routing boundary
#[tokio::test]
async fn test_orchestrator_routes_tool_calls() {
    let orchestrator = create_test_orchestrator().await;
    let response = orchestrator.process_message(
        "Generate a narrative about a hero"
    ).await.unwrap();
    
    // Should have called MCP tool internally
    assert!(orchestrator.tool_calls_made() > 0);
    assert!(response.contains("narrative"));
}
```

### Step 2.2: Multi-Step Workflow Support

**Tasks**:
1. Add workflow state machine
2. Implement step chaining (elicitation → generation → storage)
3. Add error recovery between steps

**Success Criteria**:
- [ ] Can execute multi-step workflows
- [ ] State persists between steps
- [ ] Failures in one step don't crash pipeline

**Integration Test** (`tests/orchestrator_workflow_test.rs`):
```rust
// Test: Orchestrator Workflow State Management
// Verifies: Multi-step coordination boundary
#[tokio::test]
async fn test_multi_step_workflow() {
    let orchestrator = create_test_orchestrator().await;
    
    // Step 1: Elicitation
    orchestrator.start_workflow("create_narrative").await.unwrap();
    let q1 = orchestrator.next_step().await.unwrap();
    assert_eq!(q1.step_type, "elicitation");
    
    // Step 2: Generation (triggered by elicitation completion)
    orchestrator.complete_step(q1.id, "hero story").await.unwrap();
    let gen = orchestrator.next_step().await.unwrap();
    assert_eq!(gen.step_type, "generation");
}
```

---

## Phase 2.5: Elicitation Tool Integration ✅ COMPLETE

**Goal**: Expose conversational narrative elicitation as MCP tools for LLM orchestration.

### Step 2.5.1: Elicitation Tool Implementation ✅

**Completed**:
- ✅ `ElicitationRegistry` for managing active sessions
- ✅ `CreateElicitationSessionTool` - Initialize new elicitation session  
- ✅ `ElicitMetadataTool` - Set/update narrative metadata
- ✅ `ElicitActTool` - Add/update acts during elicitation
- ✅ `FinalizeElicitationTool` - Generate TOML from session

**Files Modified**:
- `crates/botticelli_mcp_client/src/tools/elicitation.rs` (new)
- `crates/botticelli_mcp_client/src/tools/mod.rs` (updated)
- `crates/botticelli_mcp_client/Cargo.toml` (added uuid dependency)

**Success Criteria**:
- ✅ All elicitation tools registered in `register_internal_tools()`
- ✅ Tools use proper `ToolHandler` trait with `tool_info()` method
- ✅ Returns `Vec<Content>` for MCP compatibility
- ✅ Session state management with UUID tracking
- ✅ TOML generation from elicitation state

**Integration Points**:
- LLMs can now orchestrate full narrative creation conversations
- Session-based tracking allows multi-turn elicitation
- Direct integration with existing narrative generation tools
- Elicitation tools available alongside narrative CRUD tools

---

## Phase 3: Elicitation → Narrative Pipeline (Priority: HIGH)

**Goal**: Automated handoff from elicitation to narrative generation

### Step 3.1: Pipeline Coordinator

**Tasks**:
1. Create `NarrativePipeline` in `botticelli_narrative`
2. Connect elicitation output → narrative input
3. Add configuration mapping
4. Integrate with orchestrator

**Files**:
- `crates/botticelli_narrative/src/pipeline.rs` (new)
- `crates/botticelli_narrative/src/elicitation.rs` (update)

**Success Criteria**:
- [ ] Elicitation answers → structured narrative config
- [ ] Automated generation trigger
- [ ] No manual intervention required

**Integration Test** (`tests/elicitation_narrative_pipeline_test.rs`):
```rust
// Test: Elicitation System ↔ Narrative Generator
// Verifies: Data transformation boundary
#[tokio::test]
async fn test_elicitation_triggers_narrative() {
    let pipeline = create_test_pipeline().await;
    
    // Complete elicitation
    let answers = complete_test_elicitation().await;
    
    // Should auto-trigger narrative generation
    let narrative = pipeline.process_elicitation(answers).await.unwrap();
    assert!(narrative.content.len() > 0);
    assert_eq!(narrative.metadata.source, "elicitation");
}

#[tokio::test]
async fn test_elicitation_config_mapping() {
    let answers = ElicitationAnswers {
        theme: "fantasy".to_string(),
        length: "short".to_string(),
        style: "epic".to_string(),
    };
    
    let config = map_to_narrative_config(answers);
    assert_eq!(config.genre, Genre::Fantasy);
    assert_eq!(config.target_length, 500);
}
```

---

## Phase 4: External Tool Integration (Priority: MEDIUM)

**Goal**: Connect to external MCP servers (filesystem, browser, etc.)

### Step 4.1: External Server Registry

**Tasks**:
1. Add external server configuration
2. Implement connection pooling
3. Register external tools with orchestrator
4. Add tool discovery

**Files**:
- `crates/botticelli_mcp_client/src/external_registry.rs` (new)
- Configuration: `chat.toml` external servers section

**Success Criteria**:
- [ ] Can connect to filesystem MCP server
- [ ] External tools appear in orchestrator tool list
- [ ] LLM can call external tools seamlessly

**Integration Test** (`tests/external_mcp_integration_test.rs`):
```rust
// Test: External MCP Client ↔ External Server
// Verifies: External connection boundary
#[tokio::test]
#[ignore] // Requires external server running
async fn test_connect_to_filesystem_server() {
    let client = ExternalMcpClient::new(RetryConfig::default());
    let connection = client.connect_stdio(
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    ).await.unwrap();
    
    let tools = client.list_tools(&connection).await.unwrap();
    assert!(tools.iter().any(|t| t.name == "read_file"));
}
```

---

## Phase 5: End-to-End Integration Tests (Priority: HIGH)

**Goal**: Verify complete pipeline from user input → stored narrative

### Step 5.1: Full Pipeline Test

**Integration Test** (`tests/end_to_end_pipeline_test.rs`):
```rust
// Test: Complete Self-Driving Pipeline
// Verifies: All boundaries in sequence
#[tokio::test]
async fn test_full_self_driving_pipeline() {
    let system = create_test_system().await;
    
    // 1. User message → Orchestrator
    let user_msg = "Create a fantasy narrative about a hero";
    let response = system.process_user_message(user_msg).await.unwrap();
    
    // 2. Orchestrator → MCP Client → Internal Tools
    assert!(system.tool_calls_logged() > 0);
    
    // 3. Elicitation (if needed) → Narrative Generation
    let narrative = system.get_generated_narrative().await.unwrap();
    assert!(narrative.content.len() > 0);
    
    // 4. Narrative → Database Storage
    let stored = system.db.get_narrative(narrative.id).await.unwrap();
    assert_eq!(stored.id, narrative.id);
    
    // 5. Verify observability
    assert!(system.traces_recorded() > 0);
}
```

---

## Integration Test Matrix

| Boundary | Test File | What It Verifies |
|----------|-----------|------------------|
| MCP Server ↔ Internal Tools | `mcp_tool_bridge_test.rs` | Tool registration and execution |
| Orchestrator ↔ MCP Client | `orchestrator_mcp_test.rs` | Tool call routing |
| Orchestrator ↔ Workflow State | `orchestrator_workflow_test.rs` | Multi-step coordination |
| Elicitation ↔ Narrative Gen | `elicitation_narrative_pipeline_test.rs` | Data transformation |
| External Client ↔ External Server | `external_mcp_integration_test.rs` | External connections |
| Narrative Gen ↔ Database | `narrative_storage_test.rs` | Persistence boundary |
| Complete Pipeline | `end_to_end_pipeline_test.rs` | All boundaries |

---

## Implementation Order

1. **Phase 1** (Week 1): MCP tool integration - critical blocker
2. **Phase 5.1** (Week 1): Basic boundary tests as we build
3. **Phase 2.1** (Week 2): Orchestrator core
4. **Phase 3.1** (Week 2): Elicitation pipeline
5. **Phase 2.2** (Week 3): Multi-step workflows
6. **Phase 4.1** (Week 3): External tools
7. **Phase 5** (Week 4): Complete end-to-end tests

---

## Success Metrics

### Phase 1 Complete
- [ ] `just test-package botticelli_mcp` passes with tool bridge tests
- [ ] Manual MCP client can list and call internal tools
- [ ] Zero compilation warnings

### Phase 2 Complete
- [ ] Orchestrator handles multi-turn conversations
- [ ] Tool calls routed correctly (verified by integration tests)
- [ ] Conversation context maintained across turns

### Phase 3 Complete
- [ ] Elicitation automatically triggers narrative generation
- [ ] Zero manual intervention in happy path
- [ ] Integration test verifies data mapping

### Phase 4 Complete
- [ ] Can connect to ≥1 external MCP server
- [ ] External tools callable from orchestrator
- [ ] Connection pooling prevents resource exhaustion

### Phase 5 Complete
- [ ] End-to-end test covers full pipeline
- [ ] All boundary tests passing
- [ ] Observability traces show complete flow

---

## Risk Mitigation

### Risk: Feature deletion during refactoring
**Mitigation**: Integration tests catch missing functionality immediately

### Risk: Complex async orchestration bugs
**Mitigation**: Boundary tests isolate problem segments

### Risk: External server unavailability
**Mitigation**: Use `#[ignore]` for external deps, document setup

### Risk: Token budget exhaustion in tests
**Mitigation**: Mock LLM responses for integration tests, real calls only in E2E

---

## Notes

- All tests go in `tests/` directory (per CLAUDE.md)
- Use builders for all test setup
- Integration tests should be fast (<5s per test)
- Mark expensive tests with `#[ignore]` and `#[cfg_attr(not(feature = "api"), ignore)]`
- Each test should verify ONE boundary clearly
- Test names should describe what boundary they verify

---

## Tracking

Add to `PLANNING_INDEX.md` and `PLANNING_TRACKER.md` once approved.
