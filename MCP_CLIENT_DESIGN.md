# MCP Client Design - Self-Driving Botticelli

## Overview

Design and implement an MCP client that enables Botticelli to become self-driving by:
1. Presenting available MCP tools to LLM backends
2. Allowing models to select and invoke tools
3. Processing tool results and feeding back to models
4. Managing multi-turn agentic workflows

## Current State

### What We Have
- ✅ MCP server with 15+ tools (narrative, execution, Discord, database)
- ✅ Multiple LLM backends (Anthropic, Gemini, Groq, OpenAI, Ollama, HuggingFace)
- ✅ Token counting and cost tracking infrastructure
- ✅ Observability with tracing and metrics
- ✅ Narrative execution framework

### What We Need
- ❌ MCP client to connect to our server
- ❌ Tool schema conversion (MCP → LLM-specific formats)
- ❌ Tool invocation loop (model → tool → model)
- ❌ Context management for multi-turn conversations
- ❌ Result formatting and error handling
- ❌ Agentic workflow orchestration

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Botticelli Self-Driving                  │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────┐         ┌──────────────┐                │
│  │  MCP Client   │◄────────┤   LLM Router │                │
│  │               │         │              │                │
│  │  - Discovery  │         │ - Anthropic  │                │
│  │  - Tool Call  │         │ - Gemini     │                │
│  │  - Results    │         │ - Groq       │                │
│  └───────┬───────┘         │ - OpenAI     │                │
│          │                 │ - Ollama     │                │
│          │                 │ - HuggingFace│                │
│          │                 └──────┬───────┘                │
│          │                        │                         │
│          │     ┌──────────────────┘                         │
│          │     │                                            │
│  ┌───────▼─────▼──────┐      ┌─────────────────┐          │
│  │  Agentic Executor   │◄─────┤  Context Manager│          │
│  │                     │      │                 │          │
│  │  - Tool Loop        │      │ - History       │          │
│  │  - Error Handling   │      │ - State         │          │
│  │  - Result Format    │      │ - Memory        │          │
│  └──────────┬──────────┘      └─────────────────┘          │
│             │                                                │
│             ▼                                                │
│  ┌─────────────────────┐                                   │
│  │    MCP Server       │                                   │
│  │                     │                                   │
│  │  - 15+ Tools        │                                   │
│  │  - Narratives       │                                   │
│  │  - Discord          │                                   │
│  │  - Database         │                                   │
│  └─────────────────────┘                                   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Unified Tool Interface**: Convert MCP tool schemas to each LLM's native format
2. **Provider Abstraction**: Each LLM backend implements `ToolCallingProvider` trait
3. **Async Tool Execution**: Non-blocking tool calls with progress tracking
4. **Context Preservation**: Maintain conversation history across tool calls
5. **Error Recovery**: Graceful handling of tool failures with retry logic
6. **Observability**: Full tracing of tool selection, execution, and results

## Implementation Plan

### Phase 1: Core MCP Client

**Goal**: Basic MCP client that can discover and invoke tools

**Tasks**:
1. Create `botticelli_mcp_client` crate
2. Implement MCP protocol client:
   - Tool discovery (`tools/list`)
   - Tool invocation (`tools/call`)
   - Resource access
   - Prompt templates
3. Connection management (stdio, SSE, WebSocket)
4. Error handling and retries
5. Basic integration tests

**Files**:
- `crates/botticelli_mcp_client/src/client.rs` - Core client
- `crates/botticelli_mcp_client/src/connection.rs` - Transport layer
- `crates/botticelli_mcp_client/src/error.rs` - Error types
- `crates/botticelli_mcp_client/tests/client_test.rs`

### Phase 2: Tool Schema Conversion ✅ COMPLETE

**Goal**: Convert MCP tool schemas to LLM-specific formats

**Status**: Complete - All provider schema converters implemented (2025-12-05)

**Completed Tasks**:
1. ✅ Defined `ToolSchema` abstraction
2. ✅ Implemented converters for all LLMs:
   - ✅ Anthropic (native tools support)
   - ✅ Gemini (function calling)
   - ✅ OpenAI (function calling)
   - ✅ Groq (function calling - OpenAI-compatible)
   - ✅ Ollama (prompt engineering with JSON response format)
   - ✅ HuggingFace (model-dependent with prompt templates)
3. ✅ JSON Schema → Provider format conversion via `ToolSchemaConverter` trait
4. ✅ All converters compile cleanly

**Files Created**:
- `crates/botticelli_mcp_client/src/schema/mod.rs` - Core abstractions
- `crates/botticelli_mcp_client/src/schema/anthropic.rs` - Anthropic converter
- `crates/botticelli_mcp_client/src/schema/gemini.rs` - Gemini converter
- `crates/botticelli_mcp_client/src/schema/openai.rs` - OpenAI converter
- `crates/botticelli_mcp_client/src/schema/groq.rs` - Groq converter
- `crates/botticelli_mcp_client/src/schema/ollama.rs` - Ollama converter
- `crates/botticelli_mcp_client/src/schema/huggingface.rs` - HuggingFace converter

**Next**: Phase 3 - LLM Provider Integration

### Phase 3: LLM Provider Integration

**Goal**: Integrate MCP tools with existing LLM backends

**Tasks**:
1. Define `ToolCallingProvider` trait:
   ```rust
   pub trait ToolCallingProvider {
       async fn generate_with_tools(
           &self,
           messages: Vec<Message>,
           tools: Vec<ToolSchema>,
           config: ToolConfig,
       ) -> Result<ToolCallingResponse>;
   }
   ```
2. Implement for each backend:
   - Extract tool calls from responses
   - Format tool results for next turn
   - Handle tool call errors
3. Update existing clients to support tools
4. Integration tests with real MCP server

**Files**:
- `crates/botticelli_mcp_client/src/provider.rs` - Trait definition
- `crates/botticelli_anthropic/src/tools.rs` - Anthropic implementation
- `crates/botticelli_gemini/src/tools.rs` - Gemini implementation
- `crates/botticelli_openai/src/tools.rs` - OpenAI implementation
- `crates/botticelli_groq/src/tools.rs` - Groq implementation
- `crates/botticelli_ollama/src/tools.rs` - Ollama implementation
- `crates/botticelli_huggingface/src/tools.rs` - HuggingFace implementation

### Phase 4: Agentic Executor ⚠️ IN PROGRESS

**Goal**: Orchestrate multi-turn tool-calling workflows

**Status**: Started 2025-12-05 - Needs refactoring to match existing `McpClient.execute()` API

**Completed Tasks**:
1. ✅ Created `AgenticExecutor` with execution strategies
2. ✅ Implemented multi-turn conversation loop
3. ✅ Added timeout and max iteration handling
4. ✅ Execution strategies defined (Autonomous, SingleShot, Guided)

**Remaining Work**:
- ⚠️ Refactor executor to work with existing `LlmBackend` trait in `client.rs`
- ⚠️ Reconcile with existing `McpClient.execute()` method
- ⚠️ Consider whether to enhance existing implementation or create separate executor
- ✅ Error handling and recovery (completed 2025-12-05)
  - Retry logic with exponential backoff
  - Circuit breaker for cascading failure prevention
  - Error classification (retryable, rate-limited)
- 🔲 Add comprehensive tests
- 🔲 Implement guided mode approval mechanism

**Files**:
- `crates/botticelli_mcp_client/src/executor.rs` - Initial implementation (needs refactor)
- `crates/botticelli_mcp_client/src/client.rs` - Existing execute method
- `crates/botticelli_mcp_client/tests/executor_test.rs` - TODO

### Phase 5: Context Management

**Goal**: Maintain conversation history and state

**Tasks**:
1. Context storage:
   - In-memory for short sessions
   - Database for persistence
   - Summarization for long contexts
2. Token budget management:
   - Track context window usage
   - Automatic summarization when near limit
   - Prioritize recent/important messages
3. State serialization for resumption
4. Context pruning strategies

**Files**:
- `crates/botticelli_mcp_client/src/context.rs`
- `crates/botticelli_mcp_client/src/storage.rs`
- `crates/botticelli_mcp_client/src/summarizer.rs`

### Phase 6: Testing Infrastructure ✅ COMPLETE

**Goal**: Comprehensive test coverage for MCP client

**Completed Tasks**:
1. ✅ Basic client tests (creation, discovery, invocation)
2. ✅ LLM integration tests (Anthropic, Gemini, OpenAI)
3. ✅ Conversation loop testing
4. ✅ Error handling tests
5. ✅ Reconnection testing

**Files**:
- `crates/botticelli_mcp/tests/client_basic_test.rs`
- `crates/botticelli_mcp/tests/client_llm_integration_test.rs`

### Phase 7: Observability Integration ✅ COMPLETE

**Goal**: Full visibility into agentic workflows

**Completed**:
1. ✅ Metrics infrastructure (`McpClientMetrics`)
2. ✅ Tool call tracking (success/failure by tool name)
3. ✅ Tool execution duration histograms
4. ✅ Token usage per turn (input/output)
5. ✅ Workflow cost tracking by model
6. ✅ Agent iteration metrics (completed/max_iterations/error)
7. ✅ Prometheus registry integration
8. ✅ Error type for metrics errors

**Files**:
- Update `crates/botticelli_mcp_client/src/executor.rs` with instrumentation
- `grafana/dashboards/mcp_agent.json`

### Phase 8: CLI Integration

**Goal**: Expose self-driving capabilities via CLI

**Tasks**:
1. Add commands to `botticelli` binary:
   - `botticelli agent run <goal>` - Autonomous agent
   - `botticelli agent interactive` - Guided mode
   - `botticelli agent tools` - List available tools
2. Configuration:
   - Model selection
   - Max iterations
   - Approval mode
   - Tool filtering
3. Rich terminal UI:
   - Display tool calls in real-time
   - Show reasoning (if available)
   - Progress indicators
   - Interactive approval

**Files**:
- `crates/botticelli/src/commands/agent.rs`
- Update `crates/botticelli/src/main.rs`

### Phase 8: Discord Bot Integration

**Goal**: Self-driving Discord bot using MCP tools

**Tasks**:
1. Respond to queries using agentic workflow:
   - User asks question → Agent uses tools → Responds
2. Tool access control:
   - Role-based permissions
   - Rate limiting
   - Approval workflows for sensitive operations
3. Thread-based conversations:
   - Each thread = separate context
   - Automatic summarization
4. Examples:
   - "Generate a narrative about X and post it" → Uses narrative + Discord tools
   - "What's trending in #general?" → Uses Discord query tools + analysis
   - "Create and run content generation workflow" → Autonomous execution

**Files**:
- Update `crates/botticelli_discord/src/handler.rs`
- `crates/botticelli_discord/src/agent.rs`

## Tool Support by Provider

| Provider     | Native Tools | Schema Format      | Notes                          |
|--------------|--------------|--------------------|---------------------------------|
| Anthropic    | ✅           | Tools API          | Best support, streaming         |
| Gemini       | ✅           | Function Calling   | Good support                    |
| OpenAI       | ✅           | Function Calling   | Industry standard               |
| Groq         | ✅           | Function Calling   | OpenAI-compatible               |
| Ollama       | ⚠️           | Prompt Engineering | Model-dependent, no native API  |
| HuggingFace  | ⚠️           | Model-dependent    | Varies by model                 |

## Security Considerations

1. **Tool Access Control**:
   - Whitelist/blacklist tools per agent
   - Role-based permissions
   - Audit logging of all tool calls

2. **Input Validation**:
   - Validate tool arguments before execution
   - Sanitize outputs before returning to model
   - Prevent injection attacks

3. **Rate Limiting**:
   - Per-user limits on tool calls
   - Cost budgets for expensive operations
   - Circuit breakers for failing tools

4. **Secrets Management**:
   - Never expose API keys to models
   - Secure credential storage
   - Tool-level authentication

## Testing Strategy

1. **Unit Tests**:
   - Schema conversion correctness
   - Tool invocation logic
   - Error handling

2. **Integration Tests**:
   - Full agent workflows with mock MCP server
   - Each LLM provider with real API
   - Multi-turn conversations

3. **End-to-End Tests**:
   - CLI agent commands
   - Discord bot scenarios
   - Real MCP server + tools

4. **Performance Tests**:
   - Latency per tool call
   - Context window management
   - Concurrent agent executions

## Success Metrics

- ✅ Agent can discover and use all 15+ MCP tools
- ✅ All LLM backends support tool calling (or graceful fallback)
- ✅ <100ms overhead per tool call
- ✅ 99% tool invocation success rate
- ✅ Full observability (traces, metrics, logs)
- ✅ Discord bot can autonomously complete complex tasks
- ✅ Cost tracking accurate within 5%

## Dependencies

### New Crates
- `botticelli_mcp_client` - Core MCP client library

### Updates to Existing Crates
- `botticelli_anthropic` - Add tool calling support
- `botticelli_gemini` - Add tool calling support
- `botticelli_openai` - Add tool calling support
- `botticelli_groq` - Add tool calling support
- `botticelli_ollama` - Add prompt-based tool support
- `botticelli_huggingface` - Add model-specific tool support
- `botticelli_discord` - Add agent integration
- `botticelli` - Add agent CLI commands

### External Dependencies
- `mcp-rust-sdk` (if exists) or implement protocol directly
- `serde_json` for schema conversion
- `tokio` for async tool execution

## Future Enhancements

1. **Multi-Agent Collaboration**:
   - Multiple agents working together
   - Agent-to-agent communication
   - Shared context/memory

2. **Learning and Optimization**:
   - Track which tools work best for which tasks
   - Optimize tool selection over time
   - Caching of common tool results

3. **Advanced Context Management**:
   - Vector embeddings for semantic search
   - Long-term memory storage
   - Cross-conversation learning

4. **Tool Composition**:
   - Chain multiple tools automatically
   - Create tool pipelines
   - Reusable tool workflows

## Timeline Estimate

- Phase 1 (MCP Client): 2-3 days
- Phase 2 (Schema Conversion): 2-3 days
- Phase 3 (Provider Integration): 3-4 days
- Phase 4 (Agentic Executor): 2-3 days
- Phase 5 (Context Management): 2-3 days
- Phase 6 (Observability): 1-2 days
- Phase 7 (CLI): 1-2 days
- Phase 8 (Discord Bot): 2-3 days

**Total**: 15-23 days of focused development

## Next Steps

1. Review and approve this design
2. Create `botticelli_mcp_client` crate structure
3. Begin Phase 1 implementation
4. Set up integration test infrastructure
5. Document API as we build

---

**Status**: Planning
**Last Updated**: 2025-12-05
**Owner**: Erik + Claude

---

## Implementation Progress

### Phase 1: Core Infrastructure ✅ Complete

**Date**: 2025-12-05  
**Status**: Compiles and lints cleanly

**Completed**:
1. ✅ Created `botticelli_mcp_client` crate
2. ✅ Defined `LlmBackend` trait for backend abstraction
3. ✅ Implemented `McpClient` with agentic loop skeleton
4. ✅ Added `ToolDefinition` and `ToolExecutor` for tool management
5. ✅ Basic error handling with `McpClientError` and location tracking

**Files Created**:
- `crates/botticelli_mcp_client/src/lib.rs` - Module exports
- `crates/botticelli_mcp_client/src/client.rs` - Core client with agentic loop
- `crates/botticelli_mcp_client/src/error.rs` - Error types with derive_more
- `crates/botticelli_mcp_client/src/tool_executor.rs` - Tool execution infrastructure

**Key Design Decisions**:
- Used `ToolDefinition` struct instead of mcp_server types for flexibility
- Agentic loop with configurable max iterations
- Tool executor uses HashMap for O(1) tool lookup
- LlmBackend trait requires Debug for instrumentation

**Next**: Implement LLM backend adapters (Phase 2)
