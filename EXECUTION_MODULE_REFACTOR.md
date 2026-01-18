# Execution Module Refactor Plan

## Current State

**File:** `crates/botticelli_mcp/src/rmcp_server/tools/execution.rs`
**Size:** 673 lines (TOO LONG - should be split)

### Functions

1. **`generate()`** (lines 36-201) - ~165 lines
   - Status: ✅ FULLY IMPLEMENTED
   - Has placeholder only for no-feature-enabled case (testing)
   - Proper driver selection and execution

2. **`generate_with_driver()`** (lines 203-278) - ~75 lines
   - Status: ✅ FULLY IMPLEMENTED
   - Helper function for generate()
   - Proper error handling and token tracking

3. **`execute_act()`** (lines 282-315) - ~33 lines
   - Status: ❌ PLACEHOLDER ONLY
   - Returns fake response
   - Comments say "Full implementation requires LLM backend integration"
   - **NEEDS IMPLEMENTATION**

4. **`execute_narrative()`** (lines 319-623) - ~304 lines (HUGE!)
   - Status: ✅ FULLY IMPLEMENTED (just completed)
   - Prompt injection and max_tokens override working
   - Driver selection and execution working
   - **NEEDS REFACTORING TO SPLIT**

5. **`create_narrative_session()`** (lines 625-673) - ~48 lines
   - Status: ✅ FULLY IMPLEMENTED
   - Uses NarrativeHelper and PartialNarrative
   - Proper session creation

## Problems

### Problem 1: execute_act is a Placeholder

**Current behavior:**
```rust
// Placeholder implementation
let response = format!(
    "Act execution placeholder\n\nPrompt: {}\nModel: {}\nContext: {}...",
    prompt, model, context
);
```

**What it should do:**
- Select LLM driver based on model parameter
- Build proper message with system prompt + context + user prompt
- Execute with driver (like generate_with_driver does)
- Return response with token usage
- Handle errors properly

**Why this exists:**
This function is for executing a SINGLE act with optional context. It's different from:
- `generate()` - simple text generation, no context
- `execute_narrative()` - executes full narrative from TOML

**Implementation approach:**
Can reuse `generate_with_driver()` helper - just needs to build the right prompt structure.

### Problem 2: execute_narrative is Too Long (304 lines)

**Issues:**
1. Massive driver selection block (~200 lines of repetitive code)
2. TOML parsing logic embedded in function
3. Prompt injection logic embedded
4. Max tokens override logic embedded
5. Result conversion closure embedded

**Needs splitting into:**

#### Helper Functions to Extract:

1. **`parse_narrative_toml(path: &Path) -> Result<(String, Narrative), rmcp::ErrorData>`**
   - Read file
   - Parse TOML to extract narrative names
   - Load MultiNarrative
   - Get specific narrative
   - Return (narrative_name, narrative)
   - Lines to extract: ~50-60 lines

2. **`apply_runtime_overrides(narrative: &mut Narrative, prompt: &str, max_tokens: u32)`**
   - Prompt injection logic
   - Max tokens override logic
   - All the instrumentation
   - Lines to extract: ~70-80 lines

3. **`select_driver_and_execute(...) -> Result<NarrativeExecution, ...>`**
   - Driver selection based on model string
   - Create executor
   - Execute narrative
   - Return execution
   - Lines to extract: ~150-180 lines

4. **`convert_execution_result(execution: NarrativeExecution) -> ExecuteNarrativeResult`**
   - Extract final output
   - Extract models used
   - Calculate token totals
   - Build result
   - Lines to extract: ~20-30 lines

**After refactor, execute_narrative should be:**
```rust
pub async fn execute_narrative(...) -> Result<Json<ExecuteNarrativeResult>, rmcp::ErrorData> {
    // Parse parameters
    let (narrative_name, mut narrative) = parse_narrative_toml(&path)?;
    
    // Apply overrides
    apply_runtime_overrides(&mut narrative, prompt, max_tokens);
    
    // Execute
    let execution = select_driver_and_execute(self, &narrative, model).await?;
    
    // Convert result
    Ok(Json(convert_execution_result(execution)))
}
```

Target: ~30-40 lines for main function

## Refactor Plan

### Phase 1: Split execution.rs into Separate Modules

**New structure:**
```
src/rmcp_server/tools/execution/
├── mod.rs              # Public API and top-level functions
├── generation.rs       # generate() and generate_with_driver()
├── acts.rs             # execute_act() - TO BE IMPLEMENTED
├── narratives.rs       # execute_narrative() and helpers
├── sessions.rs         # create_narrative_session()
└── helpers.rs          # Shared utilities (driver selection, etc.)
```

**Benefits:**
- Each file ~100-150 lines (manageable)
- Clear separation of concerns
- Easier to understand and maintain
- Tests can target specific modules

### Phase 2: Implement execute_act()

**Requirements:**
- Accept ExecuteActParams (prompt, model, max_tokens, context)
- Select driver based on model (reuse pattern from generate)
- Build message:
  - System prompt: "You are executing a narrative act. Be concise and relevant."
  - Context: If provided, prepend as "Context from previous acts: {context}"
  - User prompt: The actual prompt
- Execute with driver
- Return ExecuteActResult with response and metadata

**Reuse:**
- Can adapt generate_with_driver() or create similar helper
- Driver selection logic should be shared (extract to helpers.rs)

**Testing:**
- Similar to generate() tests
- Add test with context
- Add test without context

### Phase 3: Refactor execute_narrative() Helpers

**Extract helpers in narratives.rs:**
- parse_narrative_toml()
- apply_runtime_overrides()
- select_driver_and_execute()
- convert_execution_result()

**Instrument all helpers** - each gets proper tracing

**Main function becomes clean orchestration**

### Phase 4: Extract Common Driver Selection Logic

**Problem:** Driver selection is copy-pasted in:
- generate()
- execute_act() (will be)
- execute_narrative()

**Solution:** Create in helpers.rs:
```rust
pub(super) async fn select_and_execute_with_driver<F, Fut, T>(
    server: &BotticelliServer,
    model_str: &str,
    executor: F,
) -> Result<T, rmcp::ErrorData>
where
    F: FnOnce(Arc<dyn BotticelliDriver>) -> Fut,
    Fut: Future<Output = Result<T, rmcp::ErrorData>>,
{
    // Check model prefix and get appropriate driver
    // Call executor function with driver
    // Return result
}
```

This eliminates the massive if/else chains.

## Implementation Order

1. **Phase 1: Module split**
   - Create execution/ directory
   - Move functions to separate files
   - Update mod.rs exports
   - Verify compilation

2. **Phase 2: Implement execute_act()**
   - Write proper implementation
   - Add tests
   - Remove placeholder

3. **Phase 3: Extract execute_narrative helpers**
   - Extract parse_narrative_toml()
   - Extract apply_runtime_overrides()
   - Extract convert_execution_result()
   - Extract driver selection
   - Refactor main function to use helpers

4. **Phase 4: Extract common driver selection**
   - Create select_and_execute_with_driver()
   - Refactor generate() to use it
   - Refactor execute_act() to use it
   - Refactor execute_narrative() to use it

5. **Phase 5: Testing and cleanup**
   - Update all tests
   - Add missing tests
   - Verify instrumentation
   - Clean up any warnings

## Module Organization

### mod.rs (Public API)
```rust
//! Narrative execution tools.

mod acts;
mod generation;
mod helpers;
mod narratives;
mod sessions;

pub use acts::*;
pub use generation::*;
pub use narratives::*;
pub use sessions::*;
```

### generation.rs
- generate()
- generate_with_driver()
- ~150-180 lines

### acts.rs
- execute_act()
- act-specific helpers if needed
- ~80-100 lines

### narratives.rs
- execute_narrative() (main function, ~30-40 lines)
- parse_narrative_toml() (~60 lines)
- apply_runtime_overrides() (~80 lines)
- convert_execution_result() (~30 lines)
- Total: ~200-210 lines

### sessions.rs
- create_narrative_session()
- ~50 lines

### helpers.rs
- select_and_execute_with_driver()
- Shared utilities
- ~80-100 lines

## Success Criteria

- [ ] No file exceeds 250 lines
- [ ] All placeholders removed
- [ ] All functions properly implemented
- [ ] All tests passing
- [ ] Full instrumentation on all functions
- [ ] No code duplication (DRY)
- [ ] Clear separation of concerns

## Next Steps

1. Create EXECUTION_MODULE_REFACTOR.md (this document) ✅
2. Get approval on approach
3. Start with Phase 1 (module split)
4. Proceed through phases sequentially
5. Test after each phase
6. Commit after each phase

## Notes

### Why Not One Giant Refactor?
- Too risky (easy to break things)
- Harder to review
- Difficult to rollback if needed
- Phases allow testing at each step

### Why Keep execute_narrative Complex Logic?
The logic itself (prompt injection, max_tokens override) is GOOD and CORRECT.
We just need to extract it into well-named functions for readability.

### Test Strategy
- Move existing tests to match new module structure
- Add tests for new helper functions
- Add tests for execute_act() implementation
- Verify all existing functionality still works
