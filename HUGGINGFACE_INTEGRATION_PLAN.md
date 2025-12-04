# HuggingFace Inference API Integration Plan

## Overview

Integrate HuggingFace Inference API support into Botticelli using a custom `reqwest`-based implementation, following the same pattern as the Anthropic integration. HuggingFace provides $0.10/month in free credits for all users, making it excellent for testing and prototyping.

## Implementation Approach: Custom Reqwest Client

**Rationale:**
- Follow established Anthropic pattern for consistency
- Direct API control and error handling
- No dependencies on incomplete/experimental crates
- Simple, maintainable implementation

**API Details:**
- Base URL: `https://router.huggingface.co/v1/chat/completions`
- Authentication: `Authorization: Bearer {token}` header
- Format: OpenAI-compatible chat completions
- Request: `{"model": "...", "messages": [...], "max_tokens": ...}`
- Response: OpenAI format with `choices[0].message.content`
- **Important**: Only works with chat-capable models (not base models like gpt2)

## Free Tier Details

- **Credits**: $0.10 USD/month (free), $2.00/month (Pro users)
- **Access**: All public models on HuggingFace Hub
- **Rate Limits**: Shared infrastructure with modest limits
- **Suitable for**: Testing, demos, prototyping, small personal apps

## Implementation Phases

### Phase 1: Feature Gates and Dependencies ✅

**Status:** Complete - feature gates already in place

**Files:**
- `Cargo.toml` (workspace root)
- `crates/botticelli_models/Cargo.toml` - uses reqwest (already present)
- `crates/botticelli_error/Cargo.toml` - has `huggingface` feature
- `crates/botticelli/Cargo.toml`

### Phase 2: Error Types ✅

**Status:** Complete

**Files:**
- `crates/botticelli_error/src/models.rs` - `HuggingFaceErrorKind` defined
- `crates/botticelli_error/src/lib.rs` - exported

### Phase 3: DTOs (Request/Response Types) ✅

**Status:** Complete

**Files:**
- `crates/botticelli_models/src/huggingface/dto.rs` - All DTOs with builders

### Phase 4: Type Conversions ✅

**Status:** Complete

**Files:**
- `crates/botticelli_models/src/huggingface/conversions.rs`

### Phase 5: Driver Implementation ✅

**Status:** Complete - Using reqwest following Anthropic pattern

**Files:**
- `crates/botticelli_models/src/huggingface/driver.rs`
- `crates/botticelli_models/src/huggingface/mod.rs`
- `crates/botticelli_models/src/lib.rs` - exports

**Key implementation details:**
- Endpoint: `https://router.huggingface.co/v1/chat/completions`
- Uses OpenAI-compatible chat completions format
- Messages with roles (user/assistant/system)
- Direct message construction (conversions module not actively used)
- Response: `choices[0].message.content` extraction
- Streaming falls back to non-streaming (placeholder for future)
- Only supports chat-capable models

### Phase 6: Configuration Integration

**Status:** Not yet implemented

**Tasks:**
1. Create `crates/botticelli_models/src/huggingface/` module
2. Define HuggingFace-specific DTOs with builders
3. All fields private with getters (derive_getters)
4. Use derive_builder for construction

**Files:**
- `crates/botticelli_models/src/huggingface/mod.rs`
- `crates/botticelli_models/src/huggingface/request.rs`
- `crates/botticelli_models/src/huggingface/response.rs`

**Types needed:**
```rust
// Request DTO
#[derive(Debug, Clone, derive_getters::Getters, derive_builder::Builder)]
#[builder(setter(into))]
pub struct HuggingFaceRequest {
    model: String,
    inputs: String,
    #[builder(default)]
    parameters: Option<HuggingFaceParameters>,
}

// Parameters DTO
#[derive(Debug, Clone, derive_getters::Getters, derive_builder::Builder)]
#[builder(setter(into))]
pub struct HuggingFaceParameters {
    #[builder(default)]
    max_new_tokens: Option<u32>,
    #[builder(default)]
    temperature: Option<f32>,
    #[builder(default)]
    top_p: Option<f32>,
}

// Response DTO
#[derive(Debug, Clone, derive_getters::Getters, derive_builder::Builder)]
pub struct HuggingFaceResponse {
    generated_text: String,
    #[builder(default)]
    metadata: Option<HuggingFaceMetadata>,
}
```

### Phase 4: Type Conversions

**Tasks:**
1. Implement `TryFrom<GenerateRequest>` for `HuggingFaceRequest`
2. Implement `TryFrom<HuggingFaceResponse>` for `GenerateResponse`
3. Helper functions for message/content conversion
4. Handle streaming vs non-streaming cases

**Files:**
- `crates/botticelli_models/src/huggingface/conversions.rs`

**Key conversions:**
- `GenerateRequest` → HuggingFace API format (text inputs)
- HuggingFace response → `GenerateResponse` with text outputs
- Handle role-based messages → plain text concatenation
- Map parameters (max_tokens, temperature, etc.)

### Phase 5: BotticelliDriver Implementation

**Tasks:**
1. Create `HuggingFaceDriver` struct
2. Wrap `huggingface_inference_rs` client
3. Implement `BotticelliDriver` trait
4. Handle API key from environment (`HUGGINGFACE_API_TOKEN`)
5. Implement both `generate()` and `generate_stream()`

**Files:**
- `crates/botticelli_models/src/huggingface/driver.rs`

**Implementation notes:**
```rust
#[cfg(feature = "huggingface")]
pub struct HuggingFaceDriver {
    client: HuggingFaceClient,
    model: String,
}

impl HuggingFaceDriver {
    pub fn new(api_key: String, model: String) -> ModelsResult<Self> {
        let client = HuggingFaceClient::new(api_key);
        Ok(Self { client, model })
    }
}

#[cfg(feature = "huggingface")]
impl BotticelliDriver for HuggingFaceDriver {
    async fn generate(&self, request: GenerateRequest) 
        -> Result<GenerateResponse, BotticelliError> {
        // Convert request
        // Call API
        // Convert response
    }
    
    async fn generate_stream(&self, request: GenerateRequest) 
        -> Result<BoxStream<'static, Result<StreamChunk, BotticelliError>>, BotticelliError> {
        // May need custom implementation or use non-streaming
    }
}
```

### Phase 6: Configuration Integration

**Tasks:**
1. Add HuggingFace to `LlmProvider` enum
2. Update provider factory in `botticelli_core`
3. Add TOML configuration support
4. Environment variable handling

**Files:**
- `crates/botticelli_core/src/provider.rs`
- Configuration examples

**TOML config:**
```toml
[llm]
provider = "huggingface"
model = "meta-llama/Llama-2-7b-chat-hf"  # Or other available models

[llm.auth]
api_key_env = "HUGGINGFACE_API_TOKEN"
```

### Phase 7: Testing ✅

**Status:** Complete

**Files:**
- `crates/botticelli_models/tests/huggingface_api_test.rs`

**Tests:**
- ✅ Basic generation test with meta-llama/Llama-3.2-1B-Instruct
- ✅ Multiple model test (Llama-3.2-1B-Instruct works, Mistral-7B fails as not chat model)
- All tests feature-gated with `#[cfg_attr(not(feature = "api"), ignore)]`
- Minimal token usage (5-10 tokens per request)
- All API tests passing

### Phase 8: Documentation

**Tasks:**
1. Create unit tests for conversions
2. Create integration tests with API (feature-gated)
3. Test multiple models if credits allow
4. Document recommended models for free tier

**Files:**
- `crates/botticelli_models/tests/huggingface_test.rs`

**Test strategy:**
```rust
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
#[cfg(feature = "huggingface")]
async fn test_huggingface_basic_generation() -> ModelsResult<()> {
    let api_key = std::env::var("HUGGINGFACE_API_TOKEN")
        .expect("HUGGINGFACE_API_TOKEN required");
    
    let driver = HuggingFaceDriver::new(
        api_key,
        "microsoft/DialoGPT-small".to_string(), // Small, fast model
    )?;
    
    let request = GenerateRequest::builder()
        .messages(vec![
            Message::builder()
                .role(Role::User)
                .content(vec![Input::Text("Hello".to_string())])
                .build()?
        ])
        .max_tokens(10) // Minimal tokens to save credits
        .build()?;
    
    let response = driver.generate(request).await?;
    assert!(!response.outputs().is_empty());
    
    Ok(())
}
```

**Recommended test models (free tier friendly):**
- `meta-llama/Llama-3.2-1B-Instruct` - Small chat model (tested, works)
- `meta-llama/Llama-3.2-3B-Instruct` - Slightly larger
- Other Instruct/Chat models on HuggingFace router

**Models that DON'T work:**
- Base models like `gpt2`, `distilgpt2` - not chat-capable
- Most completion-only models - router requires chat format

### Phase 8: Documentation

**Tasks:**
1. Update README with HuggingFace support
2. Add configuration examples
3. Document free tier limitations
4. Add troubleshooting guide

**Files:**
- `README.md`
- `HUGGINGFACE.md` (detailed guide)

**Documentation topics:**
- Getting API token from HuggingFace
- Selecting appropriate models
- Rate limiting and credit management
- Error handling
- Best practices for free tier

### Phase 9: Facade Integration

**Tasks:**
1. Re-export HuggingFace types from `botticelli` crate
2. Update examples
3. Verify all feature combinations work

**Files:**
- `crates/botticelli/src/lib.rs`
- `examples/`

## Feature Gate Structure

```toml
# Workspace features
models = []  # Generic models support
huggingface = ["models", "dep:huggingface_inference_rs"]
local = ["gemini", "ollama", "anthropic", "huggingface"]
```

## API Rate Limiting Considerations

- Free tier: $0.10/month ≈ ~10,000 tokens depending on model
- Use smallest models for testing
- Implement request caching where possible
- Consider rate limit backoff strategies
- Document token costs for common models

## Future Enhancements

1. **Dedicated Endpoints**: Support paid Inference Endpoints for production
2. **Model Hub Integration**: Use `hf-hub` for local model downloads
3. **Specialized Tasks**: Support more HF tasks beyond text generation
4. **Caching**: Implement response caching to reduce API calls
5. **Batch Processing**: Support batch inference for efficiency

## Success Criteria

- ✅ All tests pass with `just check-all`
- ✅ Feature gates work with `just check-features`
- ✅ API tests pass with valid token
- ✅ Zero clippy warnings
- ✅ All public APIs documented
- ✅ Free tier usage documented
- ✅ Examples demonstrate usage

## Notes

- HuggingFace has thousands of models; start with well-tested ones
- Some models may not support all parameters
- API responses vary by model and task type
- Consider model-specific parameter tuning
- Free tier best for experimentation, not production
