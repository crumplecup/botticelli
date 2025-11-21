# Sequential Execution Design

## Problem Statement

Currently, the narrative executor processes acts sequentially in a loop, but nested narrative execution and carousel parallelism are not properly implemented. Acts must complete fully before the next act begins, and carousel iterations can be parallelized but must complete before the carousel act finishes.

## Current Issues

1. **Nested narratives** - When an act has a nested narrative input, it's being processed but not awaited
2. **Carousel parallelism** - Carousel acts don't actually spawn parallel iterations
3. **Act sequencing** - The executor loop doesn't guarantee completion before proceeding

## Architecture Requirements

### 1. Act Execution is Strictly Sequential

```
Act 1 → [complete] → Act 2 → [complete] → Act 3 → [complete]
```

- No act begins until the previous act has fully completed
- "Completed" means all processing, all nested narratives, all carousel iterations finished

### 2. Carousel Iterations are Embarrassingly Parallel

```
Carousel Act (3 iterations):
  ├─ Iteration 1 ─┐
  ├─ Iteration 2 ─┤→ [all complete] → Next Act
  └─ Iteration 3 ─┘
```

- All iterations spawn concurrently (using rayon or tokio)
- Carousel act completes only when all iterations complete
- Budget tracking must be thread-safe

### 3. Nested Narratives Block Until Complete

```
Act 1 (nested narrative):
  └─ Execute nested.toml → [complete] → Act 1 done
```

- Nested narrative is recursively executed via the executor
- Parent act waits for nested narrative completion
- Results/tables from nested narrative are available to subsequent acts

## Implementation Strategy

### Phase 1: Make Acts Truly Sequential

**Current code location:** `crates/botticelli_narrative/src/executor.rs`

The main loop in `execute()` needs to:
1. Process each act completely before advancing to the next
2. For each act, process all inputs sequentially
3. Wait for generation backend response
4. Store results in database (if configured)
5. Only then proceed to `current_index += 1`

**Changes needed:**
- Ensure all async operations are properly awaited
- No early returns that skip completion
- Clear logging at act boundaries

### Phase 2: Implement Nested Narrative Execution

**Location:** `process_inputs()` in executor.rs

When encountering a nested narrative input:
```rust
NarrativeInput::Nested { path } => {
    info!("Executing nested narrative: {}", path);
    
    // Parse nested narrative
    let nested = Narrative::from_file(path)?;
    
    // Recursively execute using same executor context
    // (shares connection, backend, bot_registry, etc.)
    let nested_executor = NarrativeExecutor::new(
        self.backend.clone(),
        self.connection.as_mut(),
        self.content_repo.as_mut(),
        self.bot_registry.as_ref(),
    );
    
    // BLOCKING: Wait for nested narrative to complete
    nested_executor.execute(&nested)?;
    
    info!("Nested narrative completed: {}", path);
}
```

### Phase 3: Implement Carousel Parallelism

**Location:** New carousel execution logic in executor.rs

```rust
fn execute_carousel_act(
    &mut self,
    carousel: &CarouselConfig,
    act: &Act,
) -> NarrativeResult<()> {
    info!("Executing carousel: {} iterations", carousel.iterations());
    
    // Use rayon for embarrassingly parallel execution
    let results: Vec<NarrativeResult<()>> = (0..carousel.iterations())
        .into_par_iter()  // rayon parallel iterator
        .map(|iteration| {
            debug!("Carousel iteration {}", iteration);
            
            // Each iteration gets its own executor context
            // but shares budget tracker (thread-safe)
            self.execute_single_iteration(act, iteration)
        })
        .collect();
    
    // Check if any iteration failed
    for (idx, result) in results.into_iter().enumerate() {
        result.map_err(|e| {
            NarrativeError::new(
                NarrativeErrorKind::CarouselIterationFailed(idx, e.to_string())
            )
        })?;
    }
    
    info!("Carousel completed all iterations");
    Ok(())
}
```

**Budget tracking consideration:**
- `BudgetTracker` must be wrapped in `Arc<Mutex<BudgetTracker>>` for thread safety
- Each iteration checks budget before generating
- Budget updates are atomic

### Phase 4: Budget Thread Safety

**Location:** `crates/botticelli_narrative/src/carousel/budget.rs`

```rust
use std::sync::{Arc, Mutex};

pub struct ThreadSafeBudgetTracker {
    inner: Arc<Mutex<BudgetTracker>>,
}

impl ThreadSafeBudgetTracker {
    pub fn check_budget(&self, tokens: u64, requests: u64) -> bool {
        let tracker = self.inner.lock().unwrap();
        tracker.can_proceed(tokens, requests)
    }
    
    pub fn record_usage(&self, tokens: u64, requests: u64) {
        let mut tracker = self.inner.lock().unwrap();
        tracker.record(tokens, requests);
    }
}
```

## Testing Strategy

### Test 1: Sequential Acts
```toml
[[act]]
name = "act1"
[[act.prompt]]
text = "Count to 3"

[[act]]
name = "act2"
[[act.prompt]]
text = "Count to 3"
```

**Verify:** Act 2 starts only after Act 1 completes (check logs)

### Test 2: Nested Narratives
```toml
[[act]]
name = "run_nested"
[[act.prompt]]
narrative = "nested.toml"

[[act]]
name = "use_nested_results"
[[act.prompt]]
table = "nested_table"
```

**Verify:** Second act can reference table created by nested narrative

### Test 3: Carousel Parallelism
```toml
[carousel]
iterations = 5

[[act]]
name = "parallel_act"
[[act.prompt]]
text = "Generate content"
```

**Verify:** 
- 5 iterations run concurrently (check logs show overlap)
- Next act waits for all 5 to complete
- Budget tracking works correctly

## Migration Path

1. **Phase 1:** Audit and fix sequential execution (no breaking changes)
2. **Phase 2:** Implement nested narrative execution (enables new feature)
3. **Phase 3:** Implement carousel parallelism (performance improvement)
4. **Phase 4:** Make budget tracker thread-safe (required for Phase 3)

## Open Questions

1. **Database connection pooling:** Currently using `&mut PgConnection`. For parallel execution, need connection pool (r2d2 or deadpool)
2. **Backend concurrency:** Is `GenerationBackend` thread-safe? May need `Arc<Backend>` for cloning
3. **Error handling:** If one carousel iteration fails, should we cancel others or let them complete?
4. **Progress reporting:** How to report progress during long carousel runs?

## Success Criteria

- ✅ Acts execute strictly sequentially
- ✅ Nested narratives block parent act until complete
- ✅ Carousel iterations run in parallel
- ✅ All carousel iterations complete before next act
- ✅ Budget tracking is thread-safe
- ✅ Tests demonstrate correct behavior
- ✅ Zero race conditions or data races
