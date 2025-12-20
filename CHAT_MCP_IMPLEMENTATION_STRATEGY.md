# Chat MCP Host Implementation Strategy

**Created:** 2025-12-20  
**Status:** Planning  
**Goal:** Implement botticelli_chat as proper MCP host using unified trait interface

---

## Vision

The chat binary is an MCP host that:
1. Loads MCP server tools via `botticelli_mcp` client
2. Uses unified trait interface (`ToolCalling`, `TextGeneration`, etc.)
3. Calls LLM backends through trait abstraction with fallback
4. Default: Gemini with fallback to Anthropic/others
5. Handles tool execution via MCP protocol

## Current Reality

**What Works:**
- Basic chat REPL with readline
- Configuration loading from TOML
- Environment variable handling with dotenvy
- Some MCP client initialization code exists

**What's Broken/Missing:**
- No MCP host implementation (server connection/tool loading)
- No unified trait interface usage
- No fallback provider integration
- Hard-coded Anthropic client (old pattern)
- No tool calling flow
- No proper conversation state management
- Missing turn-based conversation loop with tool use

---

## Architecture Gaps

### Gap 1: MCP Host Not Implemented
**Current:** Code mentions MCP but doesn't actually load tools from servers  
**Need:** Proper `McpClient::connect()` and tool registry loading

### Gap 2: No Trait Interface Usage
**Current:** Direct `AnthropicClient` instantiation (old pattern)  
**Need:** Use `FallbackProvider` with Gemini primary + fallback strategy

### Gap 3: No Tool Execution Flow
**Current:** Chat loop just sends/receives text  
**Need:** Detect tool calls → execute via MCP → continue conversation

### Gap 4: Conversation State Not Managed
**Current:** No message history tracking  
**Need:** Maintain conversation context across turns

---

## Implementation Plan

### Phase 1: MCP Host Foundation
**Goal:** Connect to MCP servers and load tools

#### Task 1.1: Initialize MCP Client
- [ ] Add `McpClient` field to `ChatState`
- [ ] Connect to configured MCP servers in `chat.toml`
- [ ] Load available tools into registry
- [ ] Verify tools loaded with debug logging

**Success Criteria:**
- `just run-chat` connects to MCP servers
- Tools logged at startup
- Clean error if servers unavailable

#### Task 1.2: Add MCP Configuration to chat.toml
- [ ] Add `[mcp.servers]` section
- [ ] Configure botticelli MCP server
- [ ] Add stdio transport config
- [ ] Document configuration format

**Success Criteria:**
- `chat.toml` has MCP server config
- Matches working MCP client patterns

---

### Phase 2: Unified Trait Interface Integration
**Goal:** Replace hard-coded client with trait-based fallback provider

#### Task 2.1: Create Fallback Provider Configuration
- [ ] Add `[llm]` section to `chat.toml`
- [ ] Configure Gemini as primary provider
- [ ] Configure Anthropic as fallback
- [ ] Set fallback strategy (friendly/loyal)
- [ ] Add model configurations

**Success Criteria:**
- Configuration follows existing fallback patterns
- Strategy selection documented

#### Task 2.2: Initialize FallbackProvider in ChatState
- [ ] Replace `AnthropicClient` with `FallbackProvider`
- [ ] Load from configuration
- [ ] Initialize with API keys from environment
- [ ] Add proper error handling

**Success Criteria:**
- `ChatState` uses `FallbackProvider`
- Gemini tried first, fallback on error
- Clean compilation

#### Task 2.3: Update Message Loop to Use Traits
- [ ] Call `provider.generate()` instead of direct client
- [ ] Handle `TextGeneration` trait properly
- [ ] Process responses through trait interface
- [ ] Add fallback tracing

**Success Criteria:**
- Chat loop uses trait methods
- Fallback logged when triggered
- Same user experience, better architecture

---

### Phase 3: Tool Calling Flow
**Goal:** Implement full tool use conversation loop

#### Task 3.1: Detect Tool Calls in Response
- [ ] Check if response implements `ToolCalling` trait
- [ ] Extract tool calls from response
- [ ] Log tool call detection

**Success Criteria:**
- Tool calls detected from LLM response
- Proper type checking via trait

#### Task 3.2: Execute Tools via MCP
- [ ] For each tool call, look up in registry
- [ ] Call MCP server via `McpClient`
- [ ] Collect tool results
- [ ] Handle execution errors gracefully

**Success Criteria:**
- Tools execute successfully
- Results captured
- Errors logged, don't crash

#### Task 3.3: Continue Conversation with Tool Results
- [ ] Add tool results to conversation
- [ ] Call LLM again with results
- [ ] Loop until no more tool calls
- [ ] Return final text response to user

**Success Criteria:**
- Multi-turn tool use works
- Conversation flows naturally
- Final response displayed to user

---

### Phase 4: Conversation State Management
**Goal:** Track conversation history properly

#### Task 4.1: Add Message History to ChatState
- [ ] Store conversation messages
- [ ] Include user messages, assistant responses, tool calls, tool results
- [ ] Implement reasonable history limit (last N turns)

**Success Criteria:**
- History maintained across turns
- Context available to LLM

#### Task 4.2: Serialize/Deserialize for Persistence (Future)
- [ ] Design: Save conversations to file
- [ ] Load previous conversations
- [ ] Resume sessions

**Success Criteria:**
- Conversations can be saved/loaded (deferred if complex)

---

### Phase 5: Testing & Polish
**Goal:** Ensure robustness and usability

#### Task 5.1: Error Handling Audit
- [ ] Network errors handled gracefully
- [ ] MCP server failures don't crash
- [ ] API errors show helpful messages
- [ ] Fallback provider errors logged

**Success Criteria:**
- No panics in normal error cases
- User gets actionable feedback

#### Task 5.2: Add Instrumentation
- [ ] `#[instrument]` on all public functions
- [ ] Structured logging for debugging
- [ ] Trace tool execution flow
- [ ] Performance spans for LLM calls

**Success Criteria:**
- `RUST_LOG=debug` shows clear flow
- Debugging is straightforward

#### Task 5.3: Documentation
- [ ] Update README with usage examples
- [ ] Document configuration options
- [ ] Add troubleshooting guide
- [ ] Example MCP server setup

**Success Criteria:**
- New user can run chat successfully
- Configuration clear and documented

---

## Success Metrics

**Minimum Viable:**
- Chat connects to MCP servers
- Tools are loaded and discoverable
- LLM (Gemini) generates responses via trait interface
- Tool calls execute via MCP
- Conversation continues with results
- Fallback works when Gemini fails

**Stretch Goals:**
- Conversation history persistence
- Multiple MCP servers
- Streaming responses
- Rich terminal UI improvements

---

## Dependencies

**Required Crates:**
- `botticelli_mcp` - MCP client
- `botticelli_interface` - Trait definitions
- `botticelli_core` - Common types
- `botticelli_llm_fallback` - Fallback provider
- `botticelli_gemini` - Primary LLM
- `botticelli_anthropic` - Fallback LLM

**Configuration:**
- API keys in `.env` or environment
- `chat.toml` with MCP servers and LLM config
- MCP server binary available

---

## Next Steps

1. **Start Phase 1, Task 1.1** - Initialize MCP client in chat binary
2. Verify compilation after each task
3. Test incrementally (don't wait until end)
4. Update this document as we learn

