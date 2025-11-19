# Local Inference Integration Plan

## Overview

This document outlines the plan to integrate [mistral.rs](https://github.com/EricLBuehler/mistral.rs) as a local inference option alongside Google's Gemini API in the Botticelli project. Mistral.rs is a fast, cross-platform inference server written in Rust that provides an OpenAI-compatible HTTP API, making it ideal for local model hosting.

## Why Mistral.rs?

**Advantages:**

- **OpenAI-compatible API** - Drop-in replacement for existing OpenAI-style clients
- **Written in Rust** - Native performance, potential for tight integration
- **Quantization support** - Run larger models efficiently (2-8 bit quantization, GGUF/GGML)
- **Hardware flexibility** - CPU, NVIDIA CUDA, Apple Metal support
- **Multimodal** - Text, vision, audio, image generation support
- **No API costs** - Run models locally without per-token charges
- **Privacy** - All inference happens locally, no data leaves your machine

**Use Cases:**

- Development and testing without consuming API quotas
- Production deployments where data privacy is critical
- Cost optimization for high-volume inference
- Offline operation when internet connectivity is limited

## Architecture Overview

### Current State

```
botticelli_interface (traits)
         ↓
botticelli_gemini (Gemini API implementation)
```

### Proposed State

```
botticelli_interface (traits)
         ↓                    ↓
botticelli_gemini      botticelli_server
(Gemini API)           (Local inference via mistral.rs)
```

## Implementation Plan

### Phase 1: New Crate - botticelli_server ✅

**Status**: Complete

- Created `crates/botticelli_server/` crate structure
- Added to workspace `Cargo.toml`
- Basic dependencies configured (reqwest, tokio, serde, tracing)

Create a new workspace crate that mirrors `botticelli_gemini`'s structure:

**Crate structure:**

```
crates/botticelli_server/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Module declarations and crate-level exports
│   ├── client.rs        # HTTP client for mistral.rs server
│   ├── request.rs       # Request building and conversion
│   ├── response.rs      # Response parsing and conversion
│   ├── streaming.rs     # Streaming response handling (SSE)
│   ├── config.rs        # Configuration (endpoint URL, model selection)
│   └── error.rs         # Error types following CLAUDE.md patterns
```

**Dependencies:**

```toml
[dependencies]
botticelli_core = { workspace = true }
botticelli_interface = { workspace = true }
botticelli_error = { workspace = true }

reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
futures = "0.3"
```

### Phase 2: Core Components ✅

**Status**: Complete

- Created `config.rs` with `ServerConfig` type
- Created `request.rs` with OpenAI-compatible request types
- Created `response.rs` with OpenAI-compatible response types
- Created `client.rs` with `ServerClient` for HTTP communication
- Implemented streaming support with SSE parsing
- Updated `error.rs` with HTTP, API, deserialization, stream, and configuration errors
- All modules compile successfully

#### 2.1 Configuration (`config.rs`)

```rust
/// Configuration for local inference server connection
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Base URL of the server (e.g., "http://localhost:8080")
    pub base_url: String,
    /// Model identifier to use for inference
    pub model: String,
    /// Optional API key (mistral.rs doesn't require one by default)
    pub api_key: Option<String>,
}

impl ServerConfig {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
        }
    }

    /// Create config from environment variables
    pub fn from_env() -> Result<Self, ServerError> {
        let base_url = std::env::var("INFERENCE_SERVER_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let model = std::env::var("INFERENCE_SERVER_MODEL")
            .map_err(|_| ServerError::new(
                ServerErrorKind::Configuration("INFERENCE_SERVER_MODEL not set".into())
            ))?;
        Ok(Self::new(base_url, model))
    }
}
```

#### 2.2 Client (`client.rs`)

```rust
/// Client for interacting with local inference server
pub struct ServerClient {
    config: ServerConfig,
    client: reqwest::Client,
}

impl ServerClient {
    #[instrument(skip(config))]
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    #[instrument(skip(self))]
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ServerError> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let mut req = self.client
            .post(&url)
            .json(&request)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await
            .map_err(|e| ServerError::new(
                ServerErrorKind::Http(format!("Request failed: {}", e))
            ))?;

        if !response.status().is_success() {
            return Err(ServerError::new(
                ServerErrorKind::Api(format!("Server returned: {}", response.status()))
            ));
        }

        response.json().await
            .map_err(|e| ServerError::new(
                ServerErrorKind::Deserialization(format!("Failed to parse response: {}", e))
            ))
    }

    #[instrument(skip(self))]
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk, ServerError>>, ServerError> {
        // Streaming implementation using SSE parsing
        // Similar to botticelli_gemini's streaming approach
    }
}
```

#### 2.3 Request/Response Types (`request.rs`, `response.rs`)

Implement OpenAI-compatible types that match mistral.rs's API:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

// ... additional types following OpenAI spec
```

#### 2.4 Interface Implementation

Implement `botticelli_interface` traits for mistral.rs:

```rust
use botticelli_interface::{GenerateRequest, GenerateResponse, GenerativeModel};

impl GenerativeModel for ServerClient {
    #[instrument(skip(self))]
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, BotticelliError> {
        // Convert GenerateRequest to ChatCompletionRequest
        let chat_request = convert_request(request)?;

        // Call mistral.rs API
        let response = self.chat_completion(chat_request).await?;

        // Convert response back to GenerateResponse
        convert_response(response)
    }

    #[instrument(skip(self))]
    async fn generate_stream(
        &self,
        request: GenerateRequest
    ) -> Result<impl Stream<Item = Result<GenerateResponse, BotticelliError>>, BotticelliError> {
        // Convert and stream
        let chat_request = convert_request(request)?;
        let stream = self.chat_completion_stream(chat_request).await?;

        Ok(stream.map(|chunk| chunk.and_then(convert_chunk)))
    }
}
```

### Phase 3: Interface Implementation ✅

**Status**: Complete

Implemented `botticelli_interface` traits (`BotticelliDriver` and `Streaming`) to integrate with the rest of Botticelli.

#### 3.1 Dependencies Added

- `botticelli_interface` - For trait definitions
- `botticelli_core` - For request/response types
- `async-trait` - For async trait support

#### 3.2 Conversion Module (`convert.rs`)

Handles conversion between Botticelli types and OpenAI-compatible server types:

- `to_chat_request()` - Converts `GenerateRequest` to `ChatCompletionRequest`
- `from_chat_response()` - Converts `ChatCompletionResponse` to `GenerateResponse`
- `chunk_to_stream_chunk()` - Converts streaming chunks
- Handles multimodal inputs by extracting text (skips media)
- Maps finish reasons between formats

#### 3.3 Trait Implementations

Implemented both core traits in `client.rs`:

- `BotticelliDriver` - Basic generation support
- `Streaming` - Streaming response support

The `ServerClient` now integrates seamlessly with the Botticelli ecosystem.

### Phase 4: Server Management

#### 4.1 Optional: Embedded Server Control

For advanced use cases, consider adding server lifecycle management:

```rust
/// Manages an inference server process
pub struct InferenceServer {
    process: Option<Child>,
    config: ServerConfig,
}

impl InferenceServer {
    /// Start a new inference server process
    pub async fn start(config: ServerConfig) -> Result<Self, ServerError> {
        // Spawn mistralrs server binary
        // Wait for health check endpoint
        // Return server handle
    }

    /// Stop the server gracefully
    pub async fn stop(&mut self) -> Result<(), ServerError> {
        // Send shutdown signal
        // Wait for process exit
    }
}
```

**Note:** This is optional - most users can run the inference server independently.

### Phase 4: Testing Strategy

Following CLAUDE.md testing guidelines:

#### 4.1 Local Tests (No `api` feature required)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = ServerConfig::new("http://localhost:8080", "phi-3.5");
        assert_eq!(config.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_request_serialization() {
        let request = ChatCompletionRequest {
            model: "test-model".into(),
            messages: vec![
                Message {
                    role: "user".into(),
                    content: "Hello".into(),
                }
            ],
            max_tokens: Some(100),
            temperature: None,
            top_p: None,
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"test-model\""));
    }
}
```

#### 4.2 API Tests (Require `api` feature + running server)

```rust
#[cfg(all(test, feature = "api"))]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_completion() {
        // Requires INFERENCE_SERVER_BASE_URL and INFERENCE_SERVER_MODEL env vars
        let config = ServerConfig::from_env().unwrap();
        let client = ServerClient::new(config);

        let request = ChatCompletionRequest {
            model: client.config.model.clone(),
            messages: vec![
                Message {
                    role: "user".into(),
                    content: "Say 'test'".into(),
                }
            ],
            max_tokens: Some(10),
            temperature: Some(0.1),
            top_p: None,
            stream: None,
        };

        let response = client.chat_completion(request).await.unwrap();
        assert!(!response.choices.is_empty());
    }

    #[tokio::test]
    async fn test_streaming() {
        let config = ServerConfig::from_env().unwrap();
        let client = ServerClient::new(config);

        // Test streaming with minimal tokens
        let request = ChatCompletionRequest {
            model: client.config.model.clone(),
            messages: vec![
                Message {
                    role: "user".into(),
                    content: "Count to 3".into(),
                }
            ],
            max_tokens: Some(20),
            temperature: Some(0.1),
            top_p: None,
            stream: Some(true),
        };

        let mut stream = client.chat_completion_stream(request).await.unwrap();
        let mut chunk_count = 0;

        while let Some(chunk) = stream.next().await {
            chunk.unwrap();
            chunk_count += 1;
        }

        assert!(chunk_count > 0);
    }
}
```

### Phase 5: Documentation

#### 5.1 Crate Documentation

Add comprehensive documentation to `lib.rs`:

````rust
//! Local inference server integration for Botticelli
//!
//! This crate provides a client for interacting with local inference servers (like mistral.rs),
//! enabling fast local model inference for large language models.
//!
//! # Features
//!
//! - OpenAI-compatible API client
//! - Streaming and non-streaming inference
//! - Implements `botticelli_interface::GenerativeModel` trait
//! - Full observability with tracing instrumentation
//!
//! # Setup
//!
//! 1. Install an inference server (e.g., mistral.rs - see upstream docs)
//! 2. Start the server: `./mistralrs_server --port 8080`
//! 3. Set environment variables:
//!    - `INFERENCE_SERVER_BASE_URL` (default: "http://localhost:8080")
//!    - `INFERENCE_SERVER_MODEL` (required, e.g., "microsoft/Phi-3.5-mini-instruct")
//!
//! # Example
//!
//! ```rust,no_run
//! use botticelli_server::{ServerClient, ServerConfig};
//! use botticelli_interface::{GenerativeModel, GenerateRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::from_env()?;
//!     let client = ServerClient::new(config);
//!
//!     let request = GenerateRequest {
//!         prompt: "Explain Rust ownership".into(),
//!         max_tokens: Some(100),
//!         temperature: Some(0.7),
//!         ..Default::default()
//!     };
//!
//!     let response = client.generate(request).await?;
//!     println!("{}", response.text);
//!
//!     Ok(())
//! }
//! ```
````

#### 5.2 Setup Guide

Create `LOCAL_INFERENCE_SETUP.md` in the crate:

````markdown
# Local Inference Server Setup Guide

## Installation (using mistral.rs)

### Option 1: Prebuilt Binaries

Download from [releases page](https://github.com/EricLBuehler/mistral.rs/releases)

### Option 2: Build from Source

```bash
git clone https://github.com/EricLBuehler/mistral.rs
cd mistral.rs
cargo build --release
```
````

### Option 3: Docker

```bash
docker pull ghcr.io/ericbuehler/mistral.rs:latest
docker run -p 8080:8080 mistral.rs
```

## Model Selection

Recommended models for Botticelli:

- **Phi-3.5-mini-instruct** - Fast, efficient (3.8B params)
- **Mistral-7B-Instruct** - Balanced performance (7B params)
- **Llama-3.1-8B-Instruct** - High quality (8B params)

## Running the Server

Basic command:

```bash
./mistralrs_server \
    --port 8080 \
    run \
    --model-id "microsoft/Phi-3.5-mini-instruct"
```

With quantization (recommended for lower memory):

```bash
./mistralrs_server \
    --port 8080 \
    run \
    --model-id "microsoft/Phi-3.5-mini-instruct" \
    --isq "Q4K"
```

## Environment Configuration

```bash
export INFERENCE_SERVER_BASE_URL="http://localhost:8080"
export INFERENCE_SERVER_MODEL="microsoft/Phi-3.5-mini-instruct"
```

## Testing

Run local tests (no server required):

```bash
just test-local
```

Run API tests (requires running server):

```bash
just test-api
```

````

### Phase 6: Integration with Main Binary

Update the main `botticelli` binary to support model selection:

```rust
// In main.rs or config
pub enum ModelProvider {
    Gemini(GeminiConfig),
    LocalServer(ServerConfig),
}

impl ModelProvider {
    pub fn create_client(&self) -> Box<dyn GenerativeModel> {
        match self {
            ModelProvider::Gemini(config) => {
                Box::new(GeminiClient::new(config.clone()))
            }
            ModelProvider::LocalServer(config) => {
                Box::new(ServerClient::new(config.clone()))
            }
        }
    }
}
````

Configuration example:

```toml
# botticelli.toml
[model]
provider = "local"  # or "gemini"

[model.local]
base_url = "http://localhost:8080"
model = "microsoft/Phi-3.5-mini-instruct"

[model.gemini]
api_key = "..."
model = "gemini-1.5-flash"
```

## Benefits

1. **Cost Reduction**: No API charges for local inference
2. **Privacy**: All data stays local
3. **Speed**: No network latency for local models
4. **Flexibility**: Easy to switch between providers
5. **Development**: Test without consuming API quotas

## Considerations

1. **Resource Requirements**: Local inference requires GPU/CPU resources
2. **Model Selection**: Smaller models may have lower quality than Gemini
3. **Setup Complexity**: Users must install and configure mistral.rs
4. **Feature Parity**: Some Gemini-specific features may not be available

## Future Enhancements

1. **Automatic Model Management**: Download and cache models automatically
2. **Server Lifecycle**: Start/stop mistral.rs from Botticelli
3. **Model Benchmarking**: Compare performance across providers
4. **Hybrid Mode**: Use local for cheap operations, API for complex ones
5. **Fine-tuning Support**: Load custom LoRA adapters

## Timeline

- **Week 1**: Create crate structure, implement core client
- **Week 2**: Add streaming support, error handling
- **Week 3**: Implement interface traits, add tests
- **Week 4**: Documentation, integration with main binary
- **Week 5**: Testing, refinement, examples

## Success Criteria

- [ ] All `botticelli_interface` traits implemented
- [ ] Streaming and non-streaming modes working
- [ ] Test coverage >80% (local tests)
- [ ] API tests validate against running server
- [ ] Documentation complete with examples
- [ ] Zero clippy warnings
- [ ] All doctests pass
- [ ] Integration with main binary complete
