# Testing Patterns and Best Practices

This document captures the lessons learned and established patterns for writing tests in the Botticelli project.

**For Narrative TOML syntax patterns, see `AI_NARRATIVE_TOML_GUIDE.md`**

## Core Principles

1. **Tests are first-class code** - They deserve the same care and attention as production code
2. **No technical debt in tests** - Failing tests must be fixed, not ignored
3. **Use proper error handling** - Tests should exercise our error types
4. **Follow codebase conventions** - Tests follow the same patterns as production code

## Error Handling in Tests

### ❌ WRONG: Using .unwrap(), .expect(), or anyhow

```rust
#[test]
fn test_something() {
    let result = create_something().unwrap(); // BAD
    let value = result.field().expect("failed"); // BAD
}

#[test]
fn test_with_anyhow() -> anyhow::Result<()> { // ALSO BAD
    Ok(())
}
```

### ✅ CORRECT: Use native Botticelli Result types

```rust
#[test]
fn test_something() -> BotticelliResult<()> {
    let result = create_something()?;
    let value = result.field();
    Ok(())
}
```

**Why:** 
- Using native Result types exercises our error framework
- Never use anyhow in tests - only native library error types
- Provides better error messages with full context

## Builder Pattern Usage

### ❌ WRONG: Using struct literals

```rust
let request = GenerateRequest {
    model: "gemini-2.0-flash-exp".to_string(),
    prompt: "test".to_string(),
    max_tokens: Some(100),
};
```

### ❌ WRONG: Importing Builder types

```rust
use botticelli_core::GenerateRequestBuilder; // DON'T DO THIS
```

### ✅ CORRECT: Use derive_builder trait

```rust
// Import the struct, not the builder
use botticelli_core::GenerateRequest;

let request = GenerateRequest::builder()
    .model("gemini-2.0-flash-exp")
    .prompt("test")
    .max_tokens(100)
    .build()?;
```

**Why:**
- More human-readable
- Follows CLAUDE.md guidelines
- The builder trait makes `.builder()` available on the type
- Builder types are implementation details

## Builder Error Handling

### ❌ WRONG: Converting builder errors to strings

```rust
let request = Request::builder()
    .field(value)
    .build()
    .map_err(|e| e.to_string())?; // BAD - loses error context
```

### ✅ CORRECT: Wrap in native error type

```rust
let request = Request::builder()
    .field(value)
    .build()
    .map_err(|e| BotticelliError::builder_error(e))?;
```

**Required:** Builder errors must be wrapped in our error framework to capture:
- File and line information via `#[track_caller]`
- Error context for debugging
- Proper error propagation through `?`

## Test Organization

### Structure

```
crates/
└── botticelli_module/
    ├── src/
    │   └── lib.rs
    └── tests/
        ├── module_test.rs          # Unit/integration tests
        └── narratives/              # Narrative-based tests
            └── platform/
                └── command_test.toml
```

### Naming Conventions

- Test files: `{module}_{component}_test.rs`
- Test functions: `test_{feature_being_tested}`
- Narrative files: `{command}_test.toml`

## Narrative-Based Testing

### Pattern for Command Testing

```rust
#[test]
fn test_command() -> BotticelliResult<()> {
    let narrative_path = test_narrative_path("platform/command_test.toml");
    run_test_narrative(&narrative_path)?;
    Ok(())
}
```

### State Management in Tests

Use persistent state management for test resources:

```rust
// Setup creates resources and caches IDs
#[test]
fn test_setup() -> BotticelliResult<()> {
    let narrative = test_narrative_path("setup_test_channel.toml");
    run_test_narrative(&narrative)?;
    // Channel ID is cached in state for later tests
    Ok(())
}

// Test uses cached IDs
#[test]
fn test_operation() -> BotticelliResult<()> {
    let narrative = test_narrative_path("channel_operation_test.toml");
    run_test_narrative(&narrative)?;
    Ok(())
}

// Teardown cleans up using cached IDs
#[test]
fn test_teardown() -> BotticelliResult<()> {
    let narrative = test_narrative_path("teardown_test_channel.toml");
    run_test_narrative(&narrative)?;
    Ok(())
}
```

## Common Mistakes

### 1. Referencing Non-Existent Environment Variables

❌ WRONG:
```toml
[bot.args]
channel_id = "${TEST_CHANNEL_ID}"  # Doesn't exist!
```

✅ CORRECT:
```toml
# First act creates channel and caches ID
[[act]]
name = "setup_channel"
prompt = "Create test channel"

[[act.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"
required = true

[act.input.args]
guild_id = "${TEST_GUILD_ID}"
name = "test-channel"
cache_key = "TEST_CHANNEL_ID"  # Cache for later use

# Second act uses cached ID
[[act]]
name = "use_channel"
prompt = "Use the channel"

[[act.input]]
type = "bot_command"
platform = "discord"
command = "messages.send"
required = true

[act.input.args]
channel_id = "${STATE:TEST_CHANNEL_ID}"  # Load from state
content = "Test message"
```

### 2. Using [[acts]] Instead of [[act]]

❌ WRONG:
```toml
[[acts]]  # WRONG - this is not valid syntax
name = "test"
```

✅ CORRECT:
```toml
[[act]]  # CORRECT - use singular [[act]]
name = "test"
```

**Always refer to NARRATIVE_TOML_SPEC for correct syntax.**

### 3. Ignoring Test Failures

❌ WRONG:
```rust
#[test]
#[ignore]  // Don't ignore without fixing!
fn test_broken_feature() {
    // ...
}
```

✅ CORRECT:
- Fix the test
- Or remove it if obsolete
- Document why it's ignored if truly needed (rare)

**Ignored tests create technical debt and hide breaking changes.**

## API Rate Limiting in Tests

### Mark API-consuming tests

```rust
#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_api_call() -> BotticelliResult<()> {
    // Test that makes real API calls
    Ok(())
}
```

### Run with explicit opt-in

```bash
cargo test --features gemini,api
```

### Minimize token usage

- Use minimal prompts
- Set low max_tokens (10-100)
- Use cheapest models (gemini-2.5-flash-lite)
- Avoid repeated API calls in tests

## Test Helpers

### Creating Test Helpers

```rust
// In tests/common/mod.rs or similar
pub fn test_narrative_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("narratives")
        .join(relative)
}

pub fn run_test_narrative(path: &Path) -> BotticelliResult<()> {
    // Common narrative execution logic
    Ok(())
}
```

### Using Builders in Helpers

```rust
pub fn create_test_request() -> BotticelliResult<GenerateRequest> {
    GenerateRequest::builder()
        .model("gemini-2.5-flash-lite")
        .prompt("test")
        .max_tokens(10)
        .build()
        .map_err(BotticelliError::builder_error)
}
```

## Feature Gates in Tests

Tests should respect feature gates:

```rust
#[cfg(feature = "discord")]
mod discord_tests {
    // Discord-specific tests
}

#[cfg(all(feature = "discord", feature = "database"))]
mod integration_tests {
    // Tests requiring multiple features
}
```

## Verification Checklist

Before committing test changes:

1. ✅ All tests compile: `cargo test --no-run`
2. ✅ All local tests pass: `cargo test --lib --tests`
3. ✅ Feature combinations work: `just check-features`
4. ✅ Clippy is clean: `cargo clippy --all-targets`
5. ✅ Doctests pass: `cargo test --doc`
6. ✅ No ignored tests without justification
7. ✅ Builder pattern used (no struct literals)
8. ✅ Proper error handling (no .unwrap()/.expect())
9. ✅ State management for test resources
10. ✅ Correct narrative TOML syntax

## Summary

**Key Takeaways:**
- Tests are production code - treat them seriously
- Use Result types and proper error handling
- Use builder pattern, not struct literals
- Manage test resources with state management
- Follow NARRATIVE_TOML_SPEC exactly
- Fix failures immediately, don't ignore
- Verify everything before committing

These patterns ensure our tests are reliable, maintainable, and actually catch bugs.
