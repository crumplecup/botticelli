# Ollama Integration

Botticelli supports [Ollama](https://ollama.ai/) for running local LLMs.

## Features

- ✅ Local execution (no API costs)
- ✅ Privacy (data stays on your machine)
- ✅ Support for all Ollama models
- ✅ Streaming responses
- ✅ No rate limits (local only)

## Prerequisites

1. **Install Ollama**: Follow instructions at https://ollama.ai/

2. **Download a model**:
   ```bash
   ollama pull llama2
   ollama pull mistral
   ollama pull codellama
   ```

3. **Start Ollama server** (usually starts automatically):
   ```bash
   ollama serve
   ```

## Configuration

### Enable Ollama Feature

In `Cargo.toml`:
```toml
[features]
default = ["ollama"]
ollama = ["botticelli_models/ollama"]
```

### Narrative Configuration

Create a narrative that uses Ollama:

```toml
[[narrative]]
name = "local_generation"
description = "Generate content using local Ollama"
model = "llama2"  # Or mistral, codellama, etc.
provider = "ollama"

[[narrative.actors]]
role = "user"
content = "Write a short story about a robot"
```

### Ollama Server URL

Default: `http://localhost:11434`

To use a custom URL, set environment variable:
```bash
export OLLAMA_HOST=http://your-server:11434
```

Or in code:
```rust
use botticelli::OllamaClient;

let client = OllamaClient::new_with_url(
    "llama2",
    "http://192.168.1.100:11434"
)?;
```

## Usage Examples

### Basic Generation

```rust
use botticelli::{OllamaClient, BotticelliDriver, GenerateRequest, Message, Role, Input};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("llama2")?;
    
    let request = GenerateRequest::builder()
        .messages(vec![
            Message::builder()
                .role(Role::User)
                .content(vec![Input::Text("Hello!".to_string())])
                .build()?
        ])
        .build()?;
    
    let response = client.generate(request).await?;
    
    println!("Response: {:?}", response.outputs());
    
    Ok(())
}
```

### Streaming Generation

```rust
use botticelli::{OllamaClient, BotticelliDriver};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new("llama2")?;
    
    let request = /* ... */;
    
    let mut stream = client.generate_stream(request).await?;
    
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                if let Some(text) = chunk.text() {
                    print!("{}", text);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## Available Models

Common Ollama models:

- `llama2` - Meta's Llama 2 (7B, 13B, 70B)
- `mistral` - Mistral 7B
- `codellama` - Code-specialized Llama
- `phi` - Microsoft's Phi models
- `neural-chat` - Intel's neural chat model
- `starling-lm` - Starling language model

See full list: https://ollama.ai/library

## Performance

### Local Execution

- **No rate limits** - only limited by your hardware
- **No API costs** - completely free
- **Privacy** - data never leaves your machine

### Hardware Requirements

- **Minimum**: 8GB RAM for 7B models
- **Recommended**: 16GB RAM + GPU for better performance
- **Optimal**: 32GB+ RAM + NVIDIA GPU with CUDA

### Concurrent Requests

Configure in `botticelli.toml`:

```toml
[providers.ollama.tiers.local]
max_concurrent = 4  # Adjust based on your hardware
```

## Troubleshooting

### "Connection refused"

Ollama server is not running. Start it:
```bash
ollama serve
```

### "Model not found"

Download the model first:
```bash
ollama pull llama2
```

### Slow responses

1. Check if GPU is being used: `ollama ps`
2. Reduce concurrent requests in config
3. Use smaller models (7B instead of 13B/70B)

### Out of memory

Use a smaller model or increase system RAM.

## Comparison with Cloud Providers

| Feature | Ollama | Gemini | OpenAI |
|---------|--------|--------|--------|
| Cost | Free | Paid API | Paid API |
| Privacy | Full | Cloud | Cloud |
| Speed | Hardware-dependent | Fast | Fast |
| Rate Limits | None | Yes | Yes |
| Internet | Not required | Required | Required |
| Models | Open source | Proprietary | Proprietary |

## Next Steps

- See [NARRATIVE_TOML_SPEC.md](./NARRATIVE_TOML_SPEC.md) for narrative configuration
- See [PLANNING_INDEX.md](./PLANNING_INDEX.md) for integration guides
- Try example narratives in `examples/`
