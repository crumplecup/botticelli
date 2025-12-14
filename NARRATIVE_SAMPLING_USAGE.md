# Narrative Sampling Usage Guide

## Overview

The narrative sampling system enables LLM-driven creation of TOML narratives through natural language conversation. The system uses MCP tools to manage narrative state and a sampling agent to orchestrate the creation process.

## Architecture

```
User → Chat Interface → SamplingAgent → MCP Tools → NarrativeRegistry
                            ↓
                       LLM Provider
```

### Components

1. **SamplingAgent**: Orchestrates multi-turn conversations with LLM
2. **MCP Tools**: Stateful operations for narrative construction
3. **NarrativeRegistry**: In-memory narrative state management
4. **Chat Integration**: Bridges user interface to sampling layer

## Basic Usage

### Starting a Narrative Creation Session

```rust
use botticelli_chat::{start_narrative_sampling, ChatConfig};
use botticelli_mcp::NarrativeRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ChatConfig::default();
    let registry = NarrativeRegistry::new();
    
    // User provides initial description
    let user_prompt = "Create a narrative about a detective solving a murder mystery in 3 acts";
    
    // Start sampling session
    let result = start_narrative_sampling(
        &config,
        registry.clone(),
        user_prompt
    ).await?;
    
    println!("Narrative created: {}", result.narrative_id);
    Ok(())
}
```

### Interactive Workflow

1. **User initiates**: Provides high-level description
2. **LLM plans**: Analyzes requirements, determines components needed
3. **LLM creates**: Calls MCP tools to build narrative incrementally
4. **User reviews**: Examines generated TOML
5. **User refines**: Provides feedback for modifications
6. **LLM adjusts**: Updates narrative based on feedback

## MCP Tools Reference

### create_narrative

Creates a new narrative session.

```json
{
  "name": "create_narrative",
  "arguments": {
    "title": "Detective Mystery",
    "description": "A murder mystery in Victorian London",
    "genre": "Mystery"
  }
}
```

### add_act

Adds an act to the narrative.

```json
{
  "name": "add_act",
  "arguments": {
    "narrative_id": "narrative_123",
    "act_number": 1,
    "description": "Detective arrives at crime scene"
  }
}
```

### add_input

Adds an input to an act.

```json
{
  "name": "add_input",
  "arguments": {
    "narrative_id": "narrative_123",
    "act_number": 1,
    "role": "user",
    "content": "Examine the body for clues"
  }
}
```

### get_narrative

Retrieves current narrative state as TOML.

```json
{
  "name": "get_narrative",
  "arguments": {
    "narrative_id": "narrative_123"
  }
}
```

### validate_narrative

Validates narrative completeness and structure.

```json
{
  "name": "validate_narrative",
  "arguments": {
    "narrative_id": "narrative_123"
  }
}
```

### list_narratives

Lists all narratives in the registry.

```json
{
  "name": "list_narratives",
  "arguments": {}
}
```

## Example Conversation Flow

### Turn 1: User Request
```
User: "Create a narrative about a space explorer discovering an ancient alien artifact"
```

### Turn 2: LLM Planning
```
LLM: I'll create a narrative with 3 acts exploring this theme. Let me start...

Tool Call: create_narrative
{
  "title": "Ancient Discovery",
  "description": "Space explorer finds alien artifact",
  "genre": "Science Fiction"
}
```

### Turn 3: LLM Building
```
Tool Call: add_act
{
  "narrative_id": "narrative_001",
  "act_number": 1,
  "description": "Explorer approaches mysterious signal on remote planet"
}

Tool Call: add_input
{
  "narrative_id": "narrative_001",
  "act_number": 1,
  "role": "system",
  "content": "You are exploring Kepler-442b..."
}
```

### Turn 4: LLM Completion
```
Tool Call: get_narrative
{
  "narrative_id": "narrative_001"
}

Response: Here's your narrative in TOML format:
[narrative shows complete TOML structure]

Would you like to adjust any acts or add more detail?
```

### Turn 5: User Refinement
```
User: "Make act 1 more suspenseful, add strange readings"
```

### Turn 6: LLM Modification
```
Tool Call: add_input
{
  "narrative_id": "narrative_001",
  "act_number": 1,
  "role": "system",
  "content": "Your scanner detects anomalous energy signatures..."
}

I've enhanced act 1 with mysterious scanner readings. The tension builds nicely now.
```

## Configuration

### SamplingAgent Configuration

```rust
use botticelli_chat::SamplingConfig;

let config = SamplingConfig::builder()
    .max_turns(15)              // Maximum conversation turns
    .temperature(0.7)           // LLM creativity level
    .model("claude-3-5-sonnet") // Model selection
    .build()?;
```

### System Prompt Customization

The sampling agent uses a specialized system prompt that:
- Describes available MCP tools
- Provides narrative structure guidelines
- Encourages iterative refinement
- Enforces TOML schema compliance

Custom prompts can be provided via `SamplingConfig::with_system_prompt()`.

## Error Handling

### Common Errors

**NarrativeNotFound**: Narrative ID doesn't exist
```rust
match result {
    Err(BotticelliError::Mcp(McpErrorKind::NarrativeNotFound(id))) => {
        eprintln!("Narrative {} not found", id);
    }
    _ => {}
}
```

**ValidationFailed**: Narrative structure is incomplete
```rust
match validate_result {
    Err(BotticelliError::Mcp(McpErrorKind::ValidationFailed(msg))) => {
        eprintln!("Validation failed: {}", msg);
    }
    _ => {}
}
```

**ActNotFound**: Act number doesn't exist
```rust
match result {
    Err(BotticelliError::Mcp(McpErrorKind::ActNotFound { narrative_id, act_number })) => {
        eprintln!("Act {} not found in narrative {}", act_number, narrative_id);
    }
    _ => {}
}
```

## Advanced Usage

### Multi-User Sessions

```rust
// Each user gets their own registry
let user_registries: HashMap<UserId, Arc<NarrativeRegistry>> = HashMap::new();

// Isolate user narratives
let registry = user_registries
    .entry(user_id)
    .or_insert_with(|| Arc::new(NarrativeRegistry::new()))
    .clone();
```

### Persistent Storage

```rust
// Save narrative to file
let toml_content = registry.get_narrative(&narrative_id)?;
std::fs::write("my_narrative.toml", toml_content)?;

// Load narrative from file
let toml_content = std::fs::read_to_string("my_narrative.toml")?;
// User can then validate and modify via chat
```

### Fallback Strategies

The sampling agent implements fallback strategies:

1. **Tool Call Retry**: Automatic retry with adjusted parameters
2. **Simplification**: Break complex requests into smaller steps
3. **Validation Loop**: Iterative fixes for validation errors
4. **User Guidance**: Request clarification when stuck

## Testing

### Unit Tests

```bash
cargo test -p botticelli_mcp --lib
cargo test -p botticelli_chat --lib
```

### Integration Tests

```bash
cargo test -p botticelli_mcp --test narrative_sampling_test
```

### API Tests (Rate-Limited)

```bash
just test-api
```

## Best Practices

### For Users

1. **Be specific**: Clear descriptions lead to better narratives
2. **Iterate**: Start simple, refine through conversation
3. **Review TOML**: Check generated structure matches intent
4. **Save often**: Export TOML to files for backup

### For Developers

1. **Validate inputs**: Check tool arguments before processing
2. **Provide context**: Include narrative_id in error messages
3. **Log tool calls**: Use tracing for debugging
4. **Handle failures**: Implement retry logic for transient errors

## Troubleshooting

### LLM doesn't call tools

**Cause**: System prompt may be unclear or model doesn't support tools
**Solution**: Verify model supports function calling, check system prompt

### Narrative validation fails

**Cause**: Missing required fields or malformed structure
**Solution**: Use `get_narrative` to inspect current state, add missing components

### Registry grows too large

**Cause**: Long-running sessions accumulate narratives
**Solution**: Implement periodic cleanup, export completed narratives

### Tool calls timeout

**Cause**: Network issues or slow LLM responses
**Solution**: Increase timeout values, implement retry with backoff

## Next Steps

- See [NARRATIVE_SAMPLING_STRATEGY.md](NARRATIVE_SAMPLING_STRATEGY.md) for implementation details
- See [NARRATIVE_TOML_SPEC.md](NARRATIVE_TOML_SPEC.md) for TOML schema
- See [tests/narrative_sampling_test.rs](crates/botticelli_mcp/tests/narrative_sampling_test.rs) for examples
