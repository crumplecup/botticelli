# Ollama Integration Plan

**Date:** 2025-12-01  
**Goal:** Add Ollama support to Botticelli using `ollama-rs` crate  
**Strategy:** Follow the same pattern as Gemini integration

---

## Overview

Add local LLM support via Ollama, enabling:
- Free development and testing (no API costs)
- Privacy (no data leaves local machine)
- No rate limits
- Offline capability
- Support for Llama 2, Mistral, CodeLlama, DeepSeek-Coder, Phi, etc.

---

## Phase 1: Add Dependencies and Feature Flag

### Changes Required

#### 1. Update `crates/botticelli_models/Cargo.toml`
```toml
[dependencies]
# Existing
gemini-rust = { version = "1.5", optional = true }

# Add Ollama
ollama-rs = { version = "0.3", optional = true }

[features]
default = ["gemini"]
gemini = ["dep:gemini-rust"]
ollama = ["dep:ollama-rs"]  # New feature
```

#### 2. Update workspace features if needed
Check `Cargo.toml` at workspace root for feature propagation.

---

## Phase 2: Create Ollama Module Structure

### File Structure
```
crates/botticelli_models/src/
├── gemini/
│   ├── mod.rs
│   ├── client.rs
│   ├── live_client.rs
│   └── ... (existing files)
├── ollama/              # New module
│   ├── mod.rs           # Module exports
│   ├── client.rs        # OllamaClient implementation
│   ├── error.rs         # OllamaError types
│   └── conversion.rs    # Convert between Ollama and Botticelli types
└── lib.rs               # Re-export OllamaClient
```

### Create Files

#### `crates/botticelli_models/src/ollama/mod.rs`
```rust
//! Ollama LLM client implementation.

mod client;
mod error;
mod conversion;

pub use client::OllamaClient;
pub use error::{OllamaError, OllamaErrorKind, OllamaResult};
```

---

## Phase 3: Implement Error Types

Following Gemini's error pattern with `derive_more`.

### `crates/botticelli_models/src/ollama/error.rs`
```rust
use derive_more::{Display, Error};

/// Ollama-specific error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum OllamaErrorKind {
    #[display("Ollama server not running at {}", _0)]
    ServerNotRunning(String),
    
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),
    
    #[display("Failed to pull model: {}", _0)]
    ModelPullFailed(String),
    
    #[display("API error: {}", _0)]
    ApiError(String),
    
    #[display("Invalid configuration: {}", _0)]
    InvalidConfiguration(String),
}

/// Ollama error with location tracking.
#[derive(Debug, Clone, Display, Error)]
#[display("Ollama Error: {} at {}:{}", kind, file, line)]
pub struct OllamaError {
    pub kind: OllamaErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl OllamaError {
    #[track_caller]
    pub fn new(kind: OllamaErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

pub type OllamaResult<T> = Result<T, OllamaError>;
```

---

## Phase 4: Implement Type Conversions

### `crates/botticelli_models/src/ollama/conversion.rs`
```rust
//! Type conversions between Ollama and Botticelli types.

use crate::{Message, Role, Input, Output, GenerateRequest, GenerateResponse};
use ollama_rs::generation::completion::GenerationResponse;

/// Convert Botticelli messages to Ollama prompt.
pub fn messages_to_prompt(messages: &[Message]) -> String {
    let mut prompt = String::new();
    
    for msg in messages {
        let role_prefix = match msg.role() {
            Role::User => "User: ",
            Role::Model => "Assistant: ",
            Role::System => "System: ",
        };
        
        prompt.push_str(role_prefix);
        
        for input in msg.content() {
            match input {
                Input::Text(text) => {
                    prompt.push_str(text);
                    prompt.push('\n');
                }
                Input::Image(_) => {
                    // Ollama supports vision models, but handle separately
                    prompt.push_str("[Image content]\n");
                }
            }
        }
        
        prompt.push('\n');
    }
    
    prompt
}

/// Convert Ollama response to Botticelli response.
pub fn response_to_generate_response(
    response: GenerationResponse,
) -> GenerateResponse {
    GenerateResponse::builder()
        .outputs(vec![Output::Text(response.response)])
        .build()
        .expect("Valid response")
}
```

---

## Phase 5: Implement OllamaClient

### `crates/botticelli_models/src/ollama/client.rs`

Key features to implement:
1. **No rate limiting needed** (local execution)
2. **Connection validation** (check Ollama is running)
3. **Model management** (check model exists, pull if needed)
4. **Streaming support** (optional, like Gemini)
5. **Implement Driver trait** (compatibility with existing code)

```rust
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest as OllamaRequest;
use crate::{Driver, GenerateRequest, GenerateResponse};
use super::{OllamaError, OllamaErrorKind, OllamaResult};
use super::conversion::{messages_to_prompt, response_to_generate_response};
use tracing::{debug, info, instrument, warn};

/// Ollama LLM client for local model execution.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    /// Ollama client instance
    client: Ollama,
    
    /// Model name (e.g., "llama2", "mistral", "codellama")
    model_name: String,
    
    /// Ollama server URL
    base_url: String,
}

impl OllamaClient {
    /// Create a new Ollama client with default localhost connection.
    #[instrument(name = "ollama_client_new")]
    pub fn new(model_name: impl Into<String>) -> OllamaResult<Self> {
        Self::new_with_url(model_name, "http://localhost:11434")
    }
    
    /// Create a new Ollama client with custom server URL.
    #[instrument(name = "ollama_client_new_with_url")]
    pub fn new_with_url(
        model_name: impl Into<String>,
        base_url: impl Into<String>,
    ) -> OllamaResult<Self> {
        let model_name = model_name.into();
        let base_url = base_url.into();
        
        info!(
            model = %model_name,
            url = %base_url,
            "Creating Ollama client"
        );
        
        let client = Ollama::new(
            base_url.clone(),
            11434, // Default Ollama port
        );
        
        Ok(Self {
            client,
            model_name,
            base_url,
        })
    }
    
    /// Check if Ollama server is running and model is available.
    #[instrument(skip(self))]
    pub async fn validate(&self) -> OllamaResult<()> {
        debug!("Validating Ollama server and model availability");
        
        // Check if server is reachable
        match self.client.list_local_models().await {
            Ok(models) => {
                debug!(count = models.len(), "Found local models");
                
                // Check if our model exists
                let model_exists = models.iter()
                    .any(|m| m.name == self.model_name);
                
                if !model_exists {
                    warn!(
                        model = %self.model_name,
                        available = ?models.iter().map(|m| &m.name).collect::<Vec<_>>(),
                        "Model not found locally"
                    );
                    
                    return Err(OllamaError::new(
                        OllamaErrorKind::ModelNotFound(self.model_name.clone())
                    ));
                }
                
                info!("Ollama server and model validated");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "Failed to connect to Ollama server");
                Err(OllamaError::new(
                    OllamaErrorKind::ServerNotRunning(self.base_url.clone())
                ))
            }
        }
    }
    
    /// Pull model if not available locally.
    #[instrument(skip(self))]
    pub async fn ensure_model(&self) -> OllamaResult<()> {
        debug!("Ensuring model is available");
        
        match self.validate().await {
            Ok(()) => {
                debug!("Model already available");
                Ok(())
            }
            Err(_) => {
                info!(model = %self.model_name, "Pulling model");
                
                self.client
                    .pull_model(self.model_name.clone(), false)
                    .await
                    .map_err(|e| {
                        OllamaError::new(
                            OllamaErrorKind::ModelPullFailed(e.to_string())
                        )
                    })?;
                
                info!("Model pulled successfully");
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl Driver for OllamaClient {
    #[instrument(skip(self, request))]
    async fn generate(
        &self,
        request: GenerateRequest,
    ) -> BotticelliResult<GenerateResponse> {
        debug!("Generating with Ollama");
        
        // Convert messages to prompt
        let prompt = messages_to_prompt(request.messages());
        
        debug!(
            prompt_length = prompt.len(),
            "Converted messages to prompt"
        );
        
        // Create Ollama request
        let ollama_req = OllamaRequest::new(
            self.model_name.clone(),
            prompt,
        );
        
        // Execute generation (no rate limiting needed for local)
        let response = self.client
            .generate(ollama_req)
            .await
            .map_err(|e| {
                OllamaError::new(
                    OllamaErrorKind::ApiError(e.to_string())
                )
            })?;
        
        debug!(
            response_length = response.response.len(),
            "Received response from Ollama"
        );
        
        Ok(response_to_generate_response(response).into())
    }
}
```

---

## Phase 6: Update Library Exports

### `crates/botticelli_models/src/lib.rs`
```rust
// Existing
#[cfg(feature = "gemini")]
pub mod gemini;

// Add Ollama
#[cfg(feature = "ollama")]
pub mod ollama;

// Re-exports
#[cfg(feature = "gemini")]
pub use gemini::GeminiClient;

#[cfg(feature = "ollama")]
pub use ollama::OllamaClient;
```

---

## Phase 7: Add Configuration Support

### Update `botticelli.toml` to support Ollama tiers
```toml
[providers.ollama]
default_tier = "local"

[providers.ollama.tiers.local]
name = "Local"
# No rate limits for local execution
rpm = null
tpm = null
rpd = null
max_concurrent = 4  # Limit concurrent requests to avoid overwhelming local GPU

[providers.ollama.models]
# Model-specific settings if needed
llama2 = { max_concurrent = 2 }
mistral = { max_concurrent = 4 }
```

---

## Phase 8: Add Tests

### `crates/botticelli_models/tests/ollama_client_test.rs`
```rust
#[cfg(feature = "ollama")]
#[tokio::test]
#[ignore] // Requires Ollama running locally
async fn test_ollama_basic_generation() {
    use botticelli_models::{OllamaClient, Driver, Message, Role, Input};
    
    let client = OllamaClient::new("llama2")
        .expect("Failed to create client");
    
    // Validate server and model
    client.validate().await
        .expect("Ollama server not available");
    
    let messages = vec![
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Say hello".to_string())])
            .build()
            .expect("Valid message")
    ];
    
    let request = GenerateRequest::builder()
        .messages(messages)
        .build()
        .expect("Valid request");
    
    let response = client.generate(request).await
        .expect("Generation failed");
    
    assert!(!response.outputs().is_empty());
}
```

---

## Phase 9: Update Documentation

### Create `crates/botticelli_models/src/ollama/README.md`
Document:
- How to install Ollama
- How to pull models
- Supported models
- Example usage
- Troubleshooting

### Update `GEMINI.md` → Rename to `LLM_PROVIDERS.md`
Add Ollama section with:
- Installation instructions
- Model recommendations
- Configuration examples

---

## Phase 10: Integration with Narrative System

### Update narrative TOML to support Ollama
```toml
[narrative]
name = "test_ollama"

[narrative.llm]
provider = "ollama"  # New option (alongside "gemini")
model = "llama2"

[narrative.steps.generate]
processor = "ContentGenerationProcessor"
# ... rest of config
```

### Update narrative executor to handle Ollama
- Recognize `provider = "ollama"` in TOML
- Instantiate `OllamaClient` instead of `GeminiClient`
- Pass through same `Driver` trait interface

---

## Testing Strategy

### Manual Testing Checklist
- [ ] Install Ollama: `curl https://ollama.ai/install.sh | sh`
- [ ] Pull test model: `ollama pull llama2`
- [ ] Verify Ollama running: `ollama list`
- [ ] Run unit tests with `cargo test --features ollama`
- [ ] Test via narrative with simple prompt
- [ ] Test error handling (stop Ollama, verify error)
- [ ] Test model auto-pull functionality
- [ ] Compare output quality vs Gemini

### Integration Testing
- [ ] Run actor-server with Ollama-based narratives
- [ ] Verify tracing works with Ollama
- [ ] Test concurrent requests (max_concurrent)
- [ ] Performance comparison with Gemini

---

## Success Criteria

1. ✅ `OllamaClient` implements `Driver` trait
2. ✅ Feature flag `ollama` builds and tests pass
3. ✅ Can run narratives with `provider = "ollama"`
4. ✅ Error handling for server not running
5. ✅ Error handling for model not found
6. ✅ Tracing instrumentation complete
7. ✅ Documentation updated
8. ✅ Zero clippy warnings
9. ✅ Zero test failures

---

## Benefits After Implementation

### For Development
- **No API costs** during development
- **No rate limits** (only local compute limits)
- **Fast iteration** (no network latency)
- **Offline capable** (work without internet)

### For Production
- **Cost savings** for high-volume use cases
- **Privacy** (no data sent to third parties)
- **Compliance** (data never leaves infrastructure)
- **Predictable performance** (no API fluctuations)

### For Users
- **Choice** of local vs cloud LLMs
- **Experimentation** with different models
- **Learning** with open models
- **Testing** without burning API credits

---

## Rollout Plan

### Week 1: Core Implementation
- Days 1-2: Dependencies, structure, error types
- Days 3-4: Client implementation and conversions
- Day 5: Basic tests and validation

### Week 2: Integration & Testing
- Days 1-2: Configuration and narrative integration
- Days 3-4: Comprehensive testing
- Day 5: Documentation and cleanup

### Week 3: Polish & Deploy
- Days 1-2: Performance optimization
- Days 3-4: Additional models and features
- Day 5: Merge to main, announce feature

---

## Future Enhancements

### Phase 2 (Later)
- Streaming support (like Gemini's streaming)
- Vision model support (models with image inputs)
- Embeddings support (for RAG systems)
- Model temperature/parameter configuration
- Multi-modal support (images, audio)
- Automatic model recommendation based on task

### Phase 3 (Advanced)
- Model quantization options (4-bit, 8-bit)
- Custom model fine-tuning support
- Batch processing optimization
- Model caching strategies
- Performance benchmarking tools

---

## Risk Mitigation

### Risk: Ollama not installed
**Mitigation:** Clear error messages, documentation, optional feature flag

### Risk: Model takes too long to pull
**Mitigation:** Progress indicators, async pull, cache pre-pulled models

### Risk: Local hardware insufficient
**Mitigation:** Recommend models by hardware tier, fallback to cloud

### Risk: Different output quality vs Gemini
**Mitigation:** Document model capabilities, provide comparison guide

---

*Last Updated: 2025-12-01*  
*Status: Ready for Implementation*  
*Estimated Effort: 2-3 weeks*  
*Priority: High (enables cost-free development)*
