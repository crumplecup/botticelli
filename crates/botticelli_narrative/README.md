# botticelli_narrative

Narrative execution engine for the Botticelli ecosystem.

## Overview

Execute multi-act LLM narratives defined in TOML files. Each narrative consists of sequential acts that build on previous outputs, enabling complex workflows like generate → critique → improve.

## Features

- **TOML-based narratives**: Define workflows in simple configuration files
- **Sequential execution**: Acts execute in order with context passing
- **Act processors**: Extensible processing pipeline
- **Content generation**: Automatic table creation and data storage
- **Schema inference**: Generate database schemas from JSON responses
- **Template system**: Use existing schemas as templates
- **Optional persistence**: In-memory or database storage

## Defining Narratives

```toml
# narrative.toml
[narrative]
name = "content-pipeline"
description = "Generate and refine content"

[narration.toc]
acts = ["brainstorm", "draft", "refine"]

[acts.brainstorm]
model = "gemini-1.5-flash"
temperature = 0.9
max_tokens = 500
prompt = "Generate 5 topic ideas about {topic}"

[acts.draft]
model = "gemini-1.5-pro"
temperature = 0.7
max_tokens = 2000
prompt = "Write a detailed post about: {brainstorm.response}"

[acts.refine]
model = "gemini-1.5-flash"
temperature = 0.5
max_tokens = 2000
prompt = "Improve this draft:\n\n{draft.response}"
```

## Executing Narratives

```rust
use botticelli_narrative::{Narrative, NarrativeExecutor};

// Load narrative
let narrative = Narrative::from_file("narrative.toml")?;

// Execute with a driver
let executor = NarrativeExecutor::new(driver);
let execution = executor.execute(&narrative, variables).await?;

// Access results
for act in execution.act_executions {
    println!("{}: {}", act.act_name, act.response);
}
```

## Context Passing

Each act can reference previous act outputs:

```toml
[acts.act2]
prompt = "Based on {act1.response}, what is your opinion?"

[acts.act3]  
prompt = "Combining {act1.response} and {act2.response}, write a summary"
```

## Act Processors

Extend narrative execution with processors:

```rust
use botticelli_narrative::{ActProcessor, ProcessorContext};

struct MyProcessor;

#[async_trait]
impl ActProcessor for MyProcessor {
    fn should_process(&self, ctx: &ProcessorContext) -> bool {
        ctx.act_config.metadata.get("custom") == Some(&"true".to_string())
    }
    
    async fn process(&self, ctx: &mut ProcessorContext) -> BotticelliResult<()> {
        // Custom processing logic
        Ok(())
    }
}

// Register processor
executor.register_processor(Box::new(MyProcessor));
```

## Content Generation

Automatically generate database tables and store results:

```toml
[narrative]
template = "social_posts"  # Use existing schema

# Or let Botticelli infer schema from JSON response
# (no template specified)
```

Features:
- Template-based: Use predefined table schema
- Schema inference: Automatically detect structure from JSON
- Review workflow: Mark content as pending/approved/rejected
- Metadata tracking: Source narrative, act, model, timestamp

## Storage Options

### In-Memory

```rust
use botticelli_narrative::InMemoryNarrativeRepository;

let repo = InMemoryNarrativeRepository::new();
```

### Database

```toml
[dependencies]
botticelli_narrative = { version = "0.2", features = ["database"] }
```

```rust
use botticelli_database::PostgresNarrativeRepository;

let repo = PostgresNarrativeRepository::new(&mut conn)?;
let id = repo.save_execution(&execution).await?;
```

## Extraction Utilities

Extract JSON/TOML from LLM responses:

```rust
use botticelli_narrative::extraction::{extract_json, parse_json};

// LLM often wraps JSON in markdown
let response = "Here's the data:\n```json\n{\"name\": \"Alice\"}\n```";

// Extract raw JSON string
let json_str = extract_json(response)?;

// Parse into struct
let user: User = parse_json(&json_str)?;
```

## Provider Configuration

```rust
use botticelli_narrative::ActConfig;

let config = ActConfig {
    model: "gemini-1.5-flash".to_string(),
    temperature: Some(0.7),
    max_output_tokens: Some(1024),
    inputs: vec![Input::Text(prompt)],
    metadata: HashMap::new(),
};
```

## Dependencies

- `botticelli_interface` - Traits and types
- `botticelli_storage` - Media storage
- `botticelli_database` (optional) - Persistence
- `toml` - TOML parsing
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime

## Version

Current version: 0.2.0
