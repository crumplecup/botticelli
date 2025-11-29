# Inline Test Module Cleanup

## Issue

We have inline `#[cfg(test)] mod tests` blocks in 14 source files, violating CLAUDE.md policy that all tests must be in `tests/` directories.

## Violations by Crate

### botticelli_server (1 file)
- `src/schedule.rs`

### botticelli_core (1 file)
- `src/budget.rs`

### botticelli_database (3 files)
- `src/schema_docs.rs`
- `src/schema_inference.rs`
- `src/schema_reflection.rs`

### botticelli_models (4 files)
- `tests/test_utils/mock_gemini.rs` (acceptable - already in tests/)
- `src/gemini/live_client.rs`
- `src/gemini/live_rate_limit.rs`
- `src/gemini/live_protocol.rs`

### botticelli_social (1 file)
- `src/database/commands.rs`

### botticelli_narrative (5 files)
- `src/table_reference.rs`
- `src/state.rs`
- `src/processor.rs`
- `src/extraction.rs`
- `src/core.rs`

## Migration Strategy

For each file:

1. **Extract tests** - Move test functions to `tests/{module}_test.rs`
2. **Public API** - Make tested items `pub` or `pub(crate)` as needed
3. **Test helpers** - Move to `tests/test_utils/`
4. **Verify** - Run `just check-all` to ensure no regressions

## Benefits

- ✅ Centralized test location
- ✅ Cleaner source files
- ✅ No feature gate confusion
- ✅ Better CI organization
- ✅ Follows CLAUDE.md standards

## Status

**In Progress** - Migrating inline test modules

### Completed
- ✅ `botticelli_core/src/budget.rs` → `tests/budget_test.rs` (verified passing)

### Remaining
- ⏳ `botticelli_database/src/schema_docs.rs`
- ⏳ `botticelli_database/src/schema_inference.rs`
- ⏳ `botticelli_database/src/schema_reflection.rs`
- ⏳ `botticelli_models/src/gemini/live_client.rs`
- ⏳ `botticelli_models/src/gemini/live_rate_limit.rs`
- ⏳ `botticelli_models/src/gemini/live_protocol.rs`
- ⏳ `botticelli_server/src/schedule.rs`
- ⏳ `botticelli_social/src/database/commands.rs`
- ⏳ `botticelli_narrative/src/table_reference.rs`
- ⏳ `botticelli_narrative/src/state.rs`
- ⏳ `botticelli_narrative/src/processor.rs`
- ⏳ `botticelli_narrative/src/extraction.rs`
- ⏳ `botticelli_narrative/src/core.rs` (test helper method)
