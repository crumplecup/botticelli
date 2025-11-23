# StateManager Tracing and Error Handling Audit

## Current Status

### ✅ Good Practices Found
- Basic debug tracing on key operations (set, remove, clear, load, save, delete)
- Proper error context in error messages
- Using BotticelliResult for error propagation

### ❌ Missing Tracing Instrumentation

1. **No #[instrument] macros** - Functions should use `#[instrument]` for automatic span creation
2. **Missing error tracing** - Errors should be logged with `error!()` before returning
3. **No file path logging** - When reading/writing files, should log the path
4. **No performance tracking** - Load/save operations should track duration
5. **Missing context fields** - Spans should include relevant fields (scope, path, keys)

### ❌ Error Handling Issues

1. **Generic error messages** - "Failed to read state file" doesn't show which file
2. **No location tracking** - Errors don't capture file/line information automatically
3. **I/O errors lose context** - Converting `std::io::Error` to string loses error chain

## Required Fixes

### 1. Add #[instrument] to Public Methods

```rust
use tracing::{debug, error, info, instrument};

#[instrument(skip(self), fields(scope = ?scope))]
pub fn load(&self, scope: &StateScope) -> BotticelliResult<NarrativeState> {
    let path = self.scope_path(scope);
    debug!(path = %path.display(), "Loading state from file");
    
    if !path.exists() {
        debug!("No existing state file, returning empty state");
        return Ok(NarrativeState::new());
    }

    let contents = std::fs::read_to_string(&path).map_err(|e| {
        error!(path = %path.display(), error = %e, "Failed to read state file");
        ConfigError::new(format!("Failed to read state file '{}': {}", path.display(), e))
    })?;

    let state: NarrativeState = serde_json::from_str(&contents).map_err(|e| {
        error!(path = %path.display(), error = %e, "Failed to parse state JSON");
        JsonError::new(format!("Failed to parse state file '{}': {}", path.display(), e))
    })?;

    info!(keys = state.data.len(), "Loaded state successfully");
    Ok(state)
}
```

### 2. Improve Error Context

All error messages should include:
- The specific file path that failed
- The operation that was attempted
- The underlying error details

### 3. Add Performance Metrics

For load/save operations:
```rust
let start = std::time::Instant::now();
// ... operation ...
debug!(duration_ms = start.elapsed().as_millis(), "Operation completed");
```

### 4. Trace State Modifications

Track what's changing:
```rust
pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
    let key = key.into();
    let value = value.into();
    debug!(
        key = %key,
        value = %value,
        size_before = self.data.len(),
        "Setting state value"
    );
    self.data.insert(key, value);
}
```

## Implementation Priority

1. **High**: Add `#[instrument]` to all public methods
2. **High**: Add error logging before all error returns
3. **Medium**: Include file paths in all error messages
4. **Medium**: Add performance tracking for I/O operations
5. **Low**: Add detailed field tracking for state modifications

## Benefits

With proper instrumentation:
- **Debugging**: Clear trace of what state was loaded/saved and when
- **Performance**: Can identify slow I/O operations
- **Error diagnosis**: Full context when something goes wrong (which file, what scope, what operation)
- **Audit trail**: Complete record of state changes for security/compliance
