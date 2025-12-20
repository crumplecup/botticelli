# Chat MCP Host Implementation Strategy

**Created:** 2025-12-20  
**Updated:** 2025-12-20
**Status:** In Progress  
**Goal:** Implement botticelli_chat as proper MCP host using unified trait interface

---

## Vision

The chat binary is an MCP host that:
1. Loads MCP server tools via `botticelli_mcp` client
2. Uses unified trait interface (`ToolCalling`, `TextGeneration`, etc.)
3. Calls LLM backends through trait abstraction with fallback
4. Default: Gemini (free tier) with fallback to Groq
5. Handles tool execution via MCP protocol

## Current Reality

**What Works:**
- ✅ Basic chat REPL with readline
- ✅ Configuration loading from TOML
- ✅ Environment variable handling with dotenvy
- ✅ **NEW:** Free-tier fallback configured (Gemini → Groq)
- ✅ **NEW:** Unified trait interface implemented
- ✅ **NEW:** `ModelSelector` with `SelectionStrategy::FriendlyFirst`

**What's Broken/Missing:**
- No MCP host implementation (server connection/tool loading)
- No tool calling flow integrated yet
- No proper conversation state management
- Missing turn-based conversation loop with tool use

---

## Implementation Plan

### Phase 1: Provider Integration with Fallback

**Status:** ✅ Complete

#### Task 1.2: Configure Default Provider Chain
- [x] Set Gemini as primary (free tier)
- [x] Configure Groq as fallback  
- [x] Use existing `ModelSelector` with `SelectionStrategy::FriendlyFirst`
- [x] Configure in `chat.toml`
- [x] Add `FromStr` implementation for `ModelId` to support TOML parsing

**Success Criteria:**
- ✅ Chat loads with Gemini 2.5 Flash as default
- ✅ Automatic fallback to Groq on rate limits (via existing fallback architecture)
- ✅ No manual provider switching needed
- ✅ Configuration properly loaded from `chat.toml`

**Implementation Notes:**
- Existing `ModelSelector`, `SelectionStrategy`, and `RateLimitDetector` provide complete fallback logic
- Added `std::str::FromStr` for `ModelId` to parse from TOML strings
- `ChatConfig` already has all needed fields (`initial_model`, `fallback_strategy`, `model_bounds`)
- `ChatAppConfig` already loads `chat` section from TOML
- Free-tier configuration: Gemini 2.5 Flash → Groq (friendly_first strategy)

---

### Phase 2: MCP Host Integration
**Goal:** Connect to MCP servers and enable tool calling

#### Task 2.1: Initialize MCP Client
- [ ] Connect to configured MCP servers from `chat.toml`
- [ ] Load available tools into registry
- [ ] Verify tools loaded with debug logging

**Success Criteria:**
- `just run-chat` connects to MCP servers
- Tools logged at startup
- Clean error if servers unavailable

#### Task 2.2: Integrate Tool Execution Flow
- [ ] Detect tool calls in LLM responses (via `ToolCalling` trait)
- [ ] Execute tools via MCP client
- [ ] Return results to LLM for next turn
- [ ] Loop until conversation complete

**Success Criteria:**
- Tool calls execute successfully
- Multi-turn conversations work
- Results integrated properly

---

### Phase 3: Conversation State Management
**Goal:** Track conversation history properly

#### Task 3.1: Add Message History to ChatState
- [ ] Store conversation messages
- [ ] Include user messages, assistant responses, tool calls, tool results
- [ ] Implement reasonable history limit (last N turns)

**Success Criteria:**
- History maintained across turns
- Context available to LLM

---

### Phase 4: Testing & Polish
**Goal:** Ensure robustness and usability

#### Task 4.1: Error Handling Audit
- [ ] Network errors handled gracefully
- [ ] MCP server failures don't crash
- [ ] API errors show helpful messages
- [ ] Fallback provider errors logged

**Success Criteria:**
- No panics in normal error cases
- User gets actionable feedback

#### Task 4.2: Add Instrumentation
- [ ] `#[instrument]` on all public functions
- [ ] Structured logging for debugging
- [ ] Trace tool execution flow
- [ ] Performance spans for LLM calls

**Success Criteria:**
- `RUST_LOG=debug` shows clear flow
- Debugging is straightforward

---

## Success Metrics

**Minimum Viable:**
- ✅ Free-tier fallback configured (Gemini → Groq)
- ✅ Trait interface integration complete
- [ ] Chat connects to MCP servers
- [ ] Tools execute via MCP
- [ ] Conversation continues with results
- [ ] Fallback works when rate limited

**Stretch Goals:**
- [ ] Conversation history persistence
- [ ] Multiple MCP servers
- [ ] Streaming responses
- [ ] Rich terminal UI improvements

---

## Dependencies

**Required Crates:**
- `botticelli_mcp` - MCP client
- `botticelli_interface` - Trait definitions  
- `botticelli_core` - Common types
- `botticelli_models` - Model selection & fallback
- `botticelli_gemini` - Primary LLM (free tier)
- `botticelli_groq` - Fallback LLM (free tier)

**Configuration:**
- API keys in `.env` or environment
- `chat.toml` with chat config and MCP servers
- MCP server binary available

---

## Next Steps

1. **Phase 2, Task 2.1** - Initialize MCP client in chat binary
2. Verify compilation after each task
3. Test incrementally
4. Update this document as we learn
