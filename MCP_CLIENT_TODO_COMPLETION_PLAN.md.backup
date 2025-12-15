# MCP Client TODO Completion Plan

## Audit Results

Found 7 TODOs in `botticelli_mcp_client` crate that prevent self-driving functionality:

### Critical TODOs (Block Self-Driving)

1. **tool_executor.rs:45** - Tool execution stub
   - Currently returns placeholder JSON
   - Needs to integrate with actual MCP server tool execution
   
2. **client.rs:103** - Tool call parsing
   - Returns `None` for all responses
   - Needs to parse structured LLM output for tool calls
   
3. **llm_adapter.rs:177** - Anthropic API implementation
   - Throws "Not yet implemented" error
   - Needs actual API call integration
   
4. **llm_adapter.rs:227** - Gemini API implementation
   - Throws "Not yet implemented" error
   - Needs actual API call integration
   
5. **llm_adapter.rs:277** - Groq API implementation
   - Throws "Not yet implemented" error
   - Needs actual API call integration
   
6. **llm_adapter.rs:327** - Ollama API implementation
   - Throws "Not yet implemented" error
   - Needs actual API call integration

### Nice-to-Have TODOs

7. **context.rs:116** - History summarization
   - Currently just truncates old messages
   - Would benefit from LLM-based summarization

## Architecture Analysis

### Current State

The `botticelli_mcp_client` crate has:
- ✅ **Context management** (ContextManager) - tracks conversation history
- ✅ **Tool definitions** (ToolDefinition) - describes MCP tools
- ✅ **Tool executor skeleton** (ToolExecutor) - knows about tools
- ✅ **LLM adapter trait** (LlmAdapter) - interface for LLM backends
- ✅ **Client orchestration** (McpClient) - coordinates everything
- ❌ **Actual LLM calls** - all stubbed out
- ❌ **Tool execution** - returns fake data
- ❌ **Tool call parsing** - doesn't extract from responses

### Existing Resources

The main workspace has:
- ✅ **Full LLM implementations** in `botticelli_core`, `botticelli_models`
  - Anthropic (Claude)
  - Google (Gemini)
  - Groq
  - Ollama
  - HuggingFace
- ✅ **LlmProvider trait** - standard interface
- ✅ **GenerateRequest/GenerateResponse** - request/response types
- ✅ **Tool execution** in `botticelli_mcp` via MCP tools
- ✅ **LlmSampler trait** in `botticelli_mcp` - for coordinated sampling
- ✅ **ChatLlmSampler** in `botticelli_chat` - working implementation

### Key Insight

**The `botticelli_mcp_client` crate is duplicating functionality that already exists elsewhere!**

This crate was created for "self-driving" but:
1. It has its own `LlmAdapter` trait (different from `LlmProvider`)
2. It has its own `Message` types (different from `botticelli_core::Message`)
3. It has its own `ToolCall` types (different from MCP types)
4. It doesn't reuse the existing working implementations

## Strategic Options

### Option 1: Complete the Stubs (Naive)

Implement all 6 LLM adapters by duplicating existing code:
- Copy API call logic from existing providers
- Translate between type systems
- Test everything again

**Pros:**
- Straightforward
- Keeps crate isolated

**Cons:**
- Code duplication (violates DRY)
- Maintenance burden (changes need to happen twice)
- Type translation overhead
- More code to test
- Larger binary size

**Estimated Time:** 3-4 days

### Option 2: Bridge to Existing Infrastructure (Smart)

Make `botticelli_mcp_client` delegate to existing proven code:
- Wrap `LlmProvider` implementations as `LlmAdapter`
- Translate between types at boundary
- Reuse existing API call logic
- Reuse existing tool execution

**Pros:**
- No code duplication
- Single source of truth for LLM calls
- Leverage existing test coverage
- Smaller codebase
- Easier maintenance

**Cons:**
- Type translation layer needed
- Slight indirection

**Estimated Time:** 1-2 days

### Option 3: Refactor to Unified Architecture (Best Long-Term)

Eliminate `botticelli_mcp_client` separate type system:
- Use `LlmProvider` trait directly
- Use `botticelli_core::Message` types
- Use `botticelli_mcp` tool types
- Make "self-driving" a feature of MCP server, not separate crate

**Pros:**
- Single unified architecture
- No duplication anywhere
- Clean type system
- Smaller workspace
- Self-driving as first-class feature

**Cons:**
- More invasive refactor
- Requires careful migration
- Existing code depends on this crate

**Estimated Time:** 4-5 days

## Recommendation: Option 2 (Bridge Pattern)

### Why Option 2?

1. **Pragmatic** - Gets self-driving working quickly
2. **Safe** - Doesn't break existing code
3. **Proven** - Reuses battle-tested implementations
4. **Maintainable** - Changes to LLM APIs happen once
5. **Reversible** - Can refactor to Option 3 later if needed

### Implementation Plan

#### Phase 1: Tool Execution Bridge (4 hours)

**Goal:** Make `ToolExecutor::execute()` call actual MCP tools

**Steps:**
1. Add dependency on `botticelli_mcp` to `botticelli_mcp_client`
2. Add `Arc<ToolRegistry>` to `ToolExecutor`
3. Translate `Value` arguments to MCP `ToolInput`
4. Call `ToolRegistry::execute_tool()`
5. Translate `ToolOutput` back to `Value`
6. Add unit tests

**Files Changed:**
- `crates/botticelli_mcp_client/Cargo.toml` (+1 dep)
- `crates/botticelli_mcp_client/src/tool_executor.rs` (~30 lines)

**Success Criteria:**
- ✅ `execute()` calls real MCP tools
- ✅ Unit tests with mock tools pass
- ✅ Integration test with real tool passes

#### Phase 2: LLM Provider Bridge (6 hours)

**Goal:** Make `LlmAdapter` implementations delegate to `LlmProvider`

**Steps:**
1. Add `Arc<dyn LlmProvider>` to each adapter struct
2. Translate `llm_adapter::Message` → `botticelli_core::Message`
3. Translate `ToolSchema` → tool definitions for provider
4. Call `provider.generate()`
5. Translate `GenerateResponse` → `GenerationResponse`
6. Extract tool calls from response
7. Add tests for each provider

**Files Changed:**
- `crates/botticelli_mcp_client/Cargo.toml` (+1 dep on botticelli_core)
- `crates/botticelli_mcp_client/src/llm_adapter.rs` (~200 lines)

**Success Criteria:**
- ✅ All 4 adapters call real LLM APIs
- ✅ Type translation works correctly
- ✅ Unit tests with mock provider pass
- ✅ Integration tests with real API pass (api feature)

#### Phase 3: Tool Call Parsing (3 hours)

**Goal:** Extract tool calls from LLM responses

**Steps:**
1. Parse `GenerateResponse` for tool use blocks
2. Extract tool name, id, arguments
3. Convert to `ToolCall` structs
4. Handle multiple tool calls in one response
5. Add comprehensive tests

**Files Changed:**
- `crates/botticelli_mcp_client/src/client.rs` (~50 lines)

**Success Criteria:**
- ✅ Extracts tool calls from Claude responses
- ✅ Extracts tool calls from Gemini responses
- ✅ Handles multiple calls correctly
- ✅ Handles no tool calls gracefully
- ✅ Unit tests pass

#### Phase 4: History Summarization (2 hours)

**Goal:** Implement LLM-based history summarization

**Steps:**
1. Create summarization prompt
2. Call LLM with old messages
3. Replace old messages with summary
4. Keep recent messages intact
5. Add configuration for when to summarize
6. Add tests

**Files Changed:**
- `crates/botticelli_mcp_client/src/context.rs` (~40 lines)

**Success Criteria:**
- ✅ Summarizes when history exceeds threshold
- ✅ Preserves recent context
- ✅ Generated summaries are coherent
- ✅ Unit tests pass

#### Phase 5: Integration Testing (3 hours)

**Goal:** Validate end-to-end self-driving flow

**Steps:**
1. Create test MCP server with multiple tools
2. Test: User prompt → LLM → tool call → execution → response
3. Test: Multi-turn conversation with tools
4. Test: Multiple tool calls in parallel
5. Test: Error handling (bad tool, bad args)
6. Document usage examples

**Files Changed:**
- `crates/botticelli_mcp_client/tests/self_driving_test.rs` (NEW)

**Success Criteria:**
- ✅ Full self-driving loop works
- ✅ Multi-turn conversations work
- ✅ Tool calling is reliable
- ✅ Error cases handled gracefully
- ✅ Documentation has examples

### Total Estimated Time: 18 hours (2-3 days)

## Type Translation Reference

### Message Translation

```rust
// llm_adapter::Message → botticelli_core::Message
fn translate_message(msg: llm_adapter::Message) -> botticelli_core::Message {
    let role = match msg.role {
        llm_adapter::MessageRole::User => Role::User,
        llm_adapter::MessageRole::Assistant => Role::Assistant,
        llm_adapter::MessageRole::System => Role::System,
        llm_adapter::MessageRole::Tool => Role::Tool,
    };
    
    let mut content = vec![Input::Text(msg.content)];
    
    // Add tool calls
    for call in msg.tool_calls {
        content.push(Input::ToolCall {
            id: call.id,
            name: call.name,
            arguments: call.arguments,
        });
    }
    
    // Add tool results
    for result in msg.tool_results {
        content.push(Input::ToolResult {
            tool_call_id: result.tool_call_id,
            content: result.content.to_string(),
            is_error: Some(result.is_error),
        });
    }
    
    Message::builder()
        .role(role)
        .content(content)
        .build()
        .expect("Valid message")
}
```

### Tool Schema Translation

```rust
// ToolSchema → provider tool definition
fn translate_tool_schema(schema: &ToolSchema) -> Value {
    json!({
        "name": schema.name,
        "description": schema.description,
        "input_schema": schema.parameters
    })
}
```

### Response Translation

```rust
// GenerateResponse → GenerationResponse
fn translate_response(resp: GenerateResponse) -> GenerationResponse {
    GenerationResponse {
        content: extract_text(&resp),
        tool_calls: extract_tool_calls(&resp),
        finish_reason: translate_stop_reason(resp.stop_reason()),
    }
}

fn extract_tool_calls(resp: &GenerateResponse) -> Vec<ToolCall> {
    resp.outputs()
        .iter()
        .filter_map(|output| match output {
            Output::ToolUse { id, name, input } => Some(ToolCall {
                id: id.clone(),
                name: name.clone(),
                arguments: input.clone(),
            }),
            _ => None,
        })
        .collect()
}
```

## Testing Strategy

### Unit Tests (in src/)
- Type translation functions
- Message building
- Tool call extraction
- Error handling

### Integration Tests (tests/)
- Real LLM API calls (#[ignore], api feature)
- Real tool execution
- End-to-end self-driving flow
- Multi-turn conversations

### Test Coverage Goals
- Type translation: 100%
- Tool execution: 90%
- LLM integration: 80% (mocked) + manual API tests
- Self-driving flow: 100%

## Risk Mitigation

### Risk 1: Type Mismatches
**Mitigation:** Comprehensive unit tests for all translations

### Risk 2: API Changes Breaking
**Mitigation:** Existing providers already handle this, we're just wrapping

### Risk 3: Tool Call Parsing Fragility
**Mitigation:** Test with multiple LLM outputs, handle edge cases

### Risk 4: Performance Overhead
**Mitigation:** Translation is cheap (just mapping), benchmark if concerned

## Success Criteria

### Phase 1 Complete
- ✅ Tool execution calls real MCP tools
- ✅ Tests pass
- ✅ No stub code in tool_executor.rs

### Phase 2 Complete
- ✅ All 4 LLM adapters work with real APIs
- ✅ Type translation validated
- ✅ No stub code in llm_adapter.rs

### Phase 3 Complete
- ✅ Tool calls extracted from responses
- ✅ Works with multiple providers
- ✅ No stub code in client.rs

### Phase 4 Complete
- ✅ History summarization functional
- ✅ Configurable thresholds
- ✅ No TODO in context.rs

### Phase 5 Complete
- ✅ Full self-driving demonstrated
- ✅ Documentation with examples
- ✅ Integration tests passing

### Final Acceptance
- ✅ Zero TODO/FIXME/STUB comments
- ✅ All tests passing
- ✅ Documentation complete
- ✅ Self-driving works end-to-end
- ✅ No code duplication with main workspace

## Next Steps

Ready to proceed with Phase 1: Tool Execution Bridge?

This is the foundation - once tools execute, we can add LLM integration, then close the loop with tool call parsing.

Time estimate: ~4 hours to complete Phase 1.
