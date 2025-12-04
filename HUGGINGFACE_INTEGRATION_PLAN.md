# HuggingFace Inference API Integration Plan

## Overview

Integrate HuggingFace Inference API support into Botticelli using the `huggingface_inference_rs` crate as the foundation. HuggingFace provides $0.10/month in free credits for all users, making it excellent for testing and prototyping.

## Selected Crate: huggingface_inference_rs

**Rationale:**
- Dedicated Rust wrapper for HuggingFace Inference API
- Actively maintained and documented
- Supports various NLP tasks (text generation, Q&A, NER, etc.)
- Straightforward API for basic inference tasks

**Alternatives considered:**
- `hf-hub`: Model download/cache only, not inference API
- `hugging-face-client`: Experimental/nightly, less mature
- Custom reqwest implementation: More maintenance burden

## Free Tier Details

- **Credits**: $0.10 USD/month (free), $2.00/month (Pro users)
- **Access**: All public models on HuggingFace Hub
- **Rate Limits**: Shared infrastructure with modest limits
- **Suitable for**: Testing, demos, prototyping, small personal apps

## Implementation Phases

### Phase 1: Feature Gates and Dependencies

**Tasks:**
1. Add `huggingface` feature gate to workspace
2. Update `local` feature to include `huggingface`
3. Add `huggingface_inference_rs` dependency to `botticelli_models`
4. Update all Cargo.toml files with feature flags

**Files:**
- `Cargo.toml` (workspace root)
- `crates/botticelli_models/Cargo.toml`
- `crates/botticelli_error/Cargo.toml`
- `crates/botticelli/Cargo.toml`

### Phase 2: Error Types

**Tasks:**
1. Create `HuggingFaceErrorKind` enum in `botticelli_error`
2. Add API error variants (rate limit, model not found, invalid request)
3. Add conversion errors (request/response mapping)
4. Integrate into `ModelsErrorKind` with `#[from]`

**Files:**
- `crates/botticelli_error/src/models.rs`

**Error variants:**
```rust
#[cfg(feature = "huggingface")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum HuggingFaceErrorKind {
    #[display("API error: {}", _0)]
    Api(String),
    
    #[display("Rate limit exceeded")]
    RateLimit,
    
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),
    
    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),
    
    #[display("Request conversion failed: {}", _0)]
    RequestConversion(String),
    
    #[display("Response conversion failed: {}", _0)]
    ResponseConversion(String),
}
```

### Phase 3: DTOs (Request/Response Types)

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

### Phase 7: Testing

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
- `microsoft/DialoGPT-small` - Conversational, small
- `distilgpt2` - Small GPT-2 variant
- `google/flan-t5-small` - Instruction-following, efficient

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
