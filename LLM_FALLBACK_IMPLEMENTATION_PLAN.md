# LLM Fallback System - Implementation Plan

## Overview

Implement comprehensive LLM provider fallback with configurable defaults in `botticelli.toml` and hardcoded fallbacks for robustness.

**Strategy Philosophy:**
- **botticelli.toml**: User-configurable defaults
- **Code defaults**: Hardcoded fallbacks if config missing
- **Strategy selection**: "loyal" (same provider) vs "friendly" (any provider)

---

## Phase 1: Configuration Layer

### Task 1.1: Define Fallback Configuration Schema

**File:** `crates/botticelli_config/src/llm.rs`

**Action:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct LlmFallbackConfig {
    strategy: FallbackStrategy,
    max_retries: u32,
    default_provider: String,
    default_model: String,
    fallback_chain: Vec<LlmProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FallbackStrategy {
    Loyal,    // Try alternative models from same provider
    Friendly, // Try any available provider
}
```

**Success Criteria:**
- [ ] Types compile with all derives
- [ ] Serde serialization/deserialization works
- [ ] Unit test: deserialize valid TOML config

---

### Task 1.2: Add Defaults to botticelli.toml

**File:** `botticelli.toml`

**Action:**
```toml
[llm.fallback]
strategy = "friendly"
max_retries = 3
default_provider = "anthropic"
default_model = "claude-3-5-sonnet-20241022"

[[llm.fallback.chain]]
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"

[[llm.fallback.chain]]
provider = "anthropic"
model = "claude-3-5-haiku-20241022"

[[llm.fallback.chain]]
provider = "openai"
model = "gpt-4o"

[[llm.fallback.chain]]
provider = "openai"
model = "gpt-4o-mini"
```

**Success Criteria:**
- [ ] Config loads successfully: `just check`
- [ ] Values accessible in `BotticelliConfig`

---

### Task 1.3: Implement Hardcoded Fallbacks

**File:** `crates/botticelli_config/src/llm.rs`

**Action:**
```rust
impl Default for LlmFallbackConfig {
    fn default() -> Self {
        Self {
            strategy: FallbackStrategy::Friendly,
            max_retries: 3,
            default_provider: "anthropic".to_string(),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
            fallback_chain: vec![
                LlmProviderConfig {
                    provider: "anthropic".to_string(),
                    model: "claude-3-5-sonnet-20241022".to_string(),
                },
                LlmProviderConfig {
                    provider: "anthropic".to_string(),
                    model: "claude-3-5-haiku-20241022".to_string(),
                },
                LlmProviderConfig {
                    provider: "openai".to_string(),
                    model: "gpt-4o".to_string(),
                },
            ],
        }
    }
}
```

**Success Criteria:**
- [ ] `LlmFallbackConfig::default()` returns valid config
- [ ] Test: config loads even with missing TOML section
- [ ] Test: partial TOML merges with defaults

---

## Phase 2: Fallback Orchestration

### Task 2.1: Implement FallbackOrchestrator

**File:** `crates/botticelli_core/src/llm/fallback.rs` (new)

**Action:**
```rust
use botticelli_interface::LlmProvider;
use botticelli_config::LlmFallbackConfig;

pub struct FallbackOrchestrator {
    config: LlmFallbackConfig,
    providers: HashMap<String, Box<dyn LlmProvider>>,
}

impl FallbackOrchestrator {
    pub fn new(config: LlmFallbackConfig, providers: HashMap<String, Box<dyn LlmProvider>>) -> Self {
        Self { config, providers }
    }

    #[tracing::instrument(skip(self, request))]
    pub async fn generate_with_fallback(
        &self,
        request: GenerateRequest,
    ) -> Result<GenerateResponse, LlmError> {
        let chain = match self.config.strategy() {
            FallbackStrategy::Loyal => self.build_loyal_chain(&request),
            FallbackStrategy::Friendly => self.build_friendly_chain(&request),
        };

        let mut last_error = None;
        for (attempt, provider_config) in chain.iter().enumerate() {
            tracing::info!(
                attempt,
                provider = %provider_config.provider,
                model = %provider_config.model,
                "Attempting LLM generation"
            );

            match self.try_provider(provider_config, &request).await {
                Ok(response) => {
                    tracing::info!("Generation succeeded");
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "Provider failed");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::new("All providers failed")))
    }

    fn build_loyal_chain(&self, request: &GenerateRequest) -> Vec<&LlmProviderConfig> {
        // Filter fallback chain to same provider as request
        self.config.fallback_chain()
            .iter()
            .filter(|c| c.provider == request.provider())
            .collect()
    }

    fn build_friendly_chain(&self, _request: &GenerateRequest) -> Vec<&LlmProviderConfig> {
        // Use full fallback chain
        self.config.fallback_chain().iter().collect()
    }

    async fn try_provider(
        &self,
        config: &LlmProviderConfig,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, LlmError> {
        let provider = self.providers.get(&config.provider)
            .ok_or_else(|| LlmError::new(format!("Provider not found: {}", config.provider)))?;

        // Clone request and override model
        let mut request = request.clone();
        request.set_model(config.model.clone());

        provider.generate(request).await
    }
}
```

**Success Criteria:**
- [ ] Compiles with trait constraints
- [ ] Unit test: loyal strategy filters to same provider
- [ ] Unit test: friendly strategy uses all providers
- [ ] Unit test: fallback succeeds on second provider
- [ ] Unit test: all failures returns last error

---

### Task 2.2: Integrate with LlmClient

**File:** `crates/botticelli_core/src/llm/client.rs`

**Action:**
```rust
pub struct LlmClient {
    orchestrator: FallbackOrchestrator,
}

impl LlmClient {
    pub fn new(config: LlmFallbackConfig, providers: HashMap<String, Box<dyn LlmProvider>>) -> Self {
        Self {
            orchestrator: FallbackOrchestrator::new(config, providers),
        }
    }

    #[tracing::instrument(skip(self, request))]
    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, LlmError> {
        self.orchestrator.generate_with_fallback(request).await
    }
}
```

**Success Criteria:**
- [ ] `LlmClient` uses orchestrator
- [ ] Integration test: full fallback flow end-to-end
- [ ] Observability: spans show fallback attempts

---

## Phase 3: Chat Integration

### Task 3.1: Wire Fallback into Chat Binary

**File:** `crates/botticelli_chat/src/main.rs`

**Action:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = BotticelliConfig::load()?;
    
    // Build provider map
    let providers = build_providers(&config)?;
    
    // Create LLM client with fallback
    let llm_client = LlmClient::new(
        config.llm.fallback.clone(),
        providers,
    );
    
    // Pass to TUI
    let tui = Tui::new(llm_client);
    tui.run().await?;
    
    Ok(())
}
```

**Success Criteria:**
- [ ] Chat binary compiles
- [ ] TUI uses fallback-enabled client
- [ ] Manual test: primary provider fails → fallback succeeds
- [ ] Logs show fallback chain attempts

---

### Task 3.2: Add User-Visible Fallback Indicators

**File:** `crates/botticelli_tui/src/ui/chat.rs`

**Action:**
- Display current provider/model in status bar
- Show fallback indicator when provider switches
- Log fallback events to UI message area

**Success Criteria:**
- [ ] UI shows active provider/model
- [ ] Fallback event visible to user
- [ ] User can understand what happened

---

## Phase 4: Testing & Validation

### Task 4.1: Integration Tests

**File:** `crates/botticelli_core/tests/llm_fallback_integration_test.rs`

**Tests:**
1. **Primary success**: No fallback triggered
2. **Primary fail, secondary success**: Fallback works
3. **All fail**: Returns appropriate error
4. **Loyal strategy**: Only tries same provider
5. **Friendly strategy**: Tries all providers
6. **Config missing**: Uses hardcoded defaults

**Success Criteria:**
- [ ] All tests pass: `just test-package botticelli_core`
- [ ] Tests use mock providers (no API calls)

---

### Task 4.2: API Tests (Rate-Limited)

**File:** `crates/botticelli_core/tests/llm_fallback_api_test.rs`

**Tests:**
1. Test with real Anthropic API (primary)
2. Simulate failure, test OpenAI fallback
3. Test loyal vs friendly with real providers

**Gating:**
```rust
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_real_fallback_anthropic_to_openai() {
    // ...
}
```

**Success Criteria:**
- [ ] Tests pass with `just test-api`
- [ ] Minimal token usage (< 100 tokens per test)
- [ ] Tests demonstrate real-world fallback

---

### Task 4.3: Manual Testing Checklist

**Scenarios:**
1. [ ] Run chat with default config → uses Claude 3.5 Sonnet
2. [ ] Disable Anthropic key → falls back to OpenAI
3. [ ] Set strategy to "loyal" → only tries Anthropic models
4. [ ] Set strategy to "friendly" → tries all providers
5. [ ] Remove fallback section from TOML → uses hardcoded defaults
6. [ ] Check observability: Jaeger shows fallback spans

---

## Phase 5: Documentation

### Task 5.1: Update Configuration Docs

**File:** `README.md` (Configuration section)

**Content:**
- Explain fallback strategies (loyal vs friendly)
- Show example botticelli.toml config
- Describe hardcoded defaults
- Link to troubleshooting

**Success Criteria:**
- [ ] Documentation clear and complete
- [ ] Example config is copy-pasteable

---

### Task 5.2: Update Troubleshooting Guide

**File:** `TROUBLESHOOTING.md`

**Add section:**
```markdown
## LLM Provider Failures

**Symptom:** Generation requests fail

**Diagnosis:**
1. Check logs for fallback attempts
2. Verify API keys configured for fallback providers
3. Check Jaeger for provider spans

**Solution:**
- Configure fallback chain in botticelli.toml
- Ensure multiple providers configured
- Use "friendly" strategy for maximum reliability
```

**Success Criteria:**
- [ ] Troubleshooting guide updated
- [ ] Links to relevant config sections

---

## Success Metrics

### Functional Requirements
- [ ] All phases complete
- [ ] All tests passing (unit + integration)
- [ ] Zero compilation errors/warnings
- [ ] Manual testing checklist complete

### Observability
- [ ] Fallback attempts visible in Jaeger
- [ ] Metrics track fallback rate
- [ ] Logs show clear fallback progression

### User Experience
- [ ] TUI shows active provider/model
- [ ] Fallback transparent to user (just works)
- [ ] Config intuitive and well-documented

---

## Implementation Order

1. **Phase 1** (Configuration) - Foundation
2. **Phase 2** (Orchestration) - Core logic
3. **Phase 4.1** (Unit tests) - Validate logic
4. **Phase 3** (Integration) - Wire into binaries
5. **Phase 4.2-4.3** (Integration/manual tests) - End-to-end validation
6. **Phase 5** (Documentation) - User-facing docs

**Estimated Effort:** 3-4 focused implementation sessions

---

## Rollback Plan

If issues arise:
- Phase 1-2: Easy rollback (config/core only)
- Phase 3+: Feature gate behind `llm-fallback` feature flag
- Emergency: Revert to single-provider behavior

---

## Notes

- All error types use `derive_more::Display` + `derive_more::Error`
- All public functions have `#[tracing::instrument]`
- No `#[allow]` directives - fix root causes
- Private fields + derive_getters/setters
- Import from source crates, no re-exports between workspace crates
