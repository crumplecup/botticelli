# Narrative Crate Audit Checklist

## 1. Derive Usage Issues

### Manual Getters (Use derive_getters)
- [ ] `ActorConfig` in `src/actor_config.rs` - has manual getters
- [ ] `CarouselConfig` in `src/carousel.rs` - has manual getters
- [ ] `NarrativeConfig` in `src/config.rs` - has manual getters
- [ ] `LoopConfig` in `src/config.rs` - has manual getters
- [ ] `ActorState` in `src/executor.rs` - has manual getters
- [ ] `ExecutionState` in `src/executor.rs` - has manual getters

### Manual Setters (Use derive_setters with prefix = "with_")
- [ ] `CarouselConfig` in `src/carousel.rs` - has manual setters
- [ ] `NarrativeConfig` in `src/config.rs` - has manual setters
- [ ] `LoopConfig` in `src/config.rs` - has manual setters

### Manual Constructors (Use derive_new where simple)
- [ ] Review all `impl Type { pub fn new(...) }` blocks
- [ ] Convert simple cases to derive_new
- [ ] Keep complex cases (validation, resources) as manual

## 2. Error Handling Issues

### Uses of `.expect()`
- [ ] Search for all `.expect()` calls
- [ ] Replace with proper error propagation using `?`
- [ ] Ensure all functions return appropriate Result types

### Uses of `.unwrap()`
- [ ] Search for all `.unwrap()` calls
- [ ] Replace with proper error propagation using `?`
- [ ] Ensure all functions return appropriate Result types

### Manual Error Conversions
- [ ] Look for `Error::new(ErrorKind::Variant(...))` patterns
- [ ] Simplify to use `ErrorKind::Variant(...).into()` with bridge macros
- [ ] Check if all error types have bridge macros in _error crate

### External Error Capture
- [ ] Review all `.map_err(|e| ...::new(...(e.to_string())))` patterns
- [ ] Ensure external errors are captured in Arc/Box, not converted to String
- [ ] Add missing error variants to capture specific error types

## 3. Observability Issues

### Missing #[instrument]
- [ ] `src/actor_config.rs` - add to all public functions
- [ ] `src/carousel.rs` - add to all public functions
- [ ] `src/config.rs` - add to all public functions
- [ ] `src/context.rs` - add to all public functions
- [ ] `src/executor.rs` - add to all public functions
- [ ] `src/loader.rs` - add to all public functions
- [ ] `src/parser.rs` - add to all public functions
- [ ] `src/sampler.rs` - add to all public functions
- [ ] `src/types.rs` - add to all public functions

### Missing Debug Logging
- [ ] Add `debug!()` at function entry for complex operations
- [ ] Add `debug!()` for state changes
- [ ] Add `error!()` before returning errors
- [ ] Add structured fields for IDs, counts, etc.

## 4. Public Field Issues

### Structs with pub fields (should have private + getters)
- [ ] Review all structs in the crate
- [ ] Make fields private
- [ ] Add derive_getters
- [ ] Add derive_setters where appropriate

## 5. Import Issues

### Re-exports from other workspace crates
- [ ] Check lib.rs for any re-exports from botticelli_error
- [ ] Check lib.rs for any re-exports from botticelli_core
- [ ] Check lib.rs for any re-exports from botticelli_models
- [ ] Remove all cross-crate re-exports

### Workspace dependency imports
- [ ] Already fixed in Cargo.toml

## 6. Test Issues

### Tests in source files (#[cfg(test)])
- [ ] Check all source files for inline test modules
- [ ] Move to tests/ directory

### Tests using .expect() or .unwrap()
- [ ] Find all test functions
- [ ] Ensure they return anyhow::Result<()> or specific error types
- [ ] Replace .expect()/.unwrap() with ?
- [ ] Use our library error types, not Box<dyn Error>

### Tests missing tracing setup
- [ ] Add init_tracing() helper
- [ ] Call at start of each test

## 7. Code Quality Issues

### Large functions that should be split
- [ ] Review functions >100 lines
- [ ] Extract helper methods
- [ ] Improve readability

### Missing documentation
- [ ] All public items need ///
- [ ] Complex internal items should have //

## Progress Tracking

**Current Status:** Starting audit
**Last Updated:** 2026-01-05
**Completion:** 0/60+ items

## Next Steps

1. Start with derive usage (section 1)
2. Fix error handling (section 2)
3. Add observability (section 3)
4. Fix public fields (section 4)
5. Clean up imports (section 5)
6. Fix tests (section 6)
7. Improve code quality (section 7)
