# botticelli_models

LLM provider implementations for the Botticelli ecosystem.

## Overview

This crate provides implementations of the `BotticelliDriver` trait for various LLM providers. Each provider is feature-gated for flexible dependency management.

## Supported Providers

### Gemini (Google AI)

```toml
[dependencies]
botticelli_models = { version = "0.2", features = ["gemini"] }
```

```rust
use botticelli_models::GeminiClient;

let client = GeminiClient::new(api_key, "gemini-1.5-flash");
let response = client.generate(request).await?;
```

**Features**:
- REST API support
- Live API support (streaming)
- Multimodal inputs (text, images, audio, video, documents)
- System instructions
- Rate limiting integration

### Anthropic Claude

```toml
[dependencies]
botticelli_models = { version = "0.2", features = ["anthropic"] }
```

```rust
use botticelli_models::AnthropicClient;

let client = AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022");
let response = client.generate(request).await?;
```

**Features**:
- Full tool calling support
- Message API
- Streaming support
- Vision (multimodal inputs)
- Rate limiting integration

### Coming Soon

- OpenAI GPT (`openai` feature)
- HuggingFace models (`huggingface` feature)
- Ollama local models (`ollama` feature)

## Tool Calling Support

Tool calling (function calling) allows LLMs to request execution of tools/functions during generation. The framework provides a unified interface across providers via `botticelli_core::ToolDefinition`.

### Status by Provider

| Provider | Status | Notes |
|----------|--------|-------|
| **Anthropic** | ✅ Full Support | Complete tool calling implementation |
| **Gemini** | 🚧 Partial | Types defined, REST API integration pending |
| **Ollama** | ⚠️ Model-Dependent | Varies by model, needs capability detection |
| **OpenAI** | 📋 Planned | Not yet implemented |
| **HuggingFace** | ❌ Not Supported | No plans for tool calling |

### Using Tool Calling

```rust
use botticelli_core::{GenerateRequest, ToolDefinition, Output};
use botticelli_models::AnthropicClient;
use serde_json::json;

// Define a tool
let tool = ToolDefinition::new(
    "get_weather".to_string(),
    "Get weather for a location".to_string(),
    json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "City name"
            }
        },
        "required": ["location"]
    }),
);

// Add tools to request
let request = GenerateRequest::builder()
    .messages(vec![message])
    .tools(Some(vec![tool]))
    .build()?;

// Driver handles provider-specific conversion
let response = client.generate(&request).await?;

// Check for tool calls in response
for output in response.outputs() {
    match output {
        Output::ToolCalls(calls) => {
            for call in calls {
                println!("Tool: {}", call.name());
                println!("Args: {}", call.arguments());
            }
        }
        Output::Text(text) => println!("Text: {}", text),
        _ => {}
    }
}
```

### Anthropic Tool Calling

Anthropic (Claude) has full support:
- Tools defined in `GenerateRequest`
- Automatic conversion to Anthropic format
- Tool calls returned in `Output::ToolCalls`
- Stop reason set to `StopReason::ToolUse`

### Gemini Tool Calling

Gemini Live API has tool types defined in `live_protocol`:
- `Tool` struct for definitions
- `FunctionCall` struct for responses
- REST API integration requires `gemini-rust` update or direct HTTP calls

**Current limitation**: Tool calling not yet connected to `GenerateRequest` interface.

### Implementation Notes

- Each driver converts `botticelli_core::ToolDefinition` to provider-specific format
- Drivers parse tool calls from responses and return as `Output::ToolCalls`
- Tool execution is handled by `botticelli_mcp_client::UnifiedMcpClient`

## Gemini Live API

Real-time streaming conversations:

```rust
use botticelli_models::GeminiLiveClient;

let client = GeminiLiveClient::connect(api_key, model).await?;

// Send messages
client.send_text("Hello!").await?;

// Receive responses
while let Some(chunk) = client.receive().await? {
    match chunk {
        StreamChunk::Text(text) => print!("{}", text),
        StreamChunk::Audio(data) => process_audio(data),
        _ => {}
    }
}
```

## Rate Limiting

All clients integrate with `botticelli_rate_limit`:

```rust
use botticelli_rate_limit::{RateLimiter, GeminiTier};

let client = GeminiClient::new(api_key, model);
let tier = GeminiTier::free();

let limited_client = RateLimiter::new(
    client,
    tier.rpm,
    tier.tpm,
    tier.rpd,
    tier.concurrent,
);
```

## Error Handling

Provider-specific errors with automatic retries:

```rust
use botticelli_models::GeminiError;

match client.generate(request).await {
    Ok(response) => println!("{}", response.text),
    Err(GeminiError { kind, .. }) => match kind {
        GeminiErrorKind::RateLimited => {}, // Auto-retried
        GeminiErrorKind::InvalidApiKey => {}, // Fatal
        _ => {}
    }
}
```

## Dependencies

- `botticelli_interface` - Driver trait
- `botticelli_rate_limit` - Rate limiting
- `botticelli_error` - Error types
- `gemini-rust` (optional) - Gemini SDK
- `reqwest` - HTTP client
- `tokio` - Async runtime

## Version

Current version: 0.2.0
