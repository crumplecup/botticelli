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

### ❌ Critical Gaps
1. **LLM Provider initialization returns errors** (services.rs:228-234)
2. **No BotticelliDriver instantiation** (should use GeminiClient, AnthropicClient)
3. **MCP tools not passed to LLM** (ToolCalling trait unused)
4. **No fallback chain** (Gemini → Anthropic → other)
5. **PlaceholderProvider delegates to non-existent provider**

## Implementation Phases

### Phase 1: Core LLM Provider Implementation

**Objective**: Replace error-returning stubs with actual LLM client instantiation

#### Task 1.1: Implement Gemini Provider
**File**: `crates/botticelli_chat/src/services.rs`

**Success Criteria**:
- [ ] `init_llm_provider()` creates `GeminiClient` for Gemini models
- [ ] Uses API key from config: `self.config.mcp_client.gemini_api_key()`
- [ ] Returns `Arc<dyn botticelli_interface::BotticelliDriver>` (not generic LlmProvider)
- [ ] Compiles without errors
- [ ] Unit test verifies GeminiClient is created

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

#### Task 1.2: Add Anthropic Fallback
**File**: `crates/botticelli_chat/src/services.rs`

**Success Criteria**:
- [ ] Add Anthropic model variant to match in `init_llm_provider()`
- [ ] Creates `AnthropicClient` when Anthropic model specified
- [ ] Uses API key from config
- [ ] Compiles and tests pass

#### Task 1.3: Update Type References
**Files**: 
- `crates/botticelli_chat/src/services.rs`
- `crates/botticelli_chat/src/sampling_integration.rs`

**Success Criteria**:
- [ ] Change `botticelli_core::LlmProvider` to `botticelli_interface::BotticelliDriver`
- [ ] Update PlaceholderProvider to delegate to BotticelliDriver
- [ ] Update SamplingIntegration to accept BotticelliDriver
- [ ] All type signatures consistent
- [ ] Compiles without errors

### Phase 2: MCP Tool Integration

**Objective**: Wire MCP tools into LLM generate calls using ToolCalling trait

#### Task 2.1: Create Tool-Aware Sampler
**File**: `crates/botticelli_chat/src/sampling.rs` (new file)

**Success Criteria**:
- [ ] Create `ChatLlmSampler` that wraps BotticelliDriver
- [ ] Implements `botticelli_mcp::LlmSampler` trait
- [ ] Gets tools from MCP client's tool registry
- [ ] Passes tools to driver via ToolCalling trait
- [ ] Handles tool call responses
- [ ] Unit tests verify tool passing

**Implementation Outline**:
```rust
pub struct ChatLlmSampler {
    driver: Arc<dyn BotticelliDriver>,
    tool_registry: Arc<ToolRegistry>,
}

#[async_trait]
impl LlmSampler for ChatLlmSampler {
    async fn sample(&self, request: &SamplingRequest) -> McpResult<String> {
        // Check if driver supports tools
        if let Some(tool_calling) = self.driver.as_any()
            .downcast_ref::<dyn ToolCalling>() 
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

**Objective**: Implement graceful degradation across LLM providers

#### Task 3.1: Create Fallback Provider Wrapper
**File**: `crates/botticelli_chat/src/fallback_provider.rs` (new file)

**Success Criteria**:
- [ ] Wraps multiple BotticelliDriver instances
- [ ] Tries primary, falls back to secondary on failure
- [ ] Logs fallback events
- [ ] Implements BotticelliDriver trait
- [ ] Configurable retry/timeout behavior

**Implementation**:
```rust
pub struct FallbackProvider {
    providers: Vec<(String, Arc<dyn BotticelliDriver>)>,
    config: FallbackConfig,
}

impl FallbackProvider {
    pub fn new(providers: Vec<(String, Arc<dyn BotticelliDriver>)>) -> Self {
        Self { providers, config: FallbackConfig::default() }
    }
}

#[async_trait]
impl BotticelliDriver for FallbackProvider {
    async fn generate(&self, request: &GenerateRequest) 
        -> Result<GenerateResponse, ModelsError> 
    {
        let mut last_error = None;
        
        for (name, provider) in &self.providers {
            match provider.generate(request).await {
                Ok(response) => {
                    info!(provider = name, "LLM generation successful");
                    return Ok(response);
                }
                Err(e) => {
                    warn!(provider = name, error = ?e, "Provider failed, trying next");
                    last_error = Some(e);
                    continue;
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| 
            ModelsError::new("No providers available")
        ))
    }
}
```

#### Task 3.2: Integrate Fallback Chain
**File**: `crates/botticelli_chat/src/services.rs`

**Success Criteria**:
- [ ] `init_llm_provider()` creates FallbackProvider
- [ ] Chain includes: Gemini (primary) → Anthropic (fallback)
- [ ] Can add more providers via config
- [ ] Fallback tested with intentional failures

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
- Use mock providers in tests
- Implement rate limiting in fallback chain
- Document API quota requirements

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

## Timeline Estimate

- **Phase 1**: 4-6 hours (Core LLM provider)
- **Phase 2**: 6-8 hours (MCP tool integration)
- **Phase 3**: 3-4 hours (Fallback chain)
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
