# Narrative Executor Trait Object Support

## Problem

`execute_narrative()` in `narratives.rs` has massive code duplication (lines 77-215) with nearly identical blocks for each driver that differ only in:
- Model prefix string (`"gemini"`, `"claude"`, etc.)
- Driver getter method
- Driver name for logging

Currently can't abstract this because `NarrativeExecutor` is generic over `D: BotticelliDriver` which has associated types, preventing use of trait objects.

## Solution

Modify `NarrativeExecutor` to accept `ExecutionDriver` trait objects instead of being generic over `BotticelliDriver`.

## Changes Required

### 1. Add `rate_limits()` to ExecutionDriver trait

`ExecutionDriver` currently has:
- `generate()` - ✅ exists
- `provider_name()` - ✅ exists  
- `model_name()` - ✅ exists
- `rate_limits()` - ❌ missing

Need to add with boxed return type to avoid associated types:

```rust
/// Get rate limit configuration (boxed to avoid associated types).
#[instrument(skip(self))]
fn rate_limits(&self) -> Box<dyn std::any::Any + Send + Sync>;
```

Update blanket impl:
```rust
#[instrument(skip(self))]
fn rate_limits(&self) -> Box<dyn std::any::Any + Send + Sync> {
    Box::new(BotticelliDriver::rate_limits(self).clone())
}
```

### 2. Update NarrativeExecutor to use ExecutionDriver

Change from:
```rust
pub struct NarrativeExecutor<D, BE>
where
    D: BotticelliDriver<Request = GenerateRequest, Response = GenerateResponse>,
{
    driver: D,
    ...
}
```

To:
```rust
pub struct NarrativeExecutor<BE = botticelli_error::NarrativeError>
where
    BE: std::error::Error + Send + Sync + 'static,
{
    driver: Arc<dyn ExecutionDriver<GenerateRequest, GenerateResponse>>,
    ...
}
```

### 3. Update NarrativeExecutor::new() signature

From:
```rust
pub fn new(driver: D) -> Self
```

To:
```rust
#[instrument(skip(driver))]
pub fn new(driver: Arc<dyn ExecutionDriver<GenerateRequest, GenerateResponse>>) -> Self {
    debug!("Creating narrative executor");
    Self {
        driver,
        processor_registry: None,
        bot_registry: None,
        table_registry: None,
        state_manager: None,
    }
}
```

### 4. Update rate_limits() usage in carousel

In `executor/core.rs` or wherever carousel state is created:
```rust
// Old:
let state = CarouselState::new(carousel_config.clone(), self.driver.rate_limits().clone());

// New:
#[instrument(skip(self, carousel_config))]
{
    debug!("Getting rate limits for carousel");
    let rate_limits = self.driver.rate_limits();
    let rate_limits = rate_limits
        .downcast_ref::<RateLimitConfig>()
        .expect("Rate limits should be RateLimitConfig");
    debug!(?rate_limits, "Rate limits retrieved");
    let state = CarouselState::new(carousel_config.clone(), rate_limits.clone());
}
```

### 5. Refactor execute_narrative() in botticelli_mcp

Replace 140 lines of duplication with:
```rust
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
{
    use rmcp::model::ErrorCode;
    use std::borrow::Cow;

    // Determine which driver to use based on model
    let model_str = model.unwrap_or_else(default_model);
    debug!(model = %model_str, "Selecting driver for narrative execution");
    
    // Use modified narrative for execution
    let narrative_source = botticelli_narrative::NarrativeSource::Single(Box::new(narrative));

    // Use helper from execution/helpers.rs
    let driver = self.select_driver(&model_str)?;
    debug!(provider = driver.provider_name(), model = driver.model_name(), "Driver selected");
    
    let executor = botticelli_narrative::NarrativeExecutor::new(driver);
    debug!("Executing narrative from source");
    
    let execution = executor
        .execute_from_source(&narrative_source)
        .await
        .map_err(|e| {
            use rmcp::model::ErrorCode;
            use std::borrow::Cow;
            error!(error = %e, "Narrative execution failed");
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Narrative execution failed: {}", e)),
                None,
            )
        })?;
    
    debug!(acts_count = execution.act_executions().len(), "Narrative execution completed");
    let result = Self::convert_execution_result(execution);
    return Ok(Json(result));
}
```

From ~160 lines down to ~35 lines (including instrumentation).

## Files to Modify

1. `crates/botticelli_interface/src/execution_driver.rs` - Add rate_limits() method
2. `crates/botticelli_narrative/src/executor/core.rs` - Change NarrativeExecutor to use trait object
3. `crates/botticelli_narrative/src/executor/llm.rs` - Update impls if needed
4. `crates/botticelli_narrative/src/executor/processing.rs` - Update carousel rate_limits usage
5. `crates/botticelli_mcp/src/rmcp_server/tools/execution/narratives.rs` - Simplify execute_narrative()

## Testing

- Run `cargo check -p botticelli_narrative`
- Run `cargo check -p botticelli_mcp --all-features`
- Run `cargo test -p botticelli_narrative`
- Run `cargo test -p botticelli_mcp`
- Run `just check-features botticelli_narrative`
- Run `just check-features botticelli_mcp`

## Benefits

- Eliminates 140+ lines of duplication
- Consistent with our ExecutionDriver architecture  
- Makes adding new drivers trivial (no code changes in execute_narrative)
- Improves maintainability
