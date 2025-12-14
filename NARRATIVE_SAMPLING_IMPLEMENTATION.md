# Narrative Sampling Implementation Complete

## Overview

Implemented comprehensive LLM-driven narrative generation system using sampling strategy pattern as specified in `NARRATIVE_SAMPLING_STRATEGY.md`.

## Architecture

### Layer Separation

1. **Elicitation Layer** (`botticelli_mcp::elicitation`)
   - Low-level UI interaction primitives
   - Platform-agnostic dialog abstraction
   - State management for partial narratives
   
2. **Sampling Layer** (`botticelli_mcp::tools::sampling`)
   - LLM-driven orchestration
   - Tool-calling workflows  
   - Multi-turn conversation management

3. **Implementation Layer** (`botticelli_chat`)
   - Concrete UI adapters (TUI, CLI)
   - User-facing workflows
   - Depends on both MCP and elicitation

### Key Types

#### Elicitation (`botticelli_mcp::elicitation`)

```rust
// State representation
pub struct PartialNarrative {
    pub name: Option<String>,
    pub description: Option<String>,
    pub acts: Vec<PartialAct>,
    // ...
}

pub struct PartialAct {
    pub name: String,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub inputs: Vec<String>,
    pub carousel: Option<String>,
}

// UI abstraction  
pub trait ElicitationDialog {
    async fn prompt_user(&self, prompt: &str) -> BotticelliResult<String>;
    async fn confirm(&self, question: &str) -> BotticelliResult<bool>;
    async fn show_message(&self, message: &str) -> BotticelliResult<()>;
}

// Component elicitation
pub trait NarrativeElicitor {
    async fn elicit_metadata(&self, partial: &mut PartialNarrative) -> BotticelliResult<()>;
    async fn elicit_act(&self, partial_act: &mut PartialAct) -> BotticelliResult<()>;
    async fn elicit_inputs(&self, act: &mut PartialAct) -> BotticelliResult<()>;
    async fn elicit_carousel(&self, act: &mut PartialAct) -> BotticelliResult<()>;
}
```

#### Sampling (`botticelli_mcp::tools::sampling`)

```rust
// LLM-driven orchestration
pub trait LlmSampler: Send + Sync {
    async fn execute_with_tools(
        &self,
        system_prompt: &str,
        user_message: &str,
        available_tools: Vec<ToolDefinition>,
    ) -> BotticelliResult<SamplingResult>;
}

// Session state management
pub struct SamplingSession {
    state: SessionState,
    turns: Vec<Turn>,
    registry: NarrativeRegistry,
}

pub enum SessionState {
    Planning,
    ElicitingMetadata,
    ElicitingActs,
    ElicitingInputs,
    Validating,
    Complete,
}
```

### MCP Tools

#### Phase 1: Narrative Creation (Implemented)

- **start_narrative** - Initialize new narrative from description
  - Analyzes description with `ElicitationHelper`
  - Creates initial `PartialNarrative`
  - Suggests structure (acts, complexity)
  
- **elicit_metadata** - Collect name, description, model
  - Uses `NarrativeElicitor::elicit_metadata`
  - Validates with `ElicitationHelper::validate_metadata`
  
- **elicit_act** - Configure single act
  - Uses `NarrativeElicitor::elicit_act`
  - Suggests inputs with `ElicitationHelper::suggest_inputs_for_act`
  
- **finalize_narrative** - Convert partial → complete
  - Validates completeness
  - Builds `Narrative` with type-safe constructors
  - Serializes to TOML

#### Helper Utilities

**ElicitationHelper** - Analysis and validation

```rust
impl ElicitationHelper {
    // Description analysis
    pub fn analyze_description(description: &str) -> DescriptionAnalysis;
    pub fn extract_suggested_name(description: &str) -> String;
    pub fn detect_acts_in_description(description: &str) -> Vec<String>;
    pub fn extract_acts_from_description(description: &str) -> McpResult<Vec<PartialAct>>;
    
    // Metadata validation
    pub fn validate_metadata(partial: &PartialNarrative) -> Vec<String>;
    pub fn metadata_warnings(partial: &PartialNarrative) -> Vec<String>;
    pub fn is_valid_name(name: &str) -> bool;
    
    // Act configuration
    pub fn suggest_inputs_for_act(prompt: &str) -> Vec<String>;
    
    // Error construction
    pub fn missing_field(field: &str) -> McpError;
    pub fn invalid_value(field: &str, reason: &str) -> McpError;
    pub fn serialization_error(message: &str) -> McpError;
}
```

**SamplingHelper** - LLM orchestration utilities

```rust
impl SamplingHelper {
    pub fn system_prompt_for_narrative_creation() -> String;
    pub fn extract_tool_calls(response: &GenerateResponse) -> Vec<ToolCall>;
    pub fn format_partial_narrative(partial: &PartialNarrative) -> String;
    pub fn detect_completion(turns: &[Turn]) -> bool;
}
```

**NarrativeHelper** - Narrative manipulation

```rust
impl NarrativeHelper {
    pub fn extract_acts_from_description(description: &str) -> Vec<Act>;
    pub fn is_valid_narrative_name(name: &str) -> bool;
    pub fn suggest_narrative_structure(description: &str) -> Vec<String>;
}
```

### State Management

**NarrativeRegistry** - In-memory narrative tracking

```rust
impl NarrativeRegistry {
    pub fn create(&self, id: String, description: String) -> PartialNarrative;
    pub fn get(&self, id: &str) -> Option<PartialNarrative>;
    pub fn update(&self, id: &str, partial: PartialNarrative);
    pub fn delete(&self, id: &str) -> Option<PartialNarrative>;
    pub fn list(&self) -> Vec<String>;
}
```

## Workflow Example

### User Flow

1. User: "Create a narrative that generates a story with illustrations"

2. LLM calls `start_narrative`:
   ```json
   {
     "description": "Generate a story with illustrations"
   }
   ```
   Returns: `{ "id": "abc123", "suggested_acts": ["generate_text", "create_image"] }`

3. LLM calls `elicit_metadata`:
   ```json
   {
     "id": "abc123",
     "name": "story_with_illustrations",
     "model": "claude-3-5-sonnet-20241022"
   }
   ```

4. LLM calls `elicit_act` for each act:
   ```json
   {
     "id": "abc123",
     "act_name": "generate_text",
     "prompt": "Write a short story",
     "inputs": ["user_prompt"]
   }
   ```

5. LLM calls `finalize_narrative`:
   ```json
   {
     "id": "abc123",
     "save_path": "/path/to/narrative.toml"
   }
   ```

### Implementation Flow

```
User Message
    ↓
LLM with Sampling Strategy
    ↓
Tool Calls (start_narrative, elicit_metadata, etc.)
    ↓
ElicitationDialog (TUI prompts user)
    ↓
PartialNarrative State Updated
    ↓
LLM Continues Based on State
    ↓
finalize_narrative → Complete Narrative TOML
```

## Error Handling

All errors use unified `BotticelliError` type as specified in `CLAUDE.md`:

- `McpError` and `ChatError` variants added to `BotticelliErrorKind`
- Location tracking with `#[track_caller]`
- No error conversion chains that lose context
- All tools return `BotticelliResult<T>`

## Testing

### Test Coverage

- `tests/elicitation_test.rs` - Elicitation trait implementations
- `tests/sampling_test.rs` - Sampling workflow tests
- `tests/validation_test.rs` - Narrative validation

### Test Strategy

- Mock `ElicitationDialog` for automated testing
- Real implementations for integration testing
- No API calls in unit tests

## Dependencies

### Added

- None (uses existing dependencies composably)

### Required Features

- `cli` - Command-line interface
- `anthropic` - Claude API (for LLM sampling)
- `database` - Optional persistence

## Compliance

### CLAUDE.md Compliance

- ✅ All types use builders, never struct literals
- ✅ No `#[cfg(test)]` in source files
- ✅ Tests in `tests/` directory
- ✅ `lib.rs` only has `mod` + `pub use`
- ✅ Imports use `use crate::{Type}`
- ✅ All public functions have `#[instrument]`
- ✅ Errors use `derive_more::Display` + `derive_more::Error`
- ✅ No `#[allow]` directives
- ✅ Zero compilation warnings

### Dependency Graph

```
botticelli_chat (leaf)
    ├── botticelli_mcp (tools + elicitation)
    ├── botticelli_actor (execution)
    └── botticelli_core (types)

botticelli_mcp
    ├── botticelli_narrative (types)
    ├── botticelli_interface (LLM)
    └── botticelli_error (errors)
```

## Next Steps

### Phase 2: Integration with Chat Server

1. Register tools in MCP server
2. Add narrative execution tools
3. Implement persistence layer

### Phase 3: Advanced Features

1. Multi-act orchestration
2. Carousel configuration
3. Validation elicitor (for human-submitted TOML)

### Phase 4: Fallback Strategies

1. Retry with simpler prompts
2. Guided elicitation when LLM fails
3. Progressive refinement

## References

- `NARRATIVE_SAMPLING_STRATEGY.md` - Strategy design
- `NARRATIVE_ELICITATION_PLAN.md` - Original plan
- `CLAUDE.md` - Code standards
