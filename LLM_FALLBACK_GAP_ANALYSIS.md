# LLM Fallback System: Gap Analysis and Implementation Plan

## Current State Analysis

### What Exists

**1. Model Selection Infrastructure (`botticelli_models`)**
- ✅ `ModelSelector` - orchestrates fallback logic with boundary constraints
- ✅ `ModelId` - unified identifier across providers (Gemini, Groq)
- ✅ `ModelBounds` - upper/lower capability constraints
- ✅ `SelectionStrategy` - `LoyalFirst` (within-family) vs `FriendlyFirst` (cross-family)
- ✅ `RateLimitDetector` - identifies rate limit errors and tracks status
- ✅ Lateral equivalence mapping (`friends()`) - cross-provider model equivalents
- ✅ Vertical movement (`move_up()`/`move_down()`) - capability scaling within family

**2. Chat Integration (`botticelli_chat`)**
- ✅ `ChatSession` - wraps `ModelSelector` for chat-level fallback
- ✅ `handle_rate_limit()` - triggers fallback on rate limit errors
- ✅ Current model tracking

### What's Missing

#### 1. **No Default Configuration**
**Gap**: No defined default model or fallback strategy for TUI chat mode.

**Evidence**:
- `ChatSession::new()` requires explicit `ModelSelector` + `ModelId`
- No configuration file specifies chat defaults
- No environment-based model selection

**Impact**: Users must manually configure models; no "just works" experience.

#### 2. **No Configuration Loading**
**Gap**: Fallback configuration not loaded from TOML config files.

**Evidence**:
- `chat.toml`, `chat.local-dev.toml`, etc. exist but don't specify model fallback settings
- `ChatConfig` doesn't include model selection fields
- No integration between config system and `ModelSelector`

**Impact**: Cannot configure fallback behavior per environment (dev, staging, prod).

#### 3. **No Fallback Execution Integration**
**Gap**: Chat execution doesn't actually invoke fallback on LLM errors.

**Evidence**:
- `CommandExecutor` doesn't call `ChatSession::handle_rate_limit()`
- No retry loop in chat message processing
- LLM client errors bubble up without fallback attempt

**Impact**: Rate limit errors crash chat sessions instead of gracefully falling back.

#### 4. **Limited Provider Coverage**
**Gap**: Only Gemini and Groq models defined; other providers (Anthropic, Ollama) not integrated.

**Evidence**:
- `ModelId` enum only has `Gemini` and `Groq` variants
- Anthropic/Ollama clients exist but aren't in fallback system

**Impact**: Cannot use Claude or local models as fallback options.

#### 5. **No Observability for Fallback Events**
**Gap**: Fallback decisions not traced or metered.

**Evidence**:
- `ModelSelector::select_next()` lacks structured metrics emission
- No span tracking for fallback chains
- Cannot diagnose why specific fallback path was chosen

**Impact**: Debugging rate limit issues requires log archaeology.

#### 6. **No User Feedback**
**Gap**: Users don't know when fallback occurs or which model is active.

**Evidence**:
- TUI doesn't display current model
- No notification when model switches
- Silent failures when no fallback available

**Impact**: Poor UX; users confused by response quality changes.

---

## Implementation Roadmap

### Phase 1: Default Configuration (Priority: Critical)

**Goal**: Define sensible defaults for chat mode.

**Tasks**:

1.1. **Add Model Configuration to ChatConfig**
```rust
// crates/botticelli_chat/src/chat_config.rs
pub struct ChatConfig {
    // ... existing fields
    pub default_model: ModelId,
    pub fallback_strategy: SelectionStrategy,
    pub model_bounds: ModelBounds,
}
```

1.2. **Define Defaults in chat.toml**
```toml
[model]
default = "gemini:gemini-2.5-flash"
strategy = "loyal-first"  # or "friendly-first"

[model.bounds]
lower = "gemini:gemini-2.0-flash"
upper = "gemini:gemini-2.5-pro"
```

1.3. **Environment-Specific Overrides**
- `chat.local-dev.toml` → Gemini Flash Lite (cheap for testing)
- `chat.staging.toml` → Gemini 2.5 Flash (balanced)
- `chat.container.toml` → Production settings

**Success Criteria**:
- ✅ `just run-chat` uses defined default without manual config
- ✅ Different environments use appropriate models
- ✅ Configuration validates at startup (invalid model = clear error)

**Tests**:
- `tests/chat_config_defaults_test.rs` - validates config loading
- Mock LLM responses to verify correct model selected

---

### Phase 2: Fallback Execution Integration (Priority: Critical)

**Goal**: Actually invoke fallback when LLM calls fail.

**Tasks**:

2.1. **Add Retry Loop to Chat Execution**
```rust
// crates/botticelli_chat/src/executor.rs
impl CommandExecutor {
    async fn execute_with_fallback(
        &mut self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, ChatError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 3;

        loop {
            match self.llm_client.generate(request).await {
                Ok(response) => return Ok(response),
                Err(e) if attempts < MAX_ATTEMPTS => {
                    if let Some(new_model) = self.session.handle_rate_limit(&e.to_string())? {
                        tracing::info!(attempt = attempts, model = ?new_model, "Retrying with fallback");
                        // Update client to use new model
                        attempts += 1;
                    } else {
                        return Err(e.into());
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}
```

2.2. **Add Client Switching Logic**
- Store multiple LLM clients (one per provider)
- Switch active client when model changes family
- Maintain connection pooling

2.3. **Wire into Message Processing**
- Replace direct LLM calls with `execute_with_fallback()`
- Preserve message history across retries
- Track which model generated each response

**Success Criteria**:
- ✅ Rate limit from Gemini → automatically tries Groq equivalent
- ✅ Max 3 attempts before failing
- ✅ Message history preserved across retries

**Tests**:
- `tests/fallback_execution_test.rs` - mock rate limit responses
- Integration test: actual API calls with rate limit triggers

---

### Phase 3: Expand Provider Coverage (Priority: High)

**Goal**: Include Anthropic and Ollama in fallback system.

**Tasks**:

3.1. **Add Anthropic Models to ModelId**
```rust
pub enum ModelId {
    Gemini(GeminiModel),
    Groq(GroqModel),
    Anthropic(AnthropicModel),  // NEW
    Ollama(OllamaModel),         // NEW
}

pub enum AnthropicModel {
    Claude35Sonnet,
    Claude3Haiku,
}

pub enum OllamaModel {
    Llama3_8B,
    Mistral7B,
}
```

3.2. **Define Lateral Equivalences**
```rust
// Gemini 2.5 Flash ↔ Claude 3.5 Haiku ↔ Llama 3 8B
// Gemini 2.5 Pro ↔ Claude 3.5 Sonnet
```

3.3. **Implement friends() for New Models**

3.4. **Add Tier Positioning**

**Success Criteria**:
- ✅ Fallback can switch from Gemini → Claude
- ✅ Ollama usable as cheap fallback tier
- ✅ Cross-provider equivalences match capability

**Tests**:
- Unit tests for new model methods
- Integration: Gemini failure → Claude success

---

### Phase 4: Observability (Priority: High)

**Goal**: Make fallback decisions visible and debuggable.

**Tasks**:

4.1. **Add Structured Metrics**
```rust
// crates/botticelli_models/src/model_selector.rs
impl ModelSelector {
    pub fn select_next(&mut self, current: ModelId, error: &str) -> Option<ModelId> {
        let span = tracing::info_span!(
            "model_fallback",
            current = ?current,
            strategy = ?self.strategy,
        );
        let _enter = span.enter();

        // Emit metric
        metrics::counter!("llm.fallback.triggered", "family" => current.family().to_string()).increment(1);

        // ... existing logic

        if let Some(next) = result {
            tracing::info!(
                next = ?next,
                movement = if next.family() == current.family() { "loyal" } else { "friendly" },
                "Fallback selected"
            );
            metrics::counter!("llm.fallback.success", 
                "from" => current.to_string(),
                "to" => next.to_string()
            ).increment(1);
        } else {
            tracing::warn!("No fallback available");
            metrics::counter!("llm.fallback.exhausted").increment(1);
        }

        result
    }
}
```

4.2. **Add Grafana Dashboard Panel**
- Fallback rate by provider
- Most common fallback paths
- Exhaustion rate (no fallback available)

4.3. **Include Model in Response Metadata**
```rust
pub struct GenerateResponse {
    // ... existing
    pub model_used: ModelId,  // NEW - track which model generated response
}
```

**Success Criteria**:
- ✅ Jaeger traces show fallback decisions with context
- ✅ Prometheus metrics track fallback frequency
- ✅ Logs include model_used for every LLM response

**Tests**:
- Verify metrics emitted during fallback
- Check span attributes contain expected fields

---

### Phase 5: User Feedback (Priority: Medium)

**Goal**: Users see model changes in TUI.

**Tasks**:

5.1. **Add Model Display to TUI Status Bar**
```
[Gemini 2.5 Flash] │ Connected │ 3 messages
```

5.2. **Show Notification on Fallback**
```
⚠ Rate limit hit - switching to Groq Llama 3.3 70B
```

5.3. **Add Model Info to Message Metadata**
- Show which model generated each response
- Tooltip or metadata panel

**Success Criteria**:
- ✅ Current model visible in TUI at all times
- ✅ Users notified when fallback occurs
- ✅ Message history shows model per response

**Tests**:
- TUI rendering tests
- Screenshot comparison

---

### Phase 6: Advanced Features (Priority: Low)

**Goal**: Sophisticated fallback strategies.

**Tasks**:

6.1. **Token Budget Fallback**
- Track token usage across models
- Fall back when approaching budget limits
- "Cheap mode" - only use lower-tier models

6.2. **Response Quality Monitoring**
- Detect low-quality responses (short, repetitive)
- Automatically escalate to higher-tier model
- A/B testing different models

6.3. **User Preference Learning**
- Track which model user prefers (implicit feedback)
- Adapt default model based on usage patterns

**Success Criteria**:
- ✅ Budget exhaustion triggers fallback
- ✅ Poor responses trigger tier escalation
- ✅ System learns user preferences over time

**Tests**:
- Budget tracking accuracy
- Quality detection heuristics
- Preference convergence

---

## Validation Strategy

### Integration Tests (Boundary Testing)

**Test 1: Config → Selector**
```rust
#[test]
fn test_config_to_selector() {
    let config = ChatConfig::from_file("chat.local-dev.toml").unwrap();
    let selector = config.create_selector();
    assert_eq!(selector.default_model(), ModelId::Gemini(GeminiModel::Gemini25FlashLite));
}
```

**Test 2: Selector → Execution**
```rust
#[tokio::test]
async fn test_fallback_execution() {
    let mut session = ChatSession::new(/* ... */);
    let mock_client = MockLlmClient::with_rate_limit_then_success();
    
    let result = execute_with_fallback(&mut session, mock_client, request).await;
    
    assert!(result.is_ok());
    assert_eq!(session.current_model(), ModelId::Groq(GroqModel::Llama33_70BVersatile));
}
```

**Test 3: Execution → Observability**
```rust
#[tokio::test]
async fn test_fallback_metrics() {
    let recorder = TestMetricRecorder::new();
    
    trigger_fallback().await;
    
    assert_eq!(recorder.get_count("llm.fallback.triggered"), 1);
    assert!(recorder.get_span_events().iter().any(|e| e.name == "model_fallback"));
}
```

**Test 4: Observability → User Feedback**
```rust
#[test]
fn test_tui_shows_fallback() {
    let mut tui = TuiApp::new();
    tui.handle_fallback_event(/* ... */);
    
    let rendered = tui.render_status_bar();
    assert!(rendered.contains("Groq Llama"));
}
```

### End-to-End Test
```rust
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_full_fallback_pipeline() {
    // 1. Load config with Gemini default
    let config = ChatConfig::from_file("chat.test.toml").unwrap();
    
    // 2. Create session
    let mut session = ChatSession::from_config(&config);
    
    // 3. Execute with real API (Gemini rate limited)
    let response = session.execute_message("Hello").await.unwrap();
    
    // 4. Verify fallback occurred
    assert_eq!(session.current_model().family(), ModelFamily::Groq);
    
    // 5. Check observability
    assert_metrics_recorded();
    assert_trace_contains_fallback();
}
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Config schema breaks existing setups | Medium | High | Provide defaults; graceful degradation |
| Fallback loop exhausts rate limits | Low | Medium | Max retry limit (3); exponential backoff |
| Wrong model selected (equivalence mismatch) | Medium | Medium | Manual equivalence review; user override |
| TUI performance impact (frequent updates) | Low | Low | Debounce status bar updates |
| Metrics overhead | Low | Low | Sampling; async emission |

---

## Success Metrics

### Phase 1-2 (MVP)
- 🎯 **Config Coverage**: 100% of environments have model defaults
- 🎯 **Fallback Success Rate**: >90% of rate limits successfully handled
- 🎯 **Zero-Config UX**: `just run-chat` works without manual model selection

### Phase 3-4 (Production Ready)
- 🎯 **Provider Diversity**: ≥3 providers in fallback pool
- 🎯 **Observability**: 100% of fallback events traced + metered
- 🎯 **MTTR**: Fallback issues diagnosable in <5 min (from metrics/traces)

### Phase 5-6 (Optimized)
- 🎯 **User Awareness**: 100% of users see current model in TUI
- 🎯 **Budget Efficiency**: Token costs reduced 20% via smart fallback
- 🎯 **Quality Maintenance**: <5% response quality degradation during fallback

---

## Timeline Estimate

- **Phase 1 (Defaults)**: 1-2 days
- **Phase 2 (Execution)**: 2-3 days
- **Phase 3 (Providers)**: 2 days
- **Phase 4 (Observability)**: 1-2 days
- **Phase 5 (TUI)**: 1 day
- **Phase 6 (Advanced)**: 3-5 days (optional)

**Total MVP (Phases 1-2)**: ~5 days  
**Total Production (Phases 1-4)**: ~10 days  
**Total Complete (All phases)**: ~15 days

---

## Next Steps

1. **Review & Approve Plan** - Confirm strategy + priorities
2. **Create Feature Branch** - `feature/llm-fallback-integration`
3. **Implement Phase 1** - Start with config defaults (smallest, highest value)
4. **Integration Test** - Verify config → selector boundary
5. **Iterate** - Phases 2-4 in sequence with tests at each boundary

---

*Document Status: Draft for Review*  
*Created: 2025-12-18*  
*Owner: Botticelli Development Team*
