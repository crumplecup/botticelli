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

### Coming Soon

- Anthropic Claude (`anthropic` feature)
- OpenAI GPT (`openai` feature)
- HuggingFace models (`huggingface` feature)

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
