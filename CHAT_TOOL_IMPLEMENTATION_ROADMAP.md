# Chat Tool Implementation Roadmap

## Status: In Progress
**Created:** 2025-12-21  
**Last Updated:** 2025-12-21

## Vision

The chat interface should have access to ALL designed tools through the MCP server. Currently only 4 tools are accessible, but we have 30+ tools implemented. The goal is to:

1. Ensure all tools are registered in the MCP server's ToolRegistry
2. Verify tools are exposed through the Router trait
3. Confirm the chat TUI loads the server with all tools
4. Validate the LLM can see and call all tools

## Current State Audit

### Tools Implemented (30+)

**Core Tools:**
- ✅ EchoTool
- ✅ ServerInfoTool

**Narrative Creation (Phase 1):**
- ✅ CreateNarrativeTool
- ✅ ModifyNarrativeTool  
- ✅ SaveNarrativeTool
- ✅ ValidateNarrativeTool

**Narrative Elicitation (LLM-Driven):**
- ✅ CreateNarrativeSessionTool
- ✅ ElicitMetadataTool
- ✅ ElicitActTool
- ✅ ElicitCarouselTool
- ✅ FinalizeNarrativeTool
- ✅ GetNarrativeStateTool
- ✅ ValidateNarrativeSessionTool
- ✅ ApplyValidationFixesTool

**Scene Management:**
- ✅ CreateSceneTool
- ✅ ListScenesTool
- ✅ UpdateSceneTool
- ✅ DeleteSceneTool

**Execution Tools (Phase 2 & 3):**
- ✅ GenerateTool
- ✅ ExecuteActTool
- ✅ ExecuteNarrativeTool

**LLM Backend Tools (Phase 4):**
- ✅ GenerateGeminiTool (feature: gemini)
- ✅ GenerateAnthropicTool (feature: anthropic)
- ✅ GenerateOllamaTool (feature: ollama)
- ✅ GenerateHuggingFaceTool (feature: huggingface)
- ✅ GenerateGroqTool (feature: groq)

**Metrics:**
- ✅ ExportMetricsTool

**Database Tools:**
- ✅ QueryContentTool (not in default registry - needs explicit config)

**Discord Tools (feature: discord):**
- ✅ DiscordPostMessageTool
- ✅ DiscordGetMessagesTool
- ✅ DiscordGetGuildInfoTool
- ✅ DiscordGetChannelsTool
- ✅ DiscordBotCommandTool
- ✅ DiscordPostTool
- ✅ DiscordContentWorkflowTool

### Tools Currently Accessible in Chat: 4

Based on user report: Only 4 tools are visible to the LLM in the chat interface.

### Gap Analysis

**Problem:** Massive gap between implemented tools (30+) and accessible tools (4).

**Potential Causes:**
1. MCP client not fetching tool list from server
2. Tool definitions not being converted to LLM format
3. LLM provider integration not receiving tools
4. Feature gates preventing tool registration
5. Chat initialization not properly wiring up the tool flow

## Implementation Roadmap

### Phase 1: Diagnostic & Discovery
**Goal:** Understand why only 4 tools are visible

#### Task 1.1: Trace Tool Registration Flow
- [ ] Add debug logging in `ToolRegistry::default()` to count registered tools
- [ ] Verify all tools are being registered (should be ~30+)
- [ ] Check feature gate compilation (some tools are feature-gated)

**Success Criteria:** Know exact count of tools registered in server

#### Task 1.2: Trace MCP Client Tool Discovery
- [ ] Add debug logging in `UnifiedMcpClient` when it fetches tools
- [ ] Check `list_tools()` implementation
- [ ] Verify tool list is being retrieved from server

**Success Criteria:** Confirm client can see all server tools

#### Task 1.3: Trace LLM Tool Passing
- [ ] Add debug logging when tools are converted to LLM format
- [ ] Check `ToolCalling::supports_tool_calling()` implementation
- [ ] Verify tool definitions reach the LLM API call

**Success Criteria:** Confirm tools are passed to LLM in API requests

#### Task 1.4: Check Chat UI Initialization
- [ ] Review `botticelli-chat.rs` startup sequence
- [ ] Verify MCP client is properly initialized
- [ ] Check if tool registry is being passed through

**Success Criteria:** Understand complete tool flow from server → client → LLM

### Phase 2: Fix Tool Flow
**Goal:** Wire up the complete tool chain

#### Task 2.1: Ensure Server Exposes All Tools
- [ ] Verify `BotticelliRouter::list_tools()` returns all registered tools
- [ ] Check Router trait implementation matches MCP spec
- [ ] Add test to verify tool count

**Success Criteria:** Server responds with complete tool list

#### Task 2.2: Fix Client Tool Discovery
- [ ] Implement `UnifiedMcpClient::list_tools()` if missing
- [ ] Cache tool list on client initialization
- [ ] Add method to refresh tool list

**Success Criteria:** Client has access to all server tools

#### Task 2.3: Wire Tools to LLM Provider
- [ ] Ensure `FallbackProvider` passes tools to backends
- [ ] Verify `ToolCalling::call_with_tools()` implementation
- [ ] Check Gemini client receives tool definitions

**Success Criteria:** LLM receives full tool list in API calls

#### Task 2.4: Update Chat Handler
- [ ] Modify `ChatApp` to fetch tools on startup
- [ ] Pass tool list to conversation handler
- [ ] Ensure tools are included in every LLM call

**Success Criteria:** Chat passes tools to LLM on every turn

### Phase 3: Testing & Validation
**Goal:** Confirm all tools are accessible and functional

#### Task 3.1: Add Tool Count Assertions
- [ ] Add test: Server registry has expected tool count
- [ ] Add test: Client sees all server tools
- [ ] Add test: LLM receives tool definitions

**Success Criteria:** Automated tests verify tool flow

#### Task 3.2: Manual Testing
- [ ] Run `just chat`
- [ ] Check startup logs for tool counts
- [ ] Ask LLM "what tools do you have access to?"
- [ ] Verify LLM lists all 30+ tools

**Success Criteria:** User can see all tools in chat

#### Task 3.3: Functional Testing
- [ ] Test narrative elicitation workflow
- [ ] Test scene management
- [ ] Test execution tools
- [ ] Test database queries (if configured)

**Success Criteria:** All tool categories work end-to-end

### Phase 4: Documentation & Polish
**Goal:** Make tool system maintainable

#### Task 4.1: Document Tool Architecture
- [ ] Create diagram: Tool flow (Registry → Router → Client → LLM)
- [ ] Document how to add new tools
- [ ] Document feature gates for optional tools

**Success Criteria:** Clear documentation for developers

#### Task 4.2: Add Observability
- [ ] Log tool registration count at startup
- [ ] Log tool list fetched by client
- [ ] Log tools passed to each LLM call
- [ ] Add metrics for tool usage

**Success Criteria:** Full visibility into tool system

#### Task 4.3: Error Handling
- [ ] Handle missing tools gracefully
- [ ] Add helpful error messages for misconfiguration
- [ ] Validate tool schemas at registration

**Success Criteria:** Clear errors guide troubleshooting

## Success Metrics

### Phase 1 Success
- [ ] Complete diagnostic trace of tool flow
- [ ] Identified root cause of 4-tool limit
- [ ] Have plan to fix

### Phase 2 Success
- [ ] Server exposes all tools (~30+)
- [ ] Client can fetch complete tool list
- [ ] LLM receives all tool definitions
- [ ] Chat integration complete

### Phase 3 Success
- [ ] Automated tests pass
- [ ] Manual testing confirms 30+ tools visible
- [ ] All tool categories functional

### Phase 4 Success
- [ ] Documentation complete
- [ ] Observability in place
- [ ] Error handling robust

## Current Status: ROOT CAUSE FOUND 🔍

**Phase 1 Results:** All tools ARE registered properly in `ToolRegistry::default()`. Found ~33 tools registered.

**Root Cause:** Chat TUI is NOT integrated with LLM at all!
- File: `crates/botticelli_chat/src/tui/tabs/chat.rs` line 116
- `send_message()` has TODO comment and placeholder response
- LLM is never called
- Tools are never fetched or passed

**Architecture is Sound:**
- ✅ ToolRegistry has all tools (33+)
- ✅ Router exposes tools via `list_tools()`  
- ✅ `ConversationLoop` exists with tool support
- ✅ `ToolCallHandler` can execute tools
- ❌ TUI never wires these together

**Next Action:** Implement the TODO in `send_message()` to:
1. Fetch tools from MCP client
2. Call `ConversationLoop::run_conversation()` 
3. Pass tools to LLM
4. Display actual LLM responses

## Notes

### Tool Categories
1. **Core** - Always available (echo, server_info)
2. **Narrative** - Main workflow tools
3. **Elicitation** - LLM-driven creation
4. **Scene** - Scene management
5. **Execution** - Runtime execution
6. **LLM** - Backend generation (feature-gated)
7. **Database** - Data queries (needs config)
8. **Discord** - Social integration (feature-gated)
9. **Metrics** - Observability

### Feature Gates to Check
- `gemini` - Gemini LLM tools
- `anthropic` - Anthropic LLM tools
- `ollama` - Ollama LLM tools
- `huggingface` - HuggingFace LLM tools
- `groq` - Groq LLM tools
- `discord` - Discord integration tools

### Key Files
- `crates/botticelli_mcp/src/tools/mod.rs` - Tool registry and registration
- `crates/botticelli_mcp/src/server.rs` - MCP router implementation
- `crates/botticelli_mcp_client/src/unified_client.rs` - Client tool discovery
- `crates/botticelli_chat/src/bin/botticelli-chat.rs` - Chat initialization
- `crates/botticelli_chat/src/tool_handler.rs` - Tool execution in chat
