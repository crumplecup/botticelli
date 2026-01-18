# Elicitation Refactor Guide

## Current State Analysis

### Problem: Dual Implementations

We have **two separate elicitation systems** that need to be unified:

1. **Legacy PMCP system** (`src/tools/elicitation/`): 611 lines
   - `helpers.rs` (218 lines) - Helper utilities
   - `registry.rs` (248 lines) - Narrative registry
   - `state.rs` (50 lines) - State management
   - `validation.rs` (76 lines) - Validation
   - Uses old `pmcp` protocol

2. **New RMCP system** (`src/rmcp_server/tools/elicitation.rs`): 367 lines
   - Basic tools: elicit_text, elicit_bool, elicit_number, elicit_select
   - More advanced: elicit_act, elicit_metadata, elicit_carousel
   - Uses new `rmcp` protocol
   - Uses DialogResource for interaction

3. **Standalone parameter types** (`src/elicit_*.rs`): 4 files (3.6KB)
   - Parameter/Result structs for each elicitation type
   - Should be consolidated with proper organization

### The elicitation Crate (0.2.0)

The `elicitation` crate provides a trait-based system for conversational value elicitation:

**Core Traits:**
- `Elicitation` - Main entry point trait (has `elicit()` method)
- `Prompt` - Provides prompt metadata
- `Select` - Choose from finite options (enum pattern)
- `Affirm` - Yes/no confirmation (bool pattern)  
- `Survey` - Multi-field elicitation (struct pattern)
- `Authorize` - Permission policies (planned)

**Derive Macros:**
- `#[derive(Elicit)]` on enums → implements Select
- `#[derive(Elicit)]` on structs → implements Survey
- `#[prompt("...")]` attribute for custom prompts

**Integration:**
```rust
use elicitation::{Elicitation, ElicitResult};
use rmcp::service::{Peer, RoleClient};

async fn example(client: &Peer<RoleClient>) -> ElicitResult<()> {
    let age: i32 = i32::elicit(client).await?;
    let name: Option<String> = Option::<String>::elicit(client).await?;
    let scores: Vec<i32> = Vec::<i32>::elicit(client).await?;
    Ok(())
}
```

### Key Insight: Program Against Traits

> **The key to making it work is programming our interface against the elicitation traits, so that something happens when the LLM needs more context, an elicitation dialog pops up instead of the normal chat, etc.**

This means:
1. Our types should implement the `elicitation` traits
2. Our interfaces should accept `impl Elicitation` trait objects
3. When execution needs input, it calls `.elicit()` on the trait
4. The elicitation system handles the MCP interaction

## Architecture Vision

### Phase 1: Understand Current Usage

**Audit elicitation usage across codebase:**
- Where do we call elicit_text, elicit_bool, etc?
- What types are being elicited?
- What custom types need Elicit derives?
- What narrative-specific elicitation logic exists?

### Phase 2: Implement Elicit Derives

**Add derives to our custom types:**
```rust
use elicitation::Elicit;

#[derive(Debug, Clone, Elicit)]
pub enum Role {
    #[prompt("System message (instructions)")]
    System,
    #[prompt("User input")]
    User,
    #[prompt("Assistant response")]
    Assistant,
}

#[derive(Debug, Clone, Elicit)]
pub struct ActConfig {
    #[prompt("Act name (e.g., 'introduction', 'analysis')")]
    name: String,
    
    #[prompt("User prompt for this act")]
    prompt: Option<String>,
    
    #[prompt("Maximum tokens for response")]
    max_tokens: Option<u32>,
    
    #[prompt("Model to use (e.g., 'gemini-pro')")]
    model: Option<String>,
}
```

### Phase 3: Replace Direct MCP Calls with Trait Calls

**Before (direct MCP):**
```rust
pub async fn elicit_text(
    &self,
    Parameters(params): Parameters<ElicitTextParams>,
) -> Result<Json<ElicitTextResult>, rmcp::ErrorData> {
    let dialog = self.dialog().as_ref().ok_or(...)?;
    let text = dialog.ask_text(&params.prompt()).await?;
    Ok(Json(ElicitTextResult::new(text)))
}
```

**After (trait-based):**
```rust
pub async fn elicit<T: Elicitation>(
    &self,
    client: &Peer<RoleClient>,
) -> ElicitResult<T> {
    T::elicit(client).await
}
```

### Phase 4: Narrative Integration

**Connect narrative execution to elicitation:**
```rust
use elicitation::Elicitation;

pub struct NarrativeExecutor<BE> {
    driver: Arc<dyn ExecutionDriver<GenerateRequest, GenerateResponse>>,
    elicitor: Option<Arc<Peer<RoleClient>>>,  // MCP client for elicitation
    _error: PhantomData<BE>,
}

impl<BE> NarrativeExecutor<BE>
where
    BE: std::error::Error + Send + Sync + 'static,
{
    /// Execute with elicitation support.
    #[instrument(skip(self, source, elicitor))]
    pub async fn execute_with_elicitation(
        &self,
        source: &NarrativeSource,
        elicitor: &Peer<RoleClient>,
    ) -> Result<NarrativeExecution, BE> {
        // When narrative needs input, call:
        let user_input: String = String::elicit(elicitor).await
            .map_err(|e| /* convert to BE */)?;
        
        // Continue execution...
    }
}
```

### Phase 5: Clean Up Legacy

**Remove after migration:**
- `src/tools/elicitation/` (entire legacy directory)
- `src/elicit_*.rs` (parameter types - replaced by trait methods)
- `src/dialog_resource.rs` (replaced by trait-based elicitation)
- Old MCP handlers in `src/rmcp_server/tools/elicitation.rs`

## Implementation Strategy

### Step 1: Audit Current Usage

**Actions:**
1. Grep for all elicit_* function calls
2. Document what types are being elicited
3. Identify custom types needing derives
4. Map out data flow from MCP → execution

**Files to check:**
- `src/rmcp_server/tools/execution/*.rs`
- `src/tools/narrative_creation.rs`
- `tests/elicit_*_test.rs`

### Step 2: Add Elicit Derives

**Actions:**
1. Add `#[derive(Elicit)]` to core types
2. Add `#[prompt("...")]` attributes for user-facing messages
3. Implement Elicit for Option<T>, Vec<T> wrappers if needed
4. Test derives with simple examples

**Types to update:**
- `botticelli_core::Role`
- `botticelli_core::Input`
- `botticelli_core::Output`
- `botticelli_narrative::ActConfig`
- `botticelli_narrative::CarouselConfig`
- Custom MCP parameter types

### Step 3: Create Trait-Based Wrappers

**Actions:**
1. Create new `src/rmcp_server/elicitation.rs` module
2. Implement generic elicitation handlers using traits
3. Bridge rmcp::ErrorData ↔ elicitation::ElicitError
4. Add instrumentation to all elicitation paths

**Example:**
```rust
use elicitation::{Elicitation, ElicitResult};
use rmcp::service::{Peer, RoleClient};
use tracing::{debug, error, instrument};

#[instrument(skip(client), fields(type_name = std::any::type_name::<T>()))]
pub async fn elicit_value<T>(
    client: &Peer<RoleClient>,
) -> Result<T, rmcp::ErrorData>
where
    T: Elicitation + std::fmt::Debug,
{
    debug!(type_name = std::any::type_name::<T>(), "Eliciting value");
    
    T::elicit(client)
        .await
        .map_err(|e| {
            error!(error = %e, "Elicitation failed");
            // Convert ElicitError → rmcp::ErrorData
            convert_elicit_error(e)
        })
}
```

### Step 4: Update Call Sites

**Actions:**
1. Replace `elicit_text()` → `String::elicit()`
2. Replace `elicit_bool()` → `bool::elicit()`
3. Replace `elicit_number()` → `i32::elicit()` / `u32::elicit()`
4. Replace `elicit_select()` → `MyEnum::elicit()`
5. Update tests to use trait-based API

**Migration pattern:**
```rust
// Before:
let params = ElicitTextParams::new("Enter name:");
let result = server.elicit_text(Parameters(params)).await?;
let name = result.value();

// After:
use elicitation::Elicitation;
let name: String = String::elicit(&client).await?;
```

### Step 5: Test Migration

**Actions:**
1. Update all `tests/elicit_*_test.rs` files
2. Verify elicitation works with narrative execution
3. Test derive macros on custom types
4. Ensure proper error propagation

### Step 6: Remove Legacy Code

**Actions:**
1. Delete `src/tools/elicitation/` directory
2. Delete `src/elicit_*.rs` files
3. Delete `src/dialog_resource.rs`
4. Remove old handlers from `src/rmcp_server/tools/elicitation.rs`
5. Clean up unused imports
6. Update module structure

## Benefits

### Code Reduction
- **Before:** 978+ lines across multiple implementations
- **After:** ~100 lines of trait-based wrappers + derives on types

### Type Safety
- Compile-time guarantees for elicitable types
- No runtime string-based type checking
- Clear trait bounds in function signatures

### Flexibility
- New types automatically elicitable with `#[derive(Elicit)]`
- Compose elicitation (Option<T>, Vec<T>, nested structs)
- Single implementation point for elicitation logic

### Maintainability
- One trait system, not two MCP implementations
- Clear separation: types define elicitation, executors use it
- Standard patterns from elicitation crate

### Integration with Architecture
- Trait objects fit our interface-based design
- Works seamlessly with ExecutionDriver pattern
- No concrete type dependencies in high-level code

## Error Handling

### Error Conversion Chain

```rust
// Elicitation errors are Serfs
elicitation::ElicitError 
    → rmcp::ErrorData (protocol boundary)
    → McpError (our Serf)
    → BotticelliError (Royalty)
```

**Proper handling:**
```rust
use elicitation::Elicitation;

let value: MyType = MyType::elicit(client)
    .await
    .map_err(|e| {
        // Capture source error, don't cast to string
        McpError::elicitation_failed(
            format!("Failed to elicit {}", std::any::type_name::<MyType>()),
            e,  // source: elicitation::ElicitError
        )
    })?;
```

## Testing Strategy

### Unit Tests
- Test derives on simple types (enum, struct)
- Test Option<T>, Vec<T> wrappers
- Test prompt attributes

### Integration Tests
- Test narrative creation via elicitation
- Test error propagation
- Test with real MCP client (if available)

### Migration Tests
- Keep old tests during migration
- Add parallel trait-based tests
- Compare outputs for compatibility
- Remove old tests after migration confirmed

## Open Questions

1. **MCP Client Access:** How do we pass `Peer<RoleClient>` to execution layer?
   - Option A: Add to BotticelliServer struct
   - Option B: Thread through execution context
   - Option C: Separate elicitation service

2. **Async Boundaries:** How do we handle elicitation in sync contexts?
   - Most of our code is already async
   - Execution layer is async
   - Should be fine with async/.await

3. **Feature Gates:** Should elicitation be feature-gated?
   - Probably yes, some deployments don't need interactive mode
   - Feature: `elicitation` (default = true?)

4. **Backwards Compatibility:** Do we need migration period?
   - Internal crate, no external API consumers
   - Clean break is acceptable
   - Update all call sites atomically

## Success Criteria

- [ ] All custom types have `#[derive(Elicit)]`
- [ ] Zero direct MCP elicitation calls (all via traits)
- [ ] Legacy elicitation code deleted
- [ ] All tests passing with trait-based API
- [ ] Full instrumentation on elicitation paths
- [ ] Proper error source tracking (no string conversion)
- [ ] Documentation updated to show trait usage
- [ ] check-features passes with elicitation feature
- [ ] Narrative execution can elicit values mid-execution

## Timeline Estimate

- **Phase 1 (Audit):** 30 minutes
- **Phase 2 (Derives):** 1-2 hours
- **Phase 3 (Wrappers):** 1 hour
- **Phase 4 (Call Sites):** 2-3 hours
- **Phase 5 (Tests):** 1-2 hours
- **Phase 6 (Cleanup):** 30 minutes

**Total:** 6-9 hours for complete migration

## Next Steps

1. Start with Phase 1 audit
2. Document all elicitation call sites
3. Identify types needing derives
4. Create derives + test in isolation
5. Build trait-based wrappers
6. Migrate call sites one by one
7. Update tests
8. Remove legacy code
9. Final verification

---

**Key Principle:** Program against the `elicitation` traits, not concrete MCP handlers. Our types implement the traits, our code calls `.elicit()`, the library handles the rest.
