# botticelli

Unified facade crate for the Botticelli ecosystem.

## Overview

This is the main entry point for using Botticelli. It re-exports all workspace crates through a single dependency for convenience and backward compatibility.

## Quick Start

```toml
[dependencies]
botticelli = { version = "0.2", features = ["gemini", "database"] }
```

```rust
use botticelli::{
    GeminiClient,
    Narrative,
    NarrativeExecutor,
    PostgresNarrativeRepository,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = GeminiClient::new(api_key, "gemini-1.5-flash");
    
    // Load narrative
    let narrative = Narrative::from_file("narrative.toml")?;
    
    // Execute
    let executor = NarrativeExecutor::new(client);
    let execution = executor.execute(&narrative, variables).await?;
    
    // Optionally persist
    let repo = PostgresNarrativeRepository::new(&mut conn)?;
    let id = repo.save_execution(&execution).await?;
    
    Ok(())
}
```

## Feature Flags

### LLM Providers

- `gemini` - Google Gemini models (default: off)
- `anthropic` - Anthropic Claude models (default: off, coming soon)
- `openai` - OpenAI GPT models (default: off, coming soon)

### Optional Features

- `database` - PostgreSQL persistence (default: off)
- `discord` - Discord bot integration (default: off)
- `tui` - Terminal UI for content review (default: off)
- `all` - Enable all features (default: off)

### Examples

```toml
# Minimal - just Gemini
[dependencies]
botticelli = { version = "0.2", features = ["gemini"] }

# With persistence
[dependencies]
botticelli = { version = "0.2", features = ["gemini", "database"] }

# Full stack
[dependencies]
botticelli = { version = "0.2", features = ["all"] }
```

## Workspace Crates

This facade re-exports from:

### Foundation

- `botticelli_error` - Error types
- `botticelli_core` - Core data structures
- `botticelli_interface` - Trait definitions

### Core Features

- `botticelli_rate_limit` - Rate limiting and retry
- `botticelli_storage` - Content-addressable storage
- `botticelli_narrative` - Narrative execution engine

### Optional Features

- `botticelli_models` - LLM provider implementations
- `botticelli_database` - PostgreSQL integration
- `botticelli_social` - Social platform integrations
- `botticelli_tui` - Terminal UI

## Using Individual Crates

For more control, depend on crates directly:

```toml
[dependencies]
botticelli_interface = "0.2"
botticelli_models = { version = "0.2", features = ["gemini"] }
botticelli_narrative = "0.2"
```

This gives you:
- Smaller dependency tree
- Faster compile times
- More explicit control

## What's Exported

All types, traits, and functions from workspace crates are re-exported at the root level:

```rust
// These are equivalent:
use botticelli::GeminiClient;
use botticelli_models::GeminiClient;

use botticelli::Narrative;
use botticelli_narrative::Narrative;

use botticelli::BotticelliDriver;
use botticelli_interface::BotticelliDriver;
```

## Migration from 0.1.x

If upgrading from the monorepo version (0.1.x):

### Before (0.1.x)

```toml
[dependencies]
botticelli = "0.1"
```

### After (0.2.x)

```toml
[dependencies]
botticelli = { version = "0.2", features = ["gemini", "database"] }
```

Code imports remain the same:

```rust
// Still works!
use botticelli::{GeminiClient, Narrative, NarrativeExecutor};
```

## Version

Current version: 0.2.0

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
