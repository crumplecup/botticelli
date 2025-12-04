# Groq AI LPU Integration Guide

## Overview

Botticelli provides support for Groq AI's ultra-fast LPU (Language Processing Unit) inference through their OpenAI-compatible API. Groq offers speeds up to 10x faster than traditional GPU-based inference with consistently low latency.

## Quick Start

```rust
use botticelli::{GroqDriver, BotticelliDriver, GenerateRequest, Message, Role, Input};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    
    let driver = GroqDriver::new(
        "llama-3.1-8b-instant".to_string()
    )?;
    
    let message = Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Hello!".to_string())])
        .build()?;
    
    let request = GenerateRequest::builder()
        .messages(vec![message])
        .max_tokens(Some(100))
        .temperature(Some(0.7))
        .build()?;
    
    let response = driver.generate(&request).await?;
    
    for output in response.outputs() {
        println!("Response: {:?}", output);
    }
    
    Ok(())
}
```

## Setup

### 1. Enable the Feature

Add to your `Cargo.toml`:

```toml
[dependencies]
botticelli = { version = "0.2", features = ["groq"] }
```

### 2. Get an API Key

1. Sign up at [console.groq.com](https://console.groq.com)
2. Navigate to API Keys section
3. Create a new API key
4. Copy your key (starts with `gsk_`)

### 3. Set Environment Variable

```bash
export GROQ_API_KEY="gsk_your_key_here"
```

Or in `.env`:

```
GROQ_API_KEY=gsk_your_key_here
```

## API Details

### Endpoint

Groq uses an OpenAI-compatible endpoint:

```
https://api.groq.com/openai/v1/chat/completions
```

### Format

Requests use the OpenAI chat completions format:

```json
{
  "model": "llama-3.1-8b-instant",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "max_tokens": 100,
  "temperature": 0.7
}
```

Responses follow OpenAI format:

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      }
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 8,
    "total_tokens": 18
  }
}
```

## Model Selection

### ⚡ Recommended Models

**Llama 3.1 Family (Fastest):**
- `llama-3.1-8b-instant` - 8B params, ultra-fast, recommended for most use cases
- `llama-3.1-70b-versatile` - 70B params, high quality, still very fast
- `llama-3.2-1b-preview` - 1B params, smallest/fastest for simple tasks
- `llama-3.2-3b-preview` - 3B params, good balance
- `llama-3.2-90b-text-preview` - 90B params, highest quality

**Llama 3.3:**
- `llama-3.3-70b-versatile` - Latest 70B model, best quality/speed ratio

**Mixtral:**
- `mixtral-8x7b-32768` - 8x7B MoE, excellent quality, 32K context
- `mixtral-8x22b-32768` - 8x22B MoE, highest quality (slower)

**Gemma:**
- `gemma2-9b-it` - Google's 9B instruction-tuned model
- `gemma-7b-it` - 7B variant

### Context Windows

- Most models: 8K tokens
- Mixtral: 32K tokens  
- Llama 3.1: Up to 128K tokens (selected variants)
- Preview models: Varies (check documentation)

### Speed Characteristics

Groq's LPU architecture delivers:
- **300+ tokens/second** for 8B models
- **100+ tokens/second** for 70B models
- **Consistent low latency** (50-200ms time to first token)
- **No cold starts** - always ready

## Parameters

The driver supports standard parameters:

```rust
let request = GenerateRequest::builder()
    .messages(vec![message])
    .max_tokens(Some(100))      // Maximum tokens to generate
    .temperature(Some(0.7))     // Sampling temperature (0.0-1.0)
    .build()?;
```

**Available Parameters:**
- `max_tokens` - Maximum tokens in completion (default: model-specific)
- `temperature` - Randomness (0.0 = deterministic, 1.0 = creative)
- `top_p` - Nucleus sampling (not currently exposed in GenerateRequest)

## Rate Limits and Pricing

### Free Tier

Groq offers a generous free tier:
- **Requests**: 30 requests/minute
- **Tokens**: 7,000 requests/day
- **Speed**: Full LPU speed, no throttling
- **Models**: Access to all models

### Paid Tier

For higher usage:
- **Higher rate limits** per model
- **Priority access** during peak times
- **Competitive pricing** per token
- **Speed advantage** reduces compute costs

### Best Practices

1. **Choose appropriate models**: 8B for speed, 70B for quality
2. **Set reasonable max_tokens**: Avoid generating more than needed
3. **Use streaming**: Get results faster (when implemented)
4. **Cache responses**: Reuse identical requests
5. **Batch when possible**: Group related requests

## Error Handling

```rust
use botticelli_error::{BotticelliError, GroqErrorKind};

match driver.generate(&request).await {
    Ok(response) => {
        // Handle success
    }
    Err(e) => {
        if let BotticelliError::Models(models_err) = &e {
            match &models_err.kind {
                botticelli_error::ModelsErrorKind::Groq(groq_err) => {
                    match groq_err {
                        GroqErrorKind::Api(msg) => {
                            eprintln!("API error: {}", msg);
                        }
                        GroqErrorKind::RateLimit => {
                            eprintln!("Rate limit exceeded - wait before retrying");
                        }
                        GroqErrorKind::ModelNotFound(model) => {
                            eprintln!("Model not found: {}", model);
                        }
                        _ => eprintln!("Other error: {}", e),
                    }
                }
                _ => eprintln!("Error: {}", e),
            }
        }
    }
}
```

## Common Issues

### "Invalid API Key"

**Cause:** Missing, incorrect, or expired API key.

**Solution:**
- Check `GROQ_API_KEY` is set correctly
- Verify key starts with `gsk_`
- Generate new key if needed from console.groq.com

### "Model not found"

**Cause:** Model name is incorrect or not available.

**Solution:**
- Check model name spelling
- Verify model is available on Groq
- Use recommended models from this guide

### Rate Limit Exceeded

**Cause:** Exceeded free tier limits (30 req/min or 7000 req/day).

**Solution:**
- Wait 60 seconds and retry
- Implement exponential backoff
- Upgrade to paid tier for higher limits
- Reduce request frequency

### Timeout Errors

**Cause:** Network issues or unusually long generation.

**Solution:**
- Reduce `max_tokens` for faster responses
- Check network connectivity
- Retry with exponential backoff

## Streaming

**Status:** Not yet implemented

Streaming support is planned but currently falls back to non-streaming. The response will be returned as a single chunk.

```rust
// Currently returns single chunk
let stream = driver.generate_stream(&request).await?;
```

**Future:** Will support Server-Sent Events (SSE) streaming for token-by-token responses.

## Examples

### Multi-Turn Conversation

```rust
let messages = vec![
    Message::builder()
        .role(Role::System)
        .content(vec![Input::Text("You are a helpful coding assistant.".to_string())])
        .build()?,
    Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Explain Rust lifetimes.".to_string())])
        .build()?,
    Message::builder()
        .role(Role::Assistant)
        .content(vec![Input::Text("Lifetimes in Rust ensure references are valid...".to_string())])
        .build()?,
    Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Give me an example.".to_string())])
        .build()?,
];

let request = GenerateRequest::builder()
    .messages(messages)
    .max_tokens(Some(300))
    .build()?;

let response = driver.generate(&request).await?;
```

### With Custom API Key

```rust
let driver = GroqDriver::with_api_key(
    "gsk_your_key_here".to_string(),
    "llama-3.1-8b-instant".to_string()
)?;
```

### Fast Response Generation

```rust
// Use the fastest model with minimal tokens for quick responses
let driver = GroqDriver::new("llama-3.2-1b-preview".to_string())?;

let request = GenerateRequest::builder()
    .messages(vec![
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Yes or no: Is Rust memory safe?".to_string())])
            .build()?
    ])
    .max_tokens(Some(5))  // Just need a short answer
    .temperature(Some(0.0))  // Deterministic
    .build()?;

let response = driver.generate(&request).await?;
// Typically completes in < 100ms
```

## Testing

Run API tests (requires valid API key):

```bash
just test-api botticelli_models groq
```

Tests are feature-gated and ignored by default:

```rust
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_groq_basic_generation() {
    // Test code
}
```

## Comparison with Other Providers

| Feature | Groq | HuggingFace | Gemini | Anthropic | Ollama |
|---------|------|-------------|---------|-----------|---------|
| **Speed** | 300+ tok/s | 20-50 tok/s | 50-100 tok/s | 30-80 tok/s | Varies |
| **Latency** | 50-200ms | 500ms-2s | 300ms-1s | 400ms-1s | Local |
| **Free Tier** | Generous | Limited | Yes | Limited | N/A |
| **Models** | Llama, Mixtral, Gemma | Thousands | Few | Few | Any |
| **Streaming** | Planned | Planned | ✅ | ✅ | ✅ |
| **Format** | OpenAI | OpenAI | Custom | Custom | OpenAI |
| **Hardware** | LPU | GPU | TPU | GPU | CPU/GPU |

### When to Choose Groq

**Best for:**
- ✅ **Speed-critical applications** - Chatbots, real-time systems
- ✅ **Low latency requirements** - < 100ms time to first token
- ✅ **Consistent performance** - No cold starts or variability
- ✅ **Cost-effective inference** - Speed reduces compute costs
- ✅ **Prototyping** - Fast iteration during development

**Consider alternatives for:**
- ❌ **Multimodal inputs** - Use Gemini or Anthropic
- ❌ **Largest models** - Anthropic's Claude 3.5 Sonnet
- ❌ **Offline use** - Use Ollama for local deployment
- ❌ **Custom fine-tuned models** - HuggingFace or local

## LPU Technology

Groq uses custom LPU (Language Processing Unit) chips designed specifically for LLM inference:

- **Sequential Processing**: Optimized for transformer architecture
- **Memory Bandwidth**: Eliminates GPU memory bottlenecks
- **Deterministic Execution**: Predictable, consistent performance
- **Energy Efficient**: Lower power consumption than GPUs

This hardware advantage makes Groq 5-10x faster than traditional GPU inference.

## Further Reading

- [Groq Console](https://console.groq.com/)
- [Groq API Documentation](https://console.groq.com/docs/quickstart)
- [OpenAI Compatibility](https://console.groq.com/docs/openai)
- [Model Benchmarks](https://wow.groq.com/)
- [LPU Architecture](https://wow.groq.com/lpu-inference-engine/)
- [Botticelli BotticelliDriver Trait](crates/botticelli_interface/src/lib.rs)

## Support

For issues specific to Groq integration:
- Check [GROQ_INTEGRATION_PLAN.md](GROQ_INTEGRATION_PLAN.md)
- Open an issue on GitHub
- Check Groq status page for API issues
- Visit Groq community forum

## Speed Benchmarks

Real-world performance on typical requests:

| Model | Tokens/Second | Time to First Token | Total Time (100 tokens) |
|-------|---------------|---------------------|-------------------------|
| llama-3.2-1b-preview | 500+ | 30-50ms | 200ms |
| llama-3.1-8b-instant | 300-400 | 50-100ms | 300ms |
| llama-3.1-70b-versatile | 100-150 | 100-200ms | 800ms |
| mixtral-8x7b-32768 | 150-200 | 80-150ms | 600ms |

*Note: Actual speeds vary based on prompt length, network latency, and load.*
