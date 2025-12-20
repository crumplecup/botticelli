# Chat MCP Implementation Plan

## Overview

Implementation plan for integrating MCP (Model Context Protocol) tools with the chat interface, using the Gemini provider with fallback support.

## Architecture

```
Chat Interface (MCP Host)
    ↓
ServiceContainer
    ├── LLM Provider (Gemini with fallback via ChatSession)
    ├── MCP Client (UnifiedMcpClient)
    └── Tool Registry (MCP tools)
    ↓
SamplingIntegration
    ├── ChatSession (tracks current model, handles fallback)
    ├── ServiceContainer (creates clients on demand)
    └── SamplingCoordinator
        ├── ChatLlmSampler (wraps provider with retry logic)
        └── Tool Registry
```

## Phase 1: LLM Provider Setup ✅

### Task 1.1: Create LLM Provider Initialization ✅

**Implementation:**
- [x] Add `llm_provider` field to ServiceContainer (OnceCell)
- [x] Implement `llm_provider()` method that lazily creates GeminiClient
- [x] Implement `create_client_for_model()` helper for dynamic client creation
- [x] Implement `create_tool_calling_client()` for MCP integration

### Task 1.2: Integrate Fallback Architecture ✅

**Current Architecture:**
- `ChatSession` tracks current model and handles rate limit fallback
- `ModelSelector` implements "loyal" (within-family) and "friendly" (cross-family) strategies
- Selection strategies configured via `SelectionStrategy` enum

**Implementation:**
- [x] ServiceContainer creates initial client for configured model
- [x] Fallback handled at execution layer via ChatSession
- [ ] Add retry logic wrapper around LLM calls with fallback

## Phase 2: MCP Client Integration ✅

### Task 2.1: Wire MCP Client to ServiceContainer ✅
- [x] MCP client initializes lazily in ServiceContainer
- [x] UnifiedMcpClient configured with max_iterations

### Task 2.2: Connect Tool Registry ✅
- [x] SamplingIntegration creates ToolRegistry with MCP tools
- [x] ChatLlmSampler receives tool registry reference
- [x] Tools passed via generate_with_tools()

## Phase 3: Tool Execution Flow ✅

### Task 3.1: Verify ToolCalling Implementation ✅
- [x] GeminiClient implements ToolCalling trait
- [x] Tool definitions serialize to Gemini API format
- [x] Tool calls extracted from responses

### Task 3.2: Update to Rust Edition 2024 ✅
- [x] Workspace inherits edition 2024
- [x] All member crates inherit from workspace

### Task 3.3: Implement Tool Result Handling ✅
- [x] ChatLlmSampler.execute_tools() uses ToolRegistry
- [x] Tool results convert to ToolResult format
- [x] SamplingCoordinator orchestrates tool execution loop

### Task 3.4: Add Environment Loading ✅
- [x] dotenvy loads .env file in main()
- [x] API keys load from environment

## Phase 4: Fallback Integration 🔄 IN PROGRESS

### Task 4.1: Add ChatSession to SamplingIntegration

**What:** Track current model state and enable fallback

**Implementation:**
- [ ] Add `chat_session: Arc<RwLock<ChatSession>>` field to SamplingIntegration
- [ ] Initialize ChatSession from ServiceContainer config:
  - `initial_model` from config.chat.initial_model()
  - `fallback_strategy` from config.chat.fallback_strategy()
  - `bounds` from config.chat.model_bounds()
- [ ] Pass ChatSession to ChatLlmSampler

**Success Criteria:**
- SamplingIntegration tracks current model via ChatSession
- Configuration properly loaded from config

### Task 4.2: Implement Retry Logic in ChatLlmSampler

**What:** Wrap LLM calls with automatic fallback on rate limits

**Implementation:**
- [ ] Add `chat_session` and `services` fields to ChatLlmSampler
- [ ] Wrap `generate()` call in retry loop:
  ```rust
  loop {
      match provider.generate_with_tools(&request, tools).await {
          Ok(response) => return Ok(response),
          Err(e) if is_rate_limit(&e) => {
              // Use ChatSession to select next model
              let next_model = session.handle_rate_limit(&e.to_string())?;
              // Create new client via ServiceContainer
              provider = services.create_tool_calling_client(next_model)?;
              // Retry with new provider
          }
          Err(e) => return Err(e),
      }
  }
  ```
- [ ] Make provider mutable: `Arc<RwLock<dyn ToolCalling>>`
- [ ] Add max retry limit (e.g., 3 attempts)

**Success Criteria:**
- Rate limit errors trigger fallback automatically
- Fallback follows configured strategy
- Retries stop after max attempts
- All transitions traced

### Task 4.3: Test Fallback Flow

**What:** Verify fallback works end-to-end

**Implementation:**
- [ ] Manual test: trigger rate limit, observe fallback
- [ ] Verify model transitions in traces
- [ ] Verify fallback respects bounds configuration
- [ ] Test both "loyal_first" and "friendly_first" strategies

**Success Criteria:**
- Rate limits handled gracefully
- Correct model selected based on strategy
- User sees informative error if all fallbacks exhausted

## Phase 5: Testing & Documentation

### Task 5.1: Add Integration Tests
- [ ] Test narrative generation with tools
- [ ] Test fallback scenarios
- [ ] Gate tests with `#[cfg_attr(not(feature = "api"), ignore)]`

### Task 5.2: Add Observability
- [ ] Verify #[instrument] coverage
- [ ] Log model transitions during fallback
- [ ] Log tool executions

### Task 5.3: Update Documentation
- [ ] Update CHAT_MCP_ARCHITECTURE_STATUS
- [ ] Document fallback configuration
- [ ] Add tool calling examples

## Implementation Notes

### Fallback Architecture

The existing fallback system:
1. **ModelSelector**: Selects next model based on strategy and bounds
2. **ChatSession**: Wraps ModelSelector, tracks current model
3. **RateLimitDetector**: Identifies rate limit errors from error messages

Integration pattern:
1. **SamplingIntegration** owns ChatSession
2. **ChatLlmSampler** borrows ChatSession and ServiceContainer
3. On rate limit error:
   - Call `session.handle_rate_limit(error_msg)`
   - Get next model ID
   - Create new client via `services.create_tool_calling_client(model_id)`
   - Retry request with new client

### Configuration

```toml
[chat]
initial_model = { Gemini = "Gemini25Flash" }
fallback_strategy = "friendly_first"

[chat.model_bounds]
upper = { Gemini = "Gemini25Flash" }
lower = { Gemini = "Gemini25FlashLite" }
```

## Success Criteria

- ✅ Gemini provider initializes with API key
- ✅ MCP tools available via ToolRegistry
- ✅ Tool execution returns results to LLM
- ✅ Multi-turn tool calling works
- [ ] Rate limit errors trigger automatic fallback
- [ ] Fallback follows configured strategy
- [ ] All operations traced
- [ ] Tests pass with real API
- [ ] Documentation complete
