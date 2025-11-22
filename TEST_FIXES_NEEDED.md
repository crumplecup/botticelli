# Test Fixes Needed

## Summary

Tests are currently failing due to API changes. This document tracks what needs to be fixed.

**Progress Update:** Discord tests are fixed. Gemini tests partially fixed but need more work (see details below).

## Issues

### 1. Gemini Model Tests - Struct Literal Usage

**Files affected:**
- `crates/botticelli_models/tests/gemini_2_0_models_test.rs` - PARTIALLY FIXED
- `crates/botticelli_models/tests/gemini_streaming_test.rs`
- `crates/botticelli_models/tests/gemini.rs`
- `crates/botticelli_models/tests/gemini_model_test.rs`
- `crates/botticelli_models/tests/gemini_mock_test.rs`
- `crates/botticelli_models/tests/gemini_live_integration_test.rs`
- `crates/botticelli_models/tests/gemini_live_error_test.rs`

**Problem:** Tests use struct literal construction for `GenerateRequest` which now has private fields.

**Solution:** Replace all `GenerateRequest { ... }` with calls to `test_utils::create_test_request()` helper.

**Status:** In progress - duplicate imports removed, but new issues found:
1. Sed command created malformed syntax `Some("model", Some(10))` - needs manual fix
2. `Message::new()` doesn't exist - need to use builder or struct literal with public fields
3. `GenerateRequest` fields are private - tests need to use getters instead of direct field access

### 2. Discord Test API Changes

**Files affected:**
- `crates/botticelli/tests/discord_commands_test.rs`
- `crates/botticelli/tests/discord_bot_commands_test.rs`

**Problems:**
1. `NarrativeExecutor::builder()` no longer exists - use `NarrativeExecutor::new()` or `with_processors()`
2. `Narrative::from_file_with_db()` no longer exists - use `Narrative::from_file()`
3. Missing generic type parameter for `NarrativeExecutor<D>`

**Solution:** Update tests to use current API.

**Status:** FIXED - Updated to use `NarrativeExecutor::new()` and `with_bot_registry()`.

### 3. Documentation Comment Errors

**Files affected:**
- `crates/botticelli_models/tests/gemini_live_basic_test.rs`

**Problem:** Module-level doc comments (`//!`) appearing after attributes cause E0753 errors.

**Solution:** Move module doc comments to the very top of the file, before any attributes.

**Status:** FIXED - gemini_live_basic_test.rs doc comments moved before attributes.

## Testing Strategy

Once fixed, tests should be run with:

```bash
# Discord tests only
cargo test --features discord --test discord_bot_commands_test --test discord_commands_test

# Gemini model tests  
cargo test --features gemini,api --package botticelli_models

# All tests
just test-local
```

## Priority

1. **High:** Fix Discord tests - these are actively being used for development
2. **Medium:** Fix Gemini model tests - needed for model validation
3. **Low:** Documentation comment errors - cosmetic but should be fixed

## Notes

- The build succeeds with `--features discord`, only test compilation fails
- Test utility `create_test_request()` exists in `crates/botticelli_models/tests/test_utils/mod.rs`
- Builder pattern should be preferred over struct literals per CLAUDE.md
