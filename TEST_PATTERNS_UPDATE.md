# Test Patterns Update

Based on botticelli_models reference implementation.

## Current Issues in botticelli_mcp

1. **Error Handling**: Tests use `.expect()` and `.unwrap()` instead of `anyhow::Result` with `?` operator
2. **Tracing**: Only 4/28 tests initialize tracing, and those use simple `try_init()` without RUST_LOG check
3. **Observability**: Tests lack tracing spans/events for debugging failures

## Required Patterns (from botticelli_models)

### 1. Create helpers/mod.rs Module

```rust
// tests/helpers/mod.rs

/// Initialize tracing for tests.
///
/// Reads RUST_LOG from environment (including .env file) and falls back
/// to the specified default level if not set.
///
/// Safe to call multiple times - subsequent calls are no-ops.
///
/// # Arguments
///
/// * `fallback_level` - Default log level if RUST_LOG not set (e.g., "info", "debug")
pub fn init_test_tracing(fallback_level: &str) {
    use tracing_subscriber::EnvFilter;

    // Load .env if present
    let _ = dotenvy::dotenv();

    // Try to get RUST_LOG from environment, fall back to provided level
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| fallback_level.to_string());

    // Only initialize once (subsequent calls are no-ops)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_level))
        .with_test_writer()
        .try_init();
}
```

### 2. Test Function Signature

```rust
#[tokio::test]
async fn test_name() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Test description");
    
    // Test body with ? operator
    
    tracing::info!("Test passed");
    Ok(())
}
```

### 3. Tracing in Tests (Good Examples from botticelli_models)

```rust
#[tokio::test]
async fn test_feature() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Test description");
    
    let server = BotticelliServer::builder().build();
    tracing::debug!(?server, "Created server");
    
    let result = server.some_method(params).await?;
    tracing::debug!(?result, "Method completed");
    
    assert_eq!(result.field, expected);
    
    tracing::info!("Test passed");
    Ok(())
}
```

Key tracing points:
- Start: `tracing::info!("Test description")`
- After setup: `tracing::debug!(..., "Created X")`
- After operations: `tracing::debug!(..., "Operation completed")`
- End: `tracing::info!("Test passed")`

### 4. Error Handling with Context

```rust
use anyhow::Context;

// Instead of .expect()
let result = server
    .method(params)
    .await
    .context("Failed to call method with test params")?;

// Builder pattern
let request = GenerateRequest::builder()
    .messages(vec![message])
    .build()?;  // anyhow converts automatically
```

### 5. Sync Tests (Non-async)

```rust
#[test]
fn test_sync_feature() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing sync feature");
    
    let value = some_function()?;
    tracing::debug!(?value, "Got result");
    
    assert_eq!(value, expected);
    
    tracing::info!("Test passed");
    Ok(())
}
```

## Implementation Plan

### Step 1: Create helpers module
- Create `tests/helpers/mod.rs` with `init_test_tracing()` function
- Pattern matches botticelli_models exactly

### Step 2: Update existing tests (priority order)
1. `echo_tool_test.rs` - Simple, 3 tests
2. `elicit_text_test.rs` - Basic elicitation, 3 tests  
3. `narrative_tools_test.rs` - Multiple tests, good representative
4. `elicitation_integration_test.rs` - Already has tracing, needs helper
5. Remaining 24 test files

### Step 3: Verification
- Run with `RUST_LOG=debug cargo test -- --nocapture`
- Verify observability and error messages
- Ensure all tests pass

## Notes

- Use `anyhow::Result<()>` for ALL test functions (async and sync)
- Always call `helpers::init_test_tracing("info")` first
- Add `tracing::info!()` at start and end of tests
- Use `tracing::debug!()` for key operations
- Use `?` operator instead of `.expect()` or `.unwrap()`
- Context adds helpful error messages without cluttering code
