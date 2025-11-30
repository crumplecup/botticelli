# Narrative Composition Loading Issue

**Status**: Critical - Blocks actor-server execution  
**Date**: 2025-11-30  
**Priority**: HIGH - Must solve before production deployment

## Problem Statement

The actor-server fails to execute the `batch_generate` narrative because referenced sub-narratives are not loaded when using narrative composition.

### Error Message

```
ERROR: Unrecoverable error, cannot retry
ActorError: Narrative Error: Configuration error: Referenced narrative 'feature' not found. 
Narrative composition requires MultiNarrative.
at line 399 in crates/botticelli_narrative/src/executor.rs
```

### Context

- **File**: `crates/botticelli_narrative/narratives/discord/generation_carousel.toml`
- **Narrative**: `batch_generate` (carousel with 5 composition acts)
- **Missing**: Sub-narratives `feature`, `usecase`, `tutorial`, `community`, `problem`
- **Location**: All sub-narratives ARE defined in the same TOML file

## Root Cause Analysis

### File Structure

The `generation_carousel.toml` file uses multi-narrative format:

```toml
# Main carousel narrative (orchestrator)
[narratives.batch_generate]
name = "generation_batch_50"

[narratives.batch_generate.acts.feature]
narrative = "feature"  # References another narrative

[narratives.batch_generate.acts.usecase]
narrative = "usecase"

# ... more composition acts ...

# Referenced narratives (in same file)
[narratives.feature]
name = "feature_showcase"
# ... full narrative definition ...

[narratives.usecase]
name = "usecase_showcase"
# ... full narrative definition ...

# ... more narrative definitions ...
```

### Current Loading Behavior

**Actor Config** (`generation_actor.toml`):
```toml
[skills.narrative_execution]
enabled = true
narrative_path = "crates/botticelli_narrative/narratives/discord/generation_carousel.toml"
narrative_name = "batch_generate"
```

**What Happens**:
1. Narrative loader reads the TOML file
2. Parser detects multi-narrative format
3. System extracts ONLY `batch_generate` narrative by name
4. Referenced narratives (`feature`, `usecase`, etc.) are NOT included
5. Executor tries to run composition act `feature`
6. Error: `feature` narrative not found

### The Fundamental Problem

**Narrative composition requires access to the entire MultiNarrative context**, but the current loading mechanism extracts a single narrative when `narrative_name` is specified.

There are TWO conflicting requirements:
1. **Multi-narrative files need `narrative_name`** to specify which narrative to execute
2. **Composition requires the full MultiNarrative** to resolve references

The current code path:
```
Load file → Parse as MultiNarrative → Extract single narrative → Execute
                                          ^^^^^^^^^^^^^^^^^^^^^^^^
                                          This breaks composition!
```

## Minimum Reproduction

### Step 1: Create Minimal Multi-Narrative File

Create `test_composition_minimal.toml`:

```toml
# Orchestrator narrative with composition
[narratives.orchestrator]
name = "test_orchestrator"

[narratives.orchestrator.toc]
order = ["call_worker"]

[narratives.orchestrator.acts.call_worker]
narrative = "worker"  # Composition reference

# Worker narrative
[narratives.worker]
name = "test_worker"

[narratives.worker.toc]
order = ["do_work"]

[acts.do_work]
model = "gemini-2.5-flash-lite"
temperature = 0.5
max_tokens = 100

[[acts.do_work.input]]
type = "text"
content = "Say hello from worker narrative"
```

### Step 2: Create Minimal Actor Config

Create `test_composition_actor.toml`:

```toml
[actor]
name = "Test Composition Actor"
description = "Tests narrative composition loading"
knowledge = []
skills = ["narrative_execution"]

[actor.config]

[actor.execution]
stop_on_unrecoverable = false
max_retries = 1
continue_on_error = true

[skills.narrative_execution]
enabled = true
narrative_path = "test_composition_minimal.toml"
narrative_name = "orchestrator"  # Should load orchestrator + worker
```

### Step 3: Run Test

```bash
cargo run -p botticelli_actor --bin actor-server \
  --features discord \
  --config test_server.toml
```

**Expected**: Executes orchestrator → calls worker → success  
**Actual**: Error: Referenced narrative 'worker' not found

## Solution Strategy

### Option 1: Always Load Full MultiNarrative for Composition ✅ RECOMMENDED

**Approach**: When a narrative has composition acts, load the entire MultiNarrative context.

**Implementation**:
1. Parse TOML file as `MultiNarrative`
2. Find the specified `narrative_name`
3. **Check if it has composition acts** (`act.narrative` is set)
4. If yes: Pass entire `MultiNarrative` to executor
5. If no: Extract single narrative (current behavior)

**Code Changes**:
```rust
// In narrative loader
pub fn load_narrative(path: &Path, name: Option<&str>) -> Result<NarrativeSource> {
    let content = fs::read_to_string(path)?;
    
    match parse_toml(&content) {
        ParseResult::Single(narrative) => Ok(NarrativeSource::Single(narrative)),
        ParseResult::Multi(multi) => {
            if let Some(name) = name {
                let narrative = multi.get(name)?;
                
                // Check if narrative uses composition
                if has_composition_acts(&narrative) {
                    // Keep full context for composition resolution
                    Ok(NarrativeSource::MultiWithContext(multi, name))
                } else {
                    // Single narrative, no references
                    Ok(NarrativeSource::Single(narrative))
                }
            } else {
                Err(Error::MultiNarrativeRequiresName)
            }
        }
    }
}
```

**Pros**:
- Minimal code changes
- Maintains backward compatibility
- Solves composition issue correctly
- No config file changes needed

**Cons**:
- Slight memory overhead (holds unused narratives)
- Requires new enum variant `NarrativeSource`

### Option 2: Load All Referenced Narratives Transitively

**Approach**: When loading a narrative, follow all composition references and include them.

**Implementation**:
1. Parse TOML as `MultiNarrative`
2. Find specified narrative
3. Scan its acts for `narrative = "name"` references
4. Recursively load each referenced narrative
5. Build a `PartialMultiNarrative` with only required narratives

**Pros**:
- Minimal memory footprint
- Clean separation of concerns
- Explicit dependency tracking

**Cons**:
- More complex implementation
- Requires dependency graph traversal
- Potential circular reference issues
- More code to maintain

### Option 3: Change Config to Not Specify narrative_name

**Approach**: Execute entire MultiNarrative, use first narrative or require different config.

**Implementation**:
1. Remove `narrative_name` from actor configs
2. Execute first narrative in MultiNarrative
3. Or add `[narratives.default]` marker

**Pros**:
- Simplest code change
- MultiNarrative always available

**Cons**:
- Breaks explicit narrative selection
- Requires config file changes
- Ambiguous when multiple narratives present
- Less flexible

### Option 4: Flatten Composition During Parse

**Approach**: Resolve all composition references at parse time, inline them.

**Implementation**:
1. Parse multi-narrative file
2. Find specified narrative
3. For each composition act, inline the referenced narrative's acts
4. Return single narrative with no references

**Pros**:
- No executor changes needed
- Single narrative output
- Explicit flattening

**Cons**:
- Complex parsing logic
- Loses composition structure
- Hard to debug
- Variable name conflicts possible

## Recommended Solution: Option 1

**Why**: 
- Minimal risk, high compatibility
- Correctly models the problem (composition needs context)
- Clean code architecture
- No config changes needed
- Easiest to test and validate

## Implementation Plan

### Phase 1: Add NarrativeSource Enum (30 min)

**File**: `crates/botticelli_narrative/src/types.rs` or `loader.rs`

```rust
/// Source of a narrative for execution
#[derive(Debug, Clone)]
pub enum NarrativeSource {
    /// Single narrative (no composition)
    Single(Narrative),
    
    /// Multi-narrative with context (for composition)
    /// Contains full MultiNarrative and the name to execute
    MultiWithContext {
        multi: MultiNarrative,
        execute_name: String,
    },
}

impl NarrativeSource {
    /// Get the narrative to execute
    pub fn get_narrative(&self) -> Result<&Narrative> {
        match self {
            Self::Single(n) => Ok(n),
            Self::MultiWithContext { multi, execute_name } => {
                multi.get(execute_name)
            }
        }
    }
    
    /// Get the full multi-narrative context (if available)
    pub fn get_multi_context(&self) -> Option<&MultiNarrative> {
        match self {
            Self::Single(_) => None,
            Self::MultiWithContext { multi, .. } => Some(multi),
        }
    }
}
```

### Phase 2: Update Loader Logic (45 min)

**File**: `crates/botticelli_narrative/src/loader.rs`

```rust
fn has_composition_acts(narrative: &Narrative) -> bool {
    narrative.toc.order.iter().any(|act_name| {
        narrative.acts.get(act_name)
            .map(|act| act.narrative.is_some())
            .unwrap_or(false)
    })
}

pub fn load_narrative(path: &Path, name: Option<&str>) -> Result<NarrativeSource> {
    let content = fs::read_to_string(path)?;
    
    match parse_narrative_toml(&content)? {
        ParseResult::Single(narrative) => {
            Ok(NarrativeSource::Single(narrative))
        }
        ParseResult::Multi(multi) => {
            let name = name.ok_or(Error::MultiNarrativeRequiresName)?;
            
            // Validate narrative exists
            let narrative = multi.get(name)?;
            
            // Check if composition is used
            if has_composition_acts(&narrative) {
                // Keep full context for composition resolution
                Ok(NarrativeSource::MultiWithContext {
                    multi,
                    execute_name: name.to_string(),
                })
            } else {
                // No composition, extract single narrative
                Ok(NarrativeSource::Single(narrative.clone()))
            }
        }
    }
}
```

### Phase 3: Update Executor (30 min)

**File**: `crates/botticelli_narrative/src/executor.rs`

Update composition act execution:

```rust
// When executing composition act
async fn execute_composition_act(
    &self,
    act: &Act,
    narrative_source: &NarrativeSource,
) -> Result<Output> {
    let referenced_name = act.narrative.as_ref()
        .ok_or(Error::MissingNarrativeReference)?;
    
    // Get the referenced narrative from context
    let referenced = match narrative_source.get_multi_context() {
        Some(multi) => multi.get(referenced_name)?,
        None => {
            return Err(Error::CompositionRequiresMultiNarrative {
                narrative_name: referenced_name.clone(),
            });
        }
    };
    
    // Execute the referenced narrative
    self.execute_narrative_impl(referenced, narrative_source).await
}
```

### Phase 4: Update Skill Interface (15 min)

**File**: `crates/botticelli_actor/src/skills/narrative_execution.rs`

Ensure the skill passes `NarrativeSource` through the call chain:

```rust
pub async fn execute(&self, context: &ExecutionContext) -> Result<SkillOutput> {
    let narrative_source = load_narrative(
        &self.config.narrative_path,
        self.config.narrative_name.as_deref(),
    )?;
    
    let executor = NarrativeExecutor::new(/* ... */);
    
    // Execute with full source context
    let result = executor.execute(&narrative_source).await?;
    
    Ok(SkillOutput::from(result))
}
```

### Phase 5: Add Tests (1 hour)

**File**: `crates/botticelli_narrative/tests/composition_loading_test.rs`

```rust
#[tokio::test]
async fn test_composition_loads_referenced_narratives() {
    let toml = r#"
        [narratives.main]
        name = "main"
        [narratives.main.toc]
        order = ["call_sub"]
        [narratives.main.acts.call_sub]
        narrative = "sub"
        
        [narratives.sub]
        name = "sub"
        [narratives.sub.toc]
        order = ["work"]
        [acts.work]
        model = "gemini-2.5-flash-lite"
        [[acts.work.input]]
        type = "text"
        content = "test"
    "#;
    
    let source = parse_and_load(toml, Some("main")).unwrap();
    
    match source {
        NarrativeSource::MultiWithContext { multi, execute_name } => {
            assert_eq!(execute_name, "main");
            assert!(multi.get("main").is_ok());
            assert!(multi.get("sub").is_ok());
        }
        _ => panic!("Expected MultiWithContext"),
    }
}

#[tokio::test]
async fn test_non_composition_narrative_extracts_single() {
    let toml = r#"
        [narratives.simple]
        name = "simple"
        [narratives.simple.toc]
        order = ["work"]
        [acts.work]
        model = "gemini-2.5-flash-lite"
        [[acts.work.input]]
        type = "text"
        content = "test"
    "#;
    
    let source = parse_and_load(toml, Some("simple")).unwrap();
    
    match source {
        NarrativeSource::Single(narrative) => {
            assert_eq!(narrative.name, "simple");
        }
        _ => panic!("Expected Single"),
    }
}
```

### Phase 6: Integration Test (30 min)

Test with actual `generation_carousel.toml`:

```bash
# Run actor server with generation actor
cargo run -p botticelli_actor --bin actor-server \
  --features otel-otlp,discord

# Should see:
# INFO: Loading narrative batch_generate
# INFO: Detected composition acts, keeping full context
# INFO: Executing act feature (composition)
# INFO: Found referenced narrative feature in context
# SUCCESS: Narrative completed
```

## Testing Strategy

### Unit Tests
- `has_composition_acts()` correctly identifies composition
- `NarrativeSource::get_multi_context()` returns context
- Single narratives don't include context
- Multi-narrative extraction preserves all narratives

### Integration Tests
- Load `generation_carousel.toml` with `batch_generate`
- Verify all 5 sub-narratives are accessible
- Execute composition act successfully
- Validate output from composed execution

### Regression Tests
- Single narrative files still work
- Multi-narrative files without composition still extract single
- Error messages are clear and actionable

## Timeline

- **Phase 1**: 30 min - Add NarrativeSource enum
- **Phase 2**: 45 min - Update loader logic
- **Phase 3**: 30 min - Update executor
- **Phase 4**: 15 min - Update skill interface
- **Phase 5**: 1 hour - Write comprehensive tests
- **Phase 6**: 30 min - Integration testing

**Total**: ~3 hours of focused implementation

## Success Criteria

✅ Actor-server starts without errors  
✅ Generation actor executes `batch_generate` successfully  
✅ All 5 composition acts resolve their references  
✅ Content is generated to `potential_discord_posts` table  
✅ No regression in existing functionality  
✅ Tests pass for composition and non-composition cases  

## Rollback Plan

If Option 1 proves problematic:
1. Revert code changes
2. Switch to Option 3: Remove `narrative_name` temporarily
3. Use single-narrative files for each sub-narrative
4. Update actor configs to use individual files

## Next Steps

1. **Immediate**: Implement Option 1 (3 hours)
2. **Validation**: Run full actor-server test (30 min)
3. **Documentation**: Update NARRATIVE_TOML_SPEC.md with composition loading behavior
4. **Commit**: "fix(narrative): Load full MultiNarrative context for composition acts"

---

**Priority**: Must complete before actor-server production deployment  
**Blocking**: All three actors (generation, curation, posting)  
**Impact**: HIGH - Core functionality of narrative composition system
