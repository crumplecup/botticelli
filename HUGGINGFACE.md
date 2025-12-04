# HuggingFace Integration Guide

## Overview

Botticelli provides support for HuggingFace's Inference API through their OpenAI-compatible router endpoint. This gives you access to thousands of models hosted on HuggingFace with a simple, familiar API.

## Quick Start

```rust
use botticelli::{HuggingFaceDriver, BotticelliDriver, GenerateRequest, Message, Role, Input};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    
    let driver = HuggingFaceDriver::new(
        "meta-llama/Llama-3.2-1B-Instruct".to_string()
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
botticelli = { version = "0.2", features = ["huggingface"] }
```

### 2. Get an API Key

1. Sign up at [huggingface.co](https://huggingface.co)
2. Go to Settings → Access Tokens
3. Create a new token with read access
4. Copy your token (starts with `hf_`)

### 3. Set Environment Variable

```bash
export HUGGINGFACE_API_KEY="hf_your_token_here"
```

Or in `.env`:

```
HUGGINGFACE_API_KEY=hf_your_token_here
```

## API Details

### Endpoint

HuggingFace uses an OpenAI-compatible endpoint:

```
https://router.huggingface.co/v1/chat/completions
```

### Format

Requests use the OpenAI chat completions format:

```json
{
  "model": "meta-llama/Llama-3.2-1B-Instruct",
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
  ]
}
```

## Model Selection

### ✅ Supported Models

HuggingFace router **only works with chat-capable models**:

- `meta-llama/Llama-3.2-1B-Instruct` - Small, fast (tested)
- `meta-llama/Llama-3.2-3B-Instruct` - Slightly larger
- `meta-llama/Llama-3.1-8B-Instruct` - High quality
- `mistralai/Mistral-7B-Instruct-v0.3` - Good performance
- Other models ending in `-Instruct` or `-Chat`

### ❌ Unsupported Models

Base/completion-only models **will not work**:

- `gpt2`, `distilgpt2` - Base models
- `meta-llama/Llama-2-7b` - Base model (without `-Instruct`)
- Most fine-tuned models without chat formatting

**Error:** `"The requested model 'X' is not a chat model"`

### Finding Models

Browse HuggingFace Hub for chat models:
- Filter by "Text Generation"
- Look for "Instruct" or "Chat" in model names
- Check model cards for chat template support

## Parameters

The driver supports standard parameters:

```rust
let request = GenerateRequest::builder()
    .messages(vec![message])
    .max_tokens(Some(100))      // Maximum tokens to generate
    .temperature(Some(0.7))     // Sampling temperature (0.0-1.0)
    .build()?;
```

**Note:** `top_p` is not currently exposed in `GenerateRequest` but can be added.

## Rate Limits and Pricing

### Free Tier

- **Credits**: $0.10 USD/month (free)
- **Pro Users**: $2.00 USD/month
- **Usage**: Pay-per-token after credits
- **Rate Limits**: Shared infrastructure

### Best Practices

1. **Use small models** for testing (1B-3B parameters)
2. **Keep max_tokens low** during development
3. **Cache responses** when possible
4. **Monitor usage** on HuggingFace dashboard

## Error Handling

```rust
use botticelli_error::{BotticelliError, HuggingFaceErrorKind};

match driver.generate(&request).await {
    Ok(response) => {
        // Handle success
    }
    Err(e) => {
        if let BotticelliError::Models(models_err) = &e {
            match &models_err.kind {
                botticelli_error::ModelsErrorKind::HuggingFace(hf_err) => {
                    match hf_err {
                        HuggingFaceErrorKind::Api(msg) => {
                            eprintln!("API error: {}", msg);
                        }
                        HuggingFaceErrorKind::RateLimit => {
                            eprintln!("Rate limit exceeded");
                        }
                        HuggingFaceErrorKind::ModelNotFound(model) => {
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

### "Model not found"

**Cause:** Model doesn't exist or isn't accessible with your token.

**Solution:** 
- Check model name spelling
- Verify model exists on HuggingFace Hub
- Ensure token has access to the model

### "Not a chat model"

**Cause:** Model doesn't support chat format.

**Solution:** Use an Instruct or Chat variant of the model.

### Authentication Errors

**Cause:** Missing or invalid API key.

**Solution:**
- Check `HUGGINGFACE_API_KEY` is set
- Verify token is valid (starts with `hf_`)
- Generate new token if needed

### Rate Limit Exceeded

**Cause:** Used all free tier credits or hit rate limits.

**Solution:**
- Wait for credits to refresh (monthly)
- Upgrade to Pro tier
- Use smaller models or reduce max_tokens

## Streaming

**Status:** Not yet implemented

Streaming support is planned but currently falls back to non-streaming. The response will be returned as a single chunk.

```rust
// Currently returns single chunk
let stream = driver.generate_stream(&request).await?;
```

## Examples

### Multi-Turn Conversation

```rust
let messages = vec![
    Message::builder()
        .role(Role::System)
        .content(vec![Input::Text("You are a helpful assistant.".to_string())])
        .build()?,
    Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("What is Rust?".to_string())])
        .build()?,
    Message::builder()
        .role(Role::Assistant)
        .content(vec![Input::Text("Rust is a systems programming language.".to_string())])
        .build()?,
    Message::builder()
        .role(Role::User)
        .content(vec![Input::Text("Tell me more.".to_string())])
        .build()?,
];

let request = GenerateRequest::builder()
    .messages(messages)
    .max_tokens(Some(200))
    .build()?;

let response = driver.generate(&request).await?;
```

### With Custom API Token

```rust
let driver = HuggingFaceDriver::with_api_token(
    "hf_your_token_here".to_string(),
    "meta-llama/Llama-3.2-1B-Instruct".to_string()
)?;
```

## Testing

Run API tests (requires valid token):

```bash
just test-api botticelli_models huggingface
```

Tests are feature-gated and ignored by default:

```rust
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_huggingface_basic_generation() {
    // Test code
}
```

## Comparison with Other Providers

| Feature | HuggingFace | Gemini | Anthropic | Ollama |
|---------|-------------|---------|-----------|---------|
| Free Tier | $0.10/month | Yes | Limited | N/A (local) |
| Models | Thousands | Few | Few | Any (local) |
| Streaming | Planned | ✅ | ✅ | ✅ |
| Multimodal | Limited | ✅ | ✅ | Limited |
| Format | OpenAI | Custom | Custom | OpenAI |

## Further Reading

- [HuggingFace Inference API Docs](https://huggingface.co/docs/api-inference/index)
- [HuggingFace Model Hub](https://huggingface.co/models)
- [OpenAI API Reference](https://platform.openai.com/docs/api-reference/chat) (format reference)
- [Botticelli BotticelliDriver Trait](crates/botticelli_interface/src/lib.rs)

## Support

For issues specific to HuggingFace integration:
- Check [HUGGINGFACE_INTEGRATION_PLAN.md](HUGGINGFACE_INTEGRATION_PLAN.md)
- Open an issue on GitHub
- Check HuggingFace status page for API issues
