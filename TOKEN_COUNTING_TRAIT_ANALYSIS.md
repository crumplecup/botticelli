# Token Counting Trait Feasibility Analysis

## Current State

### Token Usage Tracking Exists

**Metrics Layer** (`botticelli_models/src/metrics.rs`):
- OpenTelemetry counters for `prompt_tokens`, `completion_tokens`, `total_tokens`
- Already integrated into driver implementations
- Labeled by provider and model
- Global singleton accessible via `LlmMetrics::get()`

**Provider-Specific Usage Types**:
- `AnthropicUsage`: `input_tokens: u32, output_tokens: u32`
- `ChatUsage` (OpenAI-compat): `prompt_tokens, completion_tokens, total_tokens` (all `Option<usize>`)
- `UsageMetadata` (Gemini): `prompt_token_count, candidates_token_count, total_token_count` (all `Option<u32>`)

**Response Integration**:
- Each provider returns usage in their native response format
- Drivers extract and report to metrics during `generate()` calls
- Token data flows: API response → Driver → Metrics → OpenTelemetry

### What's Missing

**No Unified Token Interface**:
- Each provider has different field names and types
- No trait to abstract over token counting
- Difficult to query "how many tokens did this request use?" without knowing provider

**No Request-Level Token Tracking**:
- Metrics are global counters (good for observability)
- But cannot answer "what were the token counts for this specific call?"
- No way to return usage alongside `GenerateResponse`

**Estimation vs. Actual**:
- Gemini driver has `estimate_tokens()` helper (4 chars = 1 token heuristic)
- Used for rate limit pre-checks before API call
- But no trait/interface for token estimation across providers

## Proposed TokenCounting Trait

### Option 1: Minimal Token Result Trait

```rust
/// Token usage information for a completed generation.
pub trait TokenUsage {
    /// Tokens in the prompt/input.
    fn input_tokens(&self) -> Option<u64>;
    
    /// Tokens in the generated output.
    fn output_tokens(&self) -> Option<u64>;
    
    /// Total tokens (may differ from input + output due to provider accounting).
    fn total_tokens(&self) -> Option<u64> {
        match (self.input_tokens(), self.output_tokens()) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        }
    }
}

// Implement for provider-specific types
impl TokenUsage for AnthropicUsage { ... }
impl TokenUsage for ChatUsage { ... }
impl TokenUsage for UsageMetadata { ... }
```

**Pros**:
- Simple, focused interface
- Easy to implement for existing types
- Provides unified query API

**Cons**:
- Doesn't help with estimation
- Still requires provider-specific response types
- Doesn't address "return usage with response" problem

### Option 2: Extended Trait with Estimation

```rust
pub trait TokenCounter: Send + Sync {
    /// Estimate tokens in text (pre-request).
    fn estimate_tokens(&self, text: &str) -> u64;
    
    /// Extract actual token usage from a response (post-request).
    fn extract_usage(&self, response: &GenerateResponse) -> Option<TokenUsageData>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenUsageData {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

// Each driver implements TokenCounter
impl TokenCounter for GeminiClient { ... }
impl TokenCounter for AnthropicClient { ... }
```

**Pros**:
- Covers both estimation and extraction
- Could be used for rate limit pre-checks
- Single interface for all token operations

**Cons**:
- Estimation accuracy varies wildly by provider
- Different providers tokenize differently (no universal estimator)
- Adds complexity to driver implementations

### Option 3: Add Usage to GenerateResponse

```rust
// In botticelli_core
pub struct GenerateResponse {
    outputs: Vec<Output>,
    usage: Option<TokenUsageData>,  // NEW
}

// Drivers populate usage when available
impl BotticelliDriver for GeminiClient {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        let response = self.client.post(...).await?;
        
        let usage = response.usage_metadata.map(|u| TokenUsageData {
            input_tokens: u.prompt_token_count.unwrap_or(0) as u64,
            output_tokens: u.candidates_token_count.unwrap_or(0) as u64,
            total_tokens: u.total_token_count.unwrap_or(0) as u64,
        });
        
        Ok(GenerateResponse::builder()
            .outputs(outputs)
            .usage(usage)
            .build())
    }
}
```

**Pros**:
- Natural API: usage travels with response
- No new trait to implement
- Backward compatible (Option field)
- Works with existing metrics reporting

**Cons**:
- Doesn't solve estimation problem
- Modifies core response type (but non-breaking)

## Recommendation

### SHORT TERM: **Option 3 + Keep Existing Metrics**

**Reasoning**:
1. **Immediate Value**: Narrative execution can access token counts per act
2. **Minimal Work**: Add optional field to existing type, populate in drivers
3. **No Breaking Changes**: Existing code continues working
4. **Complements Metrics**: Request-level data for debugging, global metrics for dashboards

**Implementation Steps**:
1. Add `usage: Option<TokenUsageData>` to `GenerateResponse` (botticelli_core)
2. Update each driver's `generate()` to populate usage from API response
3. Update narrative executor to track usage per act
4. Add usage to MCP tool responses for observability

**Estimated Effort**: 2-3 hours
- Core type update: 30 min
- Driver updates (4 providers): 1.5 hours
- Narrative/MCP integration: 1 hour

### LONG TERM: Consider Token Estimation Trait

**If** we need pre-request token estimation for:
- Rate limit budgeting before API calls
- User quota warnings
- Cost estimation

**Then** add a separate `TokenEstimator` trait, but:
- Document accuracy limitations
- Provider-specific implementations (no universal tokenizer)
- Optional feature (not required for basic functionality)

**Not urgent** because:
- Gemini driver already has internal estimation for rate limiting
- Actual usage from responses is more accurate
- Estimation is hard to do right (provider-specific tokenizers)

## Decision: Take or Leave?

### ✅ TAKE Option 3 (Add usage to GenerateResponse)

**Why**:
- Directly addresses Phase 3 observability requirement (token tracking per narrative)
- Low effort, high value
- Enables MCP tools to return token counts
- Foundation for cost monitoring
- Natural fit with existing architecture

**Why Not a Sidetrack**:
- It's literally in our Phase 3 requirements ("Token usage tracking")
- 2-3 hours is manageable
- Makes narrative observability actually useful
- MCP tools can report "Act used X tokens"

### ❌ LEAVE Token Estimation Trait

**Why**:
- Not needed for current requirements
- Complex to implement correctly (provider-specific tokenizers)
- Actual usage from responses is sufficient
- Can add later if needed

## Conclusion

**Proceed with Option 3**: Add `usage: Option<TokenUsageData>` to `GenerateResponse`.

This is NOT a sidetrack—it's a core Phase 3 requirement. The work is scoped, valuable, and integrates cleanly with existing systems.
