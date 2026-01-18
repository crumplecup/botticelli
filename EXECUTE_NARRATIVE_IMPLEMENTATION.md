# Execute Narrative Implementation Plan

## Current State Analysis

### What We Have
- Partially implemented `execute_narrative()` function in `crates/botticelli_mcp/src/rmcp_server/tools/execution.rs` (lines 304-500+)
- Function loads narrative TOML and selects appropriate LLM driver
- Successfully creates NarrativeExecutor and executes narrative
- Extracts results from execution (final output, token counts, models used)

### What's Missing
Two critical parameters are defined but **not implemented**:

1. **`prompt` parameter** - "Initial prompt to start the narrative"
   - Currently extracted but unused (line 310)
   - Should inject user-provided context into narrative execution

2. **`max_tokens` parameter** - "Maximum tokens per act"
   - Currently extracted but unused (line 311)
   - Should override narrative's default max_tokens settings

### Current Warnings
```
warning: unused import: `default_model`
warning: unused variable: `narrative` (only in non-feature-gated code path)
```

## Technical Architecture

### Data Flow
```
ExecuteNarrativeParams (user input)
  ├─ narrative_path: String         ✅ Used (load TOML)
  ├─ prompt: String                 ❌ NOT IMPLEMENTED
  ├─ model: Option<String>          ✅ Used (driver selection)
  └─ max_tokens: u32                ❌ NOT IMPLEMENTED

      ↓

MultiNarrative::from_file()
  └─ Narrative (immutable from TOML)
       └─ Vec<(String, ActConfig)>
            ├─ inputs: Vec<Input>           ← WHERE PROMPT SHOULD GO
            ├─ model: Option<String>        ← Already configurable per-act
            ├─ temperature: Option<f32>     ← Already configurable per-act
            └─ max_tokens: Option<u32>      ← WHERE OVERRIDE SHOULD GO

      ↓

NarrativeExecutor::execute_from_source()
  └─ Returns NarrativeExecution
```

### Key Structures

**ActConfig** (from `botticelli_narrative::provider`):
```rust
pub struct ActConfig {
    inputs: Vec<Input>,              // Text, images, audio, video, etc.
    narrative_ref: Option<String>,   // For narrative composition
    model: Option<String>,           // Per-act model override
    temperature: Option<f32>,        // Per-act temperature
    max_tokens: Option<u32>,         // Per-act token limit
    carousel: Option<CarouselConfig>,// Looping config
}
```

**Narrative** (from `botticelli_narrative::core`):
- Loaded from TOML file
- Contains ordered acts (from TOC)
- Acts have individual configs
- **Narrative is immutable after loading**

### The Problem

The `Narrative` type is loaded from TOML and passed immutably to the executor. We cannot simply modify it. We have several architectural options:

## Implementation Options

### Option A: Clone and Modify Narrative (Recommended)
**Approach:** Clone the narrative, modify acts before execution

**Pros:**
- Clean separation of concerns
- Preserves original narrative
- Straightforward implementation

**Cons:**
- Memory overhead (clone entire narrative)
- Must handle all edge cases (carousel acts, narrative_ref acts, etc.)

**Implementation:**
```rust
// 1. Clone narrative after loading
let mut narrative_modified = narrative.clone();

// 2. Apply prompt injection to first act
if !prompt.is_empty() {
    // Get first act's inputs
    let first_act = narrative_modified.acts_mut().first_mut();
    if let Some((_, act_config)) = first_act {
        // Prepend prompt as Input::Text to existing inputs
        let mut new_inputs = vec![Input::Text(prompt.to_string())];
        new_inputs.extend(act_config.inputs().clone());
        act_config.set_inputs(new_inputs);
    }
}

// 3. Apply max_tokens override to all acts
for (_, act_config) in narrative_modified.acts_mut() {
    if act_config.max_tokens().is_none() {
        act_config.set_max_tokens(Some(max_tokens));
    }
}

// 4. Execute modified narrative
let source = NarrativeSource::Single(Arc::new(narrative_modified));
executor.execute_from_source(&source).await
```

### Option B: Executor Configuration
**Approach:** Add execution-time overrides to NarrativeExecutor

**Pros:**
- No narrative cloning
- Cleaner architecture (separation of data vs. execution config)

**Cons:**
- Requires modifying `botticelli_narrative` crate
- More invasive change across codebase
- Breaks existing NarrativeExecutor API

**Not recommended** - would require refactoring across multiple crates.

### Option C: Wrapper/Adapter Pattern
**Approach:** Create a wrapper around Narrative that applies overrides

**Pros:**
- No cloning or mutation
- Preserves original narrative

**Cons:**
- Complex implementation (must implement NarrativeProvider trait)
- Adds layer of indirection
- More code to maintain

**Not recommended** - overengineered for current needs.

## Recommended Implementation: Option A

### Phase 1: Infrastructure Setup
**Goal:** Add mutable access to Narrative/ActConfig

**Files to modify:**
- `crates/botticelli_narrative/src/core.rs` (Narrative)
- `crates/botticelli_narrative/src/provider.rs` (ActConfig)

**Changes needed:**
1. Verify Narrative has `Clone` derive (required for modification)
2. Check if ActConfig has setters or needs them (derive_setters)
3. Add getter for mutable acts access: `acts_mut() -> &mut Vec<(String, ActConfig)>`

**Action items:**
- [ ] Check Narrative derives for Clone
- [ ] Check ActConfig for derive_setters
- [ ] Add acts_mut() to Narrative if not present
- [ ] Add setters to ActConfig if needed (inputs, max_tokens)

### Phase 2: Prompt Injection
**Goal:** Prepend user prompt to first act's inputs

**Location:** `execute_narrative()` after narrative loading (line ~395)

**Logic:**
```rust
// After: let narrative = multi.get_narrative(&narrative_name)?;

// Clone narrative for modification
let mut narrative_modified = narrative.clone();

// Inject prompt into first act if provided and non-empty
if !prompt.is_empty() {
    if let Some((act_name, act_config)) = narrative_modified.acts_mut().first_mut() {
        debug!(%act_name, prompt_len = prompt.len(), "Injecting prompt into first act");
        
        // Prepend prompt as first input
        let mut new_inputs = vec![Input::Text(prompt.to_string())];
        new_inputs.extend_from_slice(act_config.inputs());
        act_config.set_inputs(new_inputs);
    } else {
        warn!("Cannot inject prompt: narrative has no acts");
    }
}
```

**Edge cases to handle:**
- Empty prompt (skip injection)
- Narrative with no acts (log warning, continue)
- First act is narrative_ref (still prepend to inputs? Or skip?)
- First act has carousel (applies to all iterations)

**Instrumentation:**
- `debug!` when injecting prompt (act name, prompt length)
- `warn!` if no acts available
- `trace!` with input count before/after

### Phase 3: Max Tokens Override
**Goal:** Apply max_tokens to all acts that don't have explicit override

**Location:** Same place as prompt injection

**Logic:**
```rust
// Apply max_tokens override to acts without explicit max_tokens
for (act_name, act_config) in narrative_modified.acts_mut() {
    if act_config.max_tokens().is_none() {
        debug!(%act_name, max_tokens, "Applying max_tokens override");
        act_config.set_max_tokens(Some(max_tokens));
    } else {
        trace!(%act_name, act_max = act_config.max_tokens(), "Act has explicit max_tokens, not overriding");
    }
}
```

**Edge cases:**
- Act already has max_tokens (respect it, don't override)
- Act is narrative_ref (still override? Or propagate to child?)
- Zero or invalid max_tokens (validate before application?)

**Instrumentation:**
- `debug!` for each override applied
- `trace!` for acts we skip (already have max_tokens)
- Count how many acts were overridden

### Phase 4: Use Modified Narrative
**Goal:** Execute modified narrative instead of original

**Location:** In the closure at line ~412

**Changes:**
```rust
// BEFORE:
let narrative_clone = narrative.clone();

// Helper to execute narrative and convert result
let execute_narrative = |executor: NarrativeExecutor<_>| async move {
    let source = NarrativeSource::Single(Arc::new(narrative_clone));
    // ...
};

// AFTER:
let narrative_to_execute = narrative_modified; // Use our modified version

// Helper to execute narrative and convert result  
let execute_narrative = |executor: NarrativeExecutor<_>| async move {
    let source = NarrativeSource::Single(Arc::new(narrative_to_execute));
    // ...
};
```

### Phase 5: Warning Cleanup
**Goal:** Fix remaining compilation warnings

**Actions:**
- [ ] Feature-gate `default_model` import (only used in feature-gated code)
- [ ] Remove unused `narrative` warning by using modified version
- [ ] Verify all warnings resolved with `cargo check -p botticelli_mcp`

### Phase 6: Testing
**Goal:** Verify implementation works correctly

**Test cases:**
1. **Basic execution with prompt**
   - Narrative with 2 acts
   - Provide prompt, verify it appears in first act execution

2. **Max tokens override**
   - Narrative with acts missing max_tokens
   - Provide override, verify acts respect it

3. **No override (respect act settings)**
   - Narrative with explicit max_tokens per act
   - Provide override, verify act settings preserved

4. **Empty prompt handling**
   - Empty string prompt
   - Verify no modification to acts

5. **Edge cases**
   - Narrative with zero acts
   - Narrative with carousel in first act
   - Narrative with narrative_ref in first act

**Test location:** `tests/execution_test.rs`

## Success Criteria

- [ ] Prompt injection working (prepends to first act)
- [ ] Max tokens override working (applies to acts without explicit value)
- [ ] All compilation warnings resolved
- [ ] All tests passing
- [ ] Instrumentation complete (debug/trace logs for all operations)
- [ ] Error handling proper (no string-cast errors)

## Notes

### Why Clone Instead of Modify In-Place?
- Narratives are typically Arc-shared
- Multiple references may exist
- Cloning ensures we don't affect other consumers
- Memory cost is acceptable (narratives are small, ~KB range)

### Why Prompt Goes to First Act Only?
- User provides single "initial prompt"
- Subsequent acts build on previous outputs
- First act sets context for entire narrative
- If user wants per-act prompts, they should use narrative TOML

### Why Respect Existing max_tokens?
- Narrative author may have intentionally set specific limits
- Override is a "default" not a "force"
- Principle: narrative TOML is source of truth, params are suggestions

### Alternate Approach: Prompt as Narrative Metadata
Could add prompt to narrative metadata and have executor handle it. This would be cleaner architecturally but requires more invasive changes. Defer to future refactor if this becomes a pattern.

## Implementation Order

1. ✅ Document current state (this file)
2. Phase 1: Infrastructure (verify derives, add setters)
3. Phase 2: Implement prompt injection
4. Phase 3: Implement max_tokens override  
5. Phase 4: Connect modified narrative to execution
6. Phase 5: Clean up warnings
7. Phase 6: Add tests
8. Commit with proper message and push

## Open Questions

1. **Should prompt injection work with narrative_ref acts?**
   - Current thinking: Yes, prepend to inputs regardless
   - Rationale: User intent is clear, applies to execution

2. **Should we validate max_tokens range?**
   - Current thinking: No, let driver/model handle it
   - Rationale: Different models have different limits

3. **Should we support multiple prompt injection points?**
   - Current thinking: No, single prompt for first act only
   - Rationale: Keep API simple, use TOML for complex scenarios

4. **Should we expose narrative_name as a parameter?**
   - Current thinking: Future enhancement
   - Current behavior: Use first narrative found in TOML
