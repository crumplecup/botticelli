# Inline Test Module Cleanup

## Overview

Per CLAUDE.md guidelines, all tests must be in `tests/` directories at crate roots, not inline `#[cfg(test)] mod tests` within source files.

## Status

### Completed
- ✅ botticelli_core/src/rate_limit.rs → tests/rate_limit_test.rs (already done)
- ✅ botticelli_server/src/schedule.rs → tests/schedule_test.rs

### Remaining

**botticelli_database:**
- `src/schema_docs.rs` - mod tests
- `src/schema_reflection.rs` - mod tests  
- `src/schema_inference.rs` - mod tests

**botticelli_models:**
- `src/gemini/live_protocol.rs` - mod tests
- `src/gemini/live_rate_limit.rs` - mod tests
- `src/gemini/live_client.rs` - mod tests

**botticelli_narrative:**
- `src/extraction.rs` - mod tests
- `src/processor.rs` - mod tests
- `src/state.rs` - mod tests
- `src/table_reference.rs` - mod tests

**botticelli_social:**
- `src/database/commands.rs` - mod tests

## Process

For each file:
1. Extract test module to `tests/{module}_test.rs`
2. Add necessary imports (use crate-level exports)
3. Remove `#[cfg(test)] mod tests` from source
4. Verify with `just check {package}`
5. Commit with message: `refactor(tests): move {module} tests to tests/ directory`

## Benefits

- Cleaner source files (no test clutter)
- Centralized test organization
- Easier to find and maintain tests
- Follows Rust best practices
- Enforces crate-level API usage in tests
