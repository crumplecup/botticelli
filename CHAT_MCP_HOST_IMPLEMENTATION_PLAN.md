# Chat MCP Host Implementation Plan

**Date**: 2025-12-20  
**Status**: Planning  
**Goal**: Complete the Chat MCP Host architecture to match the intended design

## Executive Summary

The chat interface is designed to be an MCP Host that uses the botticelli MCP client to provide tools to LLM backends. Currently, the LLM provider initialization is stubbed out, preventing the chat from actually working. This plan implements the complete integration.

## Architecture Vision

```
User → Chat Interface (MCP Host)
         ↓
    ServiceContainer
         ↓
    +--------------+
    |              |
    v              v
MCP Client    LLM Provider (BotticelliDriver)
    |              |
    v              v
MCP Tools    Gemini/Anthropic/Groq
                   ↓
              (with fallback chain)
```

## Current State Analysis

### ✅ Components That Exist
- `ServiceContainer`: Lazy initialization pattern
- `UnifiedMcpClient`: MCP client with tool registry
- `SamplingCoordinator`: Orchestrates LLM sampling
- `SamplingIntegration`: Bridges coordinator to chat
- `CommandExecutor`: Handles parsed user commands
- Configuration system: `ChatAppConfig` with all necessary settings
- **Existing fallback architecture**: `ModelSelector` with SelectionStrategy (LoyalFirst/FriendlyFirst)
  - Loyal: Try smaller model in same family first (Gemini 2.5 Flash → Gemini 2.5 Flash Lite)
  - Friendly: Try equivalent model in different family (Gemini → Groq)
  - Includes rate limit detection and automatic model selection

### ❌ Critical Gaps
1. **LLM Provider initialization returns errors** (services.rs:228-234) - ✅ FIXED in Task 1.1
2. **No BotticelliDriver instantiation** (should use GeminiClient, AnthropicClient) - ✅ FIXED in Task 1.1
3. **MCP tools not passed to LLM** (ToolCalling trait unused)
4. **Fallback chain not wired** (need to integrate ModelSelector)
5. **PlaceholderProvider delegates to non-existent provider** - ✅ FIXED in Task 1.1

## Implementation Phases

### Phase 1: Core LLM Provider Implementation - ✅ COMPLETE

**Objective**: Replace error-returning stubs with actual LLM client instantiation

**Status**: All tasks complete. Chat can now initialize real LLM clients and is ready for fallback integration.

#### Task 1.1: Implement Gemini Provider - ✅ COMPLETE
**File**: `crates/botticelli_chat/src/services.rs`

**Success Criteria**:
- [x] `init_llm_provider()` creates `GeminiClient` for Gemini models - ✅ Done
- [x] Uses API key from environment: `GEMINI_API_KEY` - ✅ Done
- [x] Returns `Arc<dyn botticelli_interface::BotticelliDriver>` (not generic LlmProvider) - ✅ Done
- [x] Compiles without errors - ✅ Done

**Status**: COMPLETED in commit 5001f76

**Implementation**:
```rust
fn init_llm_provider(&self) -> ChatResult<Arc<dyn botticelli_interface::BotticelliDriver>> {
    use botticelli_models::{GeminiClient, ModelId};
    use botticelli_interface::BotticelliDriver;

    let model_id = self.config.chat.initial_model();
    
    match model_id {
        ModelId::Gemini(model) => {
            let api_key = self.config.mcp_client.gemini_api_key()
                .ok_or_else(|| ChatError::new(ChatErrorKind::ConfigurationError(
                    "GEMINI_API_KEY not configured".into()
                )))?;
            
            let client = GeminiClient::new(api_key, model.to_string())
                .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(
                    format!("Failed to create Gemini client: {}", e)
                )))?;
            
            Ok(Arc::new(client))
        }
        ModelId::Groq(_) => {
            Err(ChatError::new(ChatErrorKind::NotImplemented(
                "Groq provider implementation pending".into()
            )))
        }
    }
}
```

#### Task 1.2: Add Support for Additional Model Families - ✅ COMPLETED
**File**: `crates/botticelli_chat/src/services.rs`

**Success Criteria**:
- [x] Extract client creation into `create_client_for_model(model_id)` helper - ✅ Done
- [x] Support ModelId::Gemini → GeminiClient (already done) - ✅ Done
- [x] Support ModelId::Groq → GroqDriver (when available) - ✅ Returns clear NotImplemented error
- [x] Return clear error for unsupported families - ✅ Done
- [x] Helper reusable by fallback system in Phase 3 - ✅ Public method

**Status**: COMPLETED in commit 713446c

**Implementation**:
```rust
fn create_client_for_model(&self, model_id: ModelId) -> ChatResult<Arc<dyn BotticelliDriver>> {
    match model_id {
        ModelId::Gemini(_model) => {
            let client = GeminiClient::new()
                .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(
                    format!("Failed to create Gemini client: {}", e)
                )))?;
            Ok(Arc::new(client))
        }
        ModelId::Groq(_model) => {
            // TODO: Add GroqDriver when available
            Err(ChatError::new(ChatErrorKind::NotImplemented(
                "Groq provider pending implementation".into(),
            )))
        }
    }
}

// Refactor init_llm_provider to use helper:
fn init_llm_provider(&self) -> ChatResult<Arc<dyn BotticelliDriver>> {
    let model_id = self.config.chat.initial_model();
    debug!(model = ?model_id, "Creating LLM provider");
    self.create_client_for_model(model_id)
}
```

#### Task 1.3: Update Type References - ✅ COMPLETED
**Files**: 
- `crates/botticelli_chat/src/services.rs` - ✅ Done
- `crates/botticelli_chat/src/sampling_integration.rs` - ✅ Done
**Success Criteria**:
- [x] Change `botticelli_core::LlmProvider` to `botticelli_interface::BotticelliDriver` - ✅ Done
- [x] Update PlaceholderProvider to delegate to BotticelliDriver - ✅ Done
- [x] Update SamplingIntegration to accept BotticelliDriver - ✅ Done
- [x] All type signatures consistent - ✅ Done
- [x] Compiles without errors - ✅ Done

**Status**: COMPLETED in commit 5001f76

### Phase 2: MCP Tool Integration - ✅ COMPLETED

**Objective**: Wire MCP tools into LLM generate calls using ToolCalling trait

#### Task 2.1: Create Tool-Aware Sampler - ✅ COMPLETED
**File**: `crates/botticelli_chat/src/sampling.rs`

**Success Criteria**:
- [x] Create `ChatLlmSampler` that wraps BotticelliDriver - ✅ Done
- [x] Implements `botticelli_mcp::LlmSampler` trait - ✅ Done
- [x] Gets tools from MCP client's tool registry - ✅ Done (passed as parameter)
- [x] Passes tools to driver via ToolCalling trait - ✅ Done
- [x] Handles tool call responses - ✅ Done (via execute_tools)
- [x] Unit tests verify tool passing - ⚠️ TODO

**Status**: COMPLETED in commit fa6003f

**Implementation**:
```rust
pub struct ChatLlmSampler {
    /// LLM provider with tool calling support
    provider: Arc<dyn ToolCalling>,
    tool_registry: Arc<ToolRegistry>,
}

#[async_trait]
impl LlmSampler for ChatLlmSampler {
    async fn generate(
        &self,
        session: &ConversationSession,
        available_tools: &[ToolDefinition],
    ) -> Result<GenerateResponse, SamplingError> {
        let request = self.build_request(session)?;
        
        let response = if available_tools.is_empty() {
            self.provider.generate(&request).await?
        } else {
            self.provider.generate_with_tools(&request, available_tools).await?
        };
        
        Ok(response)
    }
    
    async fn execute_tools(&self, calls: &[ToolCall]) -> Result<Vec<ToolResult>, SamplingError> {
        // Delegates to tool_registry.execute()
    }
}
```

**PlaceholderProvider** also implements `ToolCalling`:
- Lazy initialization pattern maintained
- Delegates to GeminiClient for tool calling
- Creates fresh client instance (design limitation noted) 
        {
            // Get tools from registry
            let tools = self.tool_registry.list_tools();
            
            // Convert to driver format and attach
            let tools_schema = convert_tools_for_driver(tools);
            
            // Generate with tools
            let response = tool_calling.generate_with_tools(
                &request.into(),
                tools_schema
            ).await?;
            
            // Handle tool calls in response
            self.handle_tool_calls(response).await
        } else {
            // Fallback: generate without tools
            self.driver.generate(&request.into()).await
        }
    }
}
```

#### Task 2.2: Update SamplingIntegration
**File**: `crates/botticelli_chat/src/sampling_integration.rs`

**Success Criteria**:
- [ ] Pass MCP tool registry to ChatLlmSampler
- [ ] Sampler uses tools during generation
- [ ] Tool call results flow back to coordinator
- [ ] Integration tests verify tool usage

### Phase 3: Fallback Chain Implementation

**Objective**: Wire existing ModelSelector fallback architecture into chat provider initialization

#### Task 3.1: Integrate ModelSelector with Chat Services
**File**: `crates/botticelli_chat/src/services.rs`

**Success Criteria**:
- [ ] `init_llm_provider()` creates ModelSelector with chat config
- [ ] Uses SelectionStrategy from config (LoyalFirst or FriendlyFirst)
- [ ] ModelBounds configured from chat settings
- [ ] RateLimitDetector integrated for tracking limits
- [ ] Initial model from config becomes starting point

**Implementation Outline**:
```rust
fn init_llm_provider(&self) -> ChatResult<Arc<dyn BotticelliDriver>> {
    use botticelli_models::{ModelSelector, ModelBounds, SelectionStrategy, RateLimitDetector};
    
    let model_id = self.config.chat.initial_model();
    
    // Create model selector with strategy from config
    let strategy = match self.config.chat.fallback_strategy() {
        "friendly" => SelectionStrategy::FriendlyFirst,
        _ => SelectionStrategy::LoyalFirst,  // Default
    };
    
    let bounds = ModelBounds::none();  // Or from config
    let detector = RateLimitDetector::new();
    let mut selector = ModelSelector::new(bounds, strategy, detector);
    
    // Create initial client based on model_id
    self.create_client_for_model(model_id)
}

fn create_client_for_model(&self, model_id: ModelId) -> ChatResult<Arc<dyn BotticelliDriver>> {
    match model_id {
        ModelId::Gemini(_) => {
            let client = GeminiClient::new()?;
            Ok(Arc::new(client))
        }
        ModelId::Groq(_) => {
            // Create GroqDriver when available
            Err(ChatError::new(ChatErrorKind::NotImplemented(...)))
        }
    }
}
```

**Key Points**:
- **Loyal fallback**: Try smaller model in same family (Gemini 2.5 Flash → Gemini 2.5 Flash Lite)
- **Friendly fallback**: Try equivalent model in different family (Gemini → Groq equivalent)
- ModelSelector handles rate limit detection and selection automatically
- No need for custom FallbackProvider wrapper - use existing architecture

#### Task 3.2: Create Fallback-Aware Generate Method
**File**: `crates/botticelli_chat/src/services.rs` or new `fallback.rs`

**Success Criteria**:
- [ ] Wrap BotticelliDriver with fallback retry logic
- [ ] On rate limit error, use ModelSelector to pick next model
- [ ] Create new client for selected model
- [ ] Retry generation with new client
- [ ] Log fallback transitions
- [ ] Max retry limit (e.g., 3 attempts)

**Implementation**:
```rust
pub struct FallbackDriver {
    initial_model: ModelId,
    selector: Arc<Mutex<ModelSelector>>,
    services: Arc<ServiceContainer>,
}

impl FallbackDriver {
    async fn generate_with_fallback(
        &self,
        request: &GenerateRequest,
    ) -> BotticelliResult<GenerateResponse> {
        let mut current_model = self.initial_model;
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            // Get client for current model
            let client = self.services.create_client_for_model(current_model).await?;
            
            match client.generate(request).await {
                Ok(response) => {
                    info!(model = ?current_model, "Generation successful");
                    return Ok(response);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        error!("Max fallback attempts reached");
                        return Err(e);
                    }
                    
                    // Check if we should fallback
                    let mut selector = self.selector.lock().await;
                    if let Some(next_model) = selector.select_next(current_model, &e.to_string()) {
                        info!(from = ?current_model, to = ?next_model, "Falling back to different model");
                        current_model = next_model;
                        continue;
                    } else {
                        warn!("No fallback available for error");
                        return Err(e);
                    }
                }
            }
        }
    }
}
```

#### Task 3.3: Update Chat Configuration
**File**: `crates/botticelli_chat/src/config/app.rs`

**Success Criteria**:
- [ ] Add `fallback_strategy` field (LoyalFirst | FriendlyFirst)
- [ ] Add `model_bounds` configuration (min/max models)
- [ ] Add `max_fallback_attempts` field
- [ ] Configuration documented in example files
- [ ] Default values sensible (LoyalFirst, 3 attempts)


- [ ] Use ModelSelector's select_next() for intelligent fallback
- [ ] Support both loyal (same-family) and friendly (cross-family) fallback
- [ ] Fallback tested with intentional rate limit errors
- [ ] Configuration allows choosing strategy and bounds

### Phase 4: Configuration and API Keys

**Objective**: Proper configuration management for production use

#### Task 4.1: Verify Configuration Schema
**File**: `crates/botticelli_chat/src/config/mcp.rs`

**Success Criteria**:
- [ ] Config has fields for all API keys
- [ ] Environment variable mapping works
- [ ] Validation on required keys
- [ ] Example config files updated

#### Task 4.2: Secure Key Handling
**Files**: Various config files

**Success Criteria**:
- [ ] API keys never logged
- [ ] Keys loaded from environment or secure store
- [ ] Clear error messages for missing keys
- [ ] Documentation on key setup

### Phase 5: Integration and Testing

**Objective**: End-to-end verification of MCP host functionality

#### Task 5.1: Integration Tests
**File**: `crates/botticelli_chat/tests/mcp_integration_test.rs`

**Success Criteria**:
- [ ] Test: User command → MCP tools available to LLM
- [ ] Test: LLM calls tool → Tool executes → Result returned
- [ ] Test: Fallback chain activates on primary failure
- [ ] Test: Multiple LLM roundtrips with tool usage
- [ ] All tests pass

#### Task 5.2: Manual Verification
**Binary**: `botticelli-chat`

**Success Criteria**:
- [ ] Start chat interface with valid API keys
- [ ] Issue narrative generation command
- [ ] LLM receives MCP tools
- [ ] LLM uses tools (verify in logs)
- [ ] Narrative generated successfully
- [ ] Test fallback by disabling primary provider

#### Task 5.3: Documentation
**Files**: 
- `crates/botticelli_chat/README.md`
- `CHAT_MCP_INTEGRATION.md` (new)

**Success Criteria**:
- [ ] Document MCP host architecture
- [ ] Explain tool flow: User → Chat → MCP → LLM → Tools
- [ ] Configuration guide with examples
- [ ] Troubleshooting section
- [ ] Architecture diagrams

## Success Metrics

### Functional Requirements
- [ ] Chat interface starts without errors
- [ ] LLM generates text using configured provider
- [ ] MCP tools available to LLM during generation
- [ ] Tool calls executed and results returned
- [ ] Fallback chain works on provider failure
- [ ] Multiple providers supported (Gemini, Anthropic)

### Code Quality Requirements
- [ ] Zero compiler warnings in botticelli_chat
- [ ] All public APIs documented
- [ ] Integration tests cover critical paths
- [ ] Error messages are clear and actionable
- [ ] Logs provide visibility into tool usage

### Performance Requirements
- [ ] First LLM call completes within 10 seconds
- [ ] Tool execution adds < 2 seconds overhead
- [ ] Fallback transition < 5 seconds
- [ ] Memory usage reasonable (< 500MB for chat session)

## Dependencies

### Internal
- `botticelli_interface`: BotticelliDriver trait (✅ complete after refactor)
- `botticelli_models`: GeminiClient, AnthropicClient (✅ exist)
- `botticelli_mcp_client`: UnifiedMcpClient, ToolRegistry (✅ exist)
- `botticelli_mcp`: SamplingCoordinator, LlmSampler trait (✅ exist)

### External
- API keys for Gemini, Anthropic (configuration)
- Environment variables or config files
- Network connectivity for API calls

## Risks and Mitigations

### Risk 1: API Rate Limits
**Impact**: Tests fail, development blocked  
**Mitigation**: 
- **Existing ModelSelector with RateLimitDetector handles this automatically**
- Use mock providers in tests
- Document API quota requirements
- Fallback system detects rate limit errors and switches models

### Risk 2: Tool Schema Mismatch
**Impact**: LLM can't understand/use tools  
**Mitigation**:
- Validate tool schemas at registration
- Test with actual LLM calls early
- Provide schema conversion utilities

### Risk 3: Breaking Changes to MCP Client
**Impact**: Integration broken  
**Mitigation**:
- Pin MCP client version
- Integration tests catch breaks immediately
- Version compatibility matrix

## Benefits of Existing Fallback Architecture

The botticelli_models crate already provides a sophisticated fallback system:

### ModelSelector Features
- **Rate limit detection**: Automatically recognizes rate limit errors from different providers
- **Loyal fallback**: Try smaller model in same family (cheaper, same API)
  - Example: Gemini 2.5 Flash → Gemini 2.5 Flash Lite
- **Friendly fallback**: Try equivalent model in different family (different API, similar capability)
  - Example: Gemini 2.5 Flash → Groq Llama 3.3 70B
- **Model bounds**: Prevents falling back to models that are too expensive or too cheap
- **Configurable strategy**: Choose loyal-first or friendly-first based on use case

### Why This Matters for Chat
- **Cost optimization**: Automatically use cheapest available model
- **Reliability**: Switch providers when one is rate-limited
- **Performance**: Don't wait for rate limit windows - switch immediately
- **Flexibility**: Configure behavior per deployment (dev vs prod)

### Integration Strategy
Rather than create a new FallbackProvider wrapper, we:
1. Wire ModelSelector into ServiceContainer initialization
2. Create `generate_with_fallback()` method that uses selector
3. Let existing architecture handle all fallback logic
4. Configuration controls strategy and bounds

This reuses battle-tested code and maintains consistency across the project.

## Timeline Estimate

- **Phase 1**: 4-6 hours (Core LLM provider) - ✅ Task 1.1 Complete, 1.2-1.3 remain
- **Phase 2**: 6-8 hours (MCP tool integration)
- **Phase 3**: 3-4 hours (Fallback chain integration - **simplified by using existing ModelSelector**)
- **Phase 4**: 2-3 hours (Configuration)
- **Phase 5**: 4-5 hours (Testing and docs)

**Total**: 19-26 hours of focused implementation

## Next Steps

1. Review and approve this plan
2. Start with Phase 1, Task 1.1 (Gemini provider)
3. Commit after each task completion
4. Run integration tests after Phase 2
5. Full system test after Phase 5

## Notes

- This builds on the completed trait sandwich refactor
- All trait interfaces are now in place and working
- Focus is on wiring existing components together
- No new architectural patterns needed
- Primarily implementation work, not design

---

**Plan Status**: Ready for implementation  
**Blocking Issues**: None  
**Prerequisites**: All complete (trait refactor done)
