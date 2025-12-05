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

### Phase 2: Tool Schema Conversion

**Goal**: Convert MCP tool schemas to LLM-specific formats

**Tasks**:
1. Define `ToolSchema` abstraction
2. Implement converters for each LLM:
   - Anthropic (native tools support)
   - Gemini (function calling)
   - OpenAI (function calling)
   - Groq (function calling)
   - Ollama (requires prompt engineering)
   - HuggingFace (model-dependent)
3. JSON Schema → Provider format conversion
4. Parameter validation
5. Tests for each converter

**Files**:
- `crates/botticelli_mcp_client/src/schema/mod.rs`
- `crates/botticelli_mcp_client/src/schema/anthropic.rs`
- `crates/botticelli_mcp_client/src/schema/gemini.rs`
- `crates/botticelli_mcp_client/src/schema/openai.rs`
- `crates/botticelli_mcp_client/src/schema/groq.rs`
- `crates/botticelli_mcp_client/src/schema/ollama.rs`
- `crates/botticelli_mcp_client/src/schema/huggingface.rs`

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

### Phase 4: Agentic Executor

**Goal**: Orchestrate multi-turn tool-calling workflows

**Tasks**:
1. Create `AgenticExecutor`:
   - Initialize with LLM provider + MCP client
   - Manage conversation state
   - Execute tool call loop
   - Handle max iterations / timeouts
2. Implement execution strategies:
   - **Autonomous**: Model decides when to stop
   - **Single-shot**: One tool call, then return
   - **Guided**: User approves each tool call
3. Result formatting and streaming
4. Error handling and recovery
5. Comprehensive tests

**Files**:
- `crates/botticelli_mcp_client/src/executor.rs`
- `crates/botticelli_mcp_client/src/strategy.rs`
- `crates/botticelli_mcp_client/tests/executor_test.rs`

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

### Phase 6: Observability Integration

**Goal**: Full visibility into agentic workflows

**Tasks**:
1. Tracing integration:
   - Span per agent execution
   - Tool selection decisions
   - Tool execution timing
   - Result processing
2. Metrics:
   - Tool call counts by type
   - Success/failure rates
   - Execution duration
   - Token usage per turn
   - Cost per workflow
3. Structured logging:
   - Model reasoning (if available)
   - Tool selection rationale
   - Error details
4. Grafana dashboard for agent monitoring

**Files**:
- Update `crates/botticelli_mcp_client/src/executor.rs` with instrumentation
- `grafana/dashboards/mcp_agent.json`

### Phase 7: CLI Integration

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
