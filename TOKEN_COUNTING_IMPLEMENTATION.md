# TokenCounting Trait Implementation Plan

## Current State

- ✅ `TokenCounting` trait defined in `botticelli_interface`
- ✅ Metrics infrastructure in `botticelli_models::metrics`
- ✅ Observability with tracing
- ❌ No backend implementations of `TokenCounting`
- ❌ No integration with narrative execution
- ❌ No cost tracking

## Implementation Phases

### Phase 1: Backend Implementations

Implement `TokenCounting` for each LLM backend:

1. **Gemini** - Use API's `countTokens` endpoint
2. **Claude** - Use tiktoken approximation (Claude uses similar tokenizer to GPT)
3. **Groq** - Use tiktoken (most Groq models are GPT-based)
4. **Ollama** - Use model-specific tokenizers via API
5. **HuggingFace** - Use transformers tokenizers

#### Implementation Strategy

**Accurate (API-based):**
- Gemini: Has dedicated `countTokens` API endpoint
- Ollama: Can query model tokenizer

**Approximate (tiktoken):**
- Claude: Use tiktoken with `cl100k_base` encoding
- Groq: Use tiktoken based on model family
- HuggingFace: Use tiktoken as fallback

**Dependencies:**
```toml
tiktoken-rs = "0.5"  # For Claude, Groq approximations
```

### Phase 2: Observability Integration

Add token tracking to spans:

```rust
#[instrument(skip(self, req), fields(
    model = %self.model_name(),
    prompt_tokens,
    completion_tokens,
    total_tokens
))]
async fn generate(&self, req: &GenerateRequest) -> Result<Response> {
    let prompt_tokens = self.count_request_tokens(req)?;
    tracing::Span::current().record("prompt_tokens", prompt_tokens);
    
    let response = self.do_generate(req).await?;
    
    if let Some(usage) = response.usage() {
        tracing::Span::current().record("completion_tokens", usage.completion_tokens);
        tracing::Span::current().record("total_tokens", usage.total_tokens);
    }
    
    Ok(response)
}
```

### Phase 3: Cost Calculation

Add cost estimation per provider:

```rust
/// Cost estimation for LLM usage.
pub struct CostEstimator {
    /// Cost per 1M input tokens (in USD).
    input_cost_per_million: f64,
    /// Cost per 1M output tokens (in USD).
    output_cost_per_million: f64,
}

impl CostEstimator {
    pub fn estimate(&self, prompt_tokens: usize, completion_tokens: usize) -> f64 {
        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (completion_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }
}
```

Pricing (as of Dec 2024):
- Gemini 1.5 Flash: $0.075/1M input, $0.30/1M output
- Claude 3.5 Sonnet: $3.00/1M input, $15.00/1M output
- Groq (Llama 3.1 70B): $0.59/1M input, $0.79/1M output
- Ollama: Free (local)

### Phase 4: Narrative Integration

Track tokens across narrative execution:

```rust
pub struct NarrativeMetrics {
    pub total_prompt_tokens: usize,
    pub total_completion_tokens: usize,
    pub estimated_cost_usd: f64,
    pub acts_executed: usize,
}

impl NarrativeExecutor {
    #[instrument(skip(self), fields(
        narrative = %narrative_name,
        total_tokens,
        estimated_cost
    ))]
    pub async fn execute(&self, narrative: &Narrative) -> Result<NarrativeMetrics> {
        let mut metrics = NarrativeMetrics::default();
        
        for act in &narrative.acts {
            let result = self.execute_act(act).await?;
            metrics.total_prompt_tokens += result.prompt_tokens;
            metrics.total_completion_tokens += result.completion_tokens;
            metrics.acts_executed += 1;
        }
        
        metrics.estimated_cost_usd = self.cost_estimator.estimate(
            metrics.total_prompt_tokens,
            metrics.total_completion_tokens,
        );
        
        tracing::Span::current().record("total_tokens", 
            metrics.total_prompt_tokens + metrics.total_completion_tokens);
        tracing::Span::current().record("estimated_cost", metrics.estimated_cost_usd);
        
        Ok(metrics)
    }
}
```

### Phase 5: MCP Integration

Expose token counting via MCP tools:

```json
{
  "tools": [
    {
      "name": "count_tokens",
      "description": "Count tokens in text using specified model's tokenizer",
      "inputSchema": {
        "type": "object",
        "properties": {
          "text": { "type": "string" },
          "model": { "type": "string", "enum": ["gemini", "claude", "groq"] }
        },
        "required": ["text", "model"]
      }
    },
    {
      "name": "estimate_cost",
      "description": "Estimate cost for a narrative execution",
      "inputSchema": {
        "type": "object",
        "properties": {
          "narrative_path": { "type": "string" },
          "model": { "type": "string" }
        },
        "required": ["narrative_path", "model"]
      }
    }
  ]
}
```

### Phase 6: Testing

1. Unit tests for each backend's `count_tokens`
2. Integration tests comparing API token counts vs approximations
3. Cost calculation tests with known token counts
4. Narrative execution with token tracking
5. MCP tool tests

## Success Criteria

- ✅ All LLM backends implement `TokenCounting`
- ✅ Token counts recorded in traces
- ✅ Cost estimation available for all models
- ✅ Narrative execution reports token usage
- ✅ MCP exposes token counting tools
- ✅ <5% error rate for approximations (compare with API counts)
- ✅ Zero performance regression (token counting is async/cached where possible)

## Implementation Order

1. ✅ Add tiktoken-rs dependency
2. ✅ Implement `TokenCounting` for all backends (Gemini, Claude, Groq, Ollama, HuggingFace)
3. ⏳ Add `CostEstimator` infrastructure
4. ⏳ Integrate with observability spans
5. ⏳ Add to narrative executor
6. ⏳ Expose via MCP tools
7. ⏳ Comprehensive testing

## Phase 1 Complete (Dec 5, 2024)

Successfully implemented `TokenCounting` trait for all LLM backends:

- **Gemini**: Character approximation (chars / 4)
- **Claude**: tiktoken cl100k_base encoding
- **Groq**: tiktoken cl100k_base encoding
- **Ollama**: tiktoken cl100k_base encoding  
- **HuggingFace**: tiktoken cl100k_base encoding

All implementations instrumented with tracing for observability.

Tests added in `token_counting` module confirming tokenizer loading works correctly.

## Notes

- Token counting should be **best-effort** - failures shouldn't block generation
- Cache token counts for repeated text (narrative templates)
- Log discrepancies between estimated and actual API-reported tokens
- Use structured logging for easy analysis: `token.prompt`, `token.completion`, `cost.usd`
