# Nested Narrative Execution Issue

## Problem

When running `just narrate publish_rules`, the narrative fails because:

1. Act 1 references `narratives.generate_content` which points to `rules_content_generation.toml`
2. This nested narrative should execute and create the `community_rules` table
3. Act 2 tries to query the `community_rules` table
4. **BUG**: The nested narrative is NOT being executed - it's being treated as a text input
5. Act 2 fails because the table doesn't exist

## Current Behavior

The TOML parser recognizes `[narratives.generate_content]` and parses it, but the executor doesn't know how to:
- Detect that an input is a nested narrative reference
- Load the narrative file from disk
- Execute it and wait for completion
- Continue with the next act only after the nested narrative completes

## Root Cause

The `Input` enum in `botticelli_core` only supports:
- Text
- Image
- Audio
- Video
- Document

There's NO variant for "nested narrative" or "narrative reference".

## Solution Options

### Option 1: Add InputKind::Narrative Variant (Not Recommended)

Add a new variant to `Input` enum:
```rust
pub enum Input {
    // ... existing variants
    Narrative {
        path: String,
    },
}
```

**Problems:**
- Input is meant for LLM API inputs (text, media)
- Narrative execution is a meta-operation, not LLM input
- Pollutes the core type system with execution concerns

### Option 2: Handle at ActConfig Level (Recommended)

Create a special handling in the executor for narrative references:

1. **TOML parsing**: When we see `[narratives.X]`, store it separately from regular inputs
2. **Executor**: Check if an act input is a narrative reference before processing
3. **Execute nested**: If it's a narrative ref, recursively call `execute()` and await completion
4. **Continue**: Only proceed to next act after nested narrative completes

**Benefits:**
- Keeps `Input` enum clean and focused on LLM inputs
- Narrative composition handled at the right abstraction level
- No changes to core types needed

### Option 3: Preprocessing Step (Alternative)

Add a preprocessing phase that:
1. Detects all narrative references in a narrative
2. Executes them in dependency order
3. Replaces references with their outputs

**Problems:**
- Breaks the sequential act execution model
- Hard to reason about execution order
- Loses the declarative nature of the TOML

## Recommended Implementation (Option 2)

### Step 1: Update TOML Parser

Mark narrative references distinctly from regular inputs:

```rust
// In toml_parser.rs
pub struct NarrativeReference {
    pub path: String,
}

// Store these separately in ActConfig
pub struct ActConfig {
    inputs: Vec<Input>,
    narrative_refs: Vec<NarrativeReference>,  // NEW
    // ... rest
}
```

### Step 2: Update Executor

In `process_inputs()`:

```rust
async fn process_inputs(&self, config: &ActConfig) -> Result<String> {
    let mut processed = Vec::new();
    
    // Execute nested narratives FIRST
    for narrative_ref in config.narrative_refs() {
        let nested_narrative = Narrative::from_file(&narrative_ref.path)?;
        let execution = self.execute(&nested_narrative).await?;
        // Optionally include execution results in processed inputs
    }
    
    // Then process regular inputs (bot commands, tables, text)
    for input in config.inputs() {
        // ... existing logic
    }
    
    Ok(processed.join("\n\n"))
}
```

### Step 3: Test

Run `just narrate publish_rules` and verify:
1. Act 1 loads and executes `rules_content_generation.toml`
2. Table `community_rules` is created and populated
3. Act 2 successfully queries the table
4. Act 3 publishes the selected content

## Testing Strategy

1. **Unit test**: Parse TOML with narrative references
2. **Integration test**: Execute nested narrative and verify table creation
3. **End-to-end test**: Run publish_rules and verify Discord message

## Timeline

- [ ] Update TOML parser to recognize narrative references
- [ ] Update ActConfig to store narrative references separately
- [ ] Implement recursive execution in NarrativeExecutor
- [ ] Add await/blocking to ensure completion before next act
- [ ] Test with publish_rules.toml
- [ ] Document nested narrative feature in NARRATIVE_TOML_SPEC.md
