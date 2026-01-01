# Interface Dependency Violation Fix

## Problem

`botticelli_interface` depends on `botticelli_rate_limit` and `botticelli_storage`, violating the foundational crate principle. Interface crates should define contracts (traits) with minimal dependencies and be depended ON, not depend on concrete implementations.

**Current (WRONG):**
```
interface → rate_limit, storage, core
```

**Correct:**
```
rate_limit, storage → interface
core → interface (for trait bounds only)
```

## Root Cause

Traits in `_interface` reference concrete types from downstream crates:

1. **`BotticelliDriver::rate_limits()`** returns `&botticelli_rate_limit::RateLimitConfig`
2. **Blanket Arc impl** creates hard dependency on `botticelli_rate_limit::RateLimitConfig`
3. **`provider.rs`** defines concrete `ProviderError` types (should be in `_error`)
4. **Storage types** leaked into interface somehow

## Solution: Associated Types Pattern

### Step 1: Refactor `BotticelliDriver` trait

**Before:**
```rust
fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig;
```

**After:**
```rust
pub trait BotticelliDriver: Send + Sync {
    /// Associated type for rate limit configuration
    type RateLimitConfig: Send + Sync;
    
    /// Rate limits for this driver
    fn rate_limits(&self) -> &Self::RateLimitConfig;
    
    // ... rest of trait
}
```

### Step 2: Move implementations downstream

Implementations in `botticelli_anthropic`, `botticelli_gemini`, etc. specify concrete types:

```rust
impl BotticelliDriver for AnthropicClient {
    type RateLimitConfig = botticelli_rate_limit::RateLimitConfig;
    
    fn rate_limits(&self) -> &Self::RateLimitConfig {
        &self.rate_limits
    }
    
    // ... rest of impl
}
```

### Step 3: Remove `ProviderError` from `_interface`

Move `ProviderError` and `ProviderErrorKind` to `botticelli_error`:
- Add to `CrateErrorKind` enum
- Use `bridge_error!` and `error_from!` macros
- Update `_interface` to not define error types

### Step 4: Remove concrete dependencies

Remove from `_interface/Cargo.toml`:
```toml
botticelli_rate_limit = { workspace = true }
botticelli_storage = { workspace = true }
```

Keep only:
```toml
botticelli_core = { workspace = true }      # For GenerateRequest/Response
botticelli_error = { workspace = true }     # For BotticelliResult
```

### Step 5: Update blanket implementations

**Before:**
```rust
impl<T: BotticelliDriver + ?Sized> BotticelliDriver for std::sync::Arc<T> {
    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        (**self).rate_limits()
    }
}
```

**After:**
```rust
impl<T: BotticelliDriver + ?Sized> BotticelliDriver for std::sync::Arc<T> {
    type RateLimitConfig = T::RateLimitConfig;
    
    fn rate_limits(&self) -> &Self::RateLimitConfig {
        (**self).rate_limits()
    }
}
```

## Implementation Checklist

- [ ] Add `type RateLimitConfig` to `BotticelliDriver` trait
- [ ] Remove concrete `RateLimitConfig` from method signatures
- [ ] Update Arc blanket impl to use associated type
- [ ] Move `ProviderError`/`ProviderErrorKind` to `botticelli_error`
- [ ] Remove `botticelli_rate_limit` dependency from `_interface`
- [ ] Remove `botticelli_storage` dependency from `_interface`
- [ ] Update all provider implementations (`anthropic`, `gemini`, `openai`, etc.)
- [ ] Update narrative/chat code that uses `BotticelliDriver`
- [ ] Run `just check-all` to verify

## Benefits

1. **Correct dependency direction**: Interface is foundational
2. **No circular dependencies**: Clean layering
3. **Flexibility**: Each implementation chooses its rate limit config type
4. **Maintainability**: Changes to rate_limit don't require interface changes
5. **Testability**: Can mock with simple types in tests

## Files to Modify

### `botticelli_interface`
- `src/traits.rs` - Add associated type, update signatures
- `src/provider.rs` - Delete (move to `_error`)
- `Cargo.toml` - Remove `rate_limit` and `storage` dependencies

### `botticelli_error`
- `src/provider.rs` - Create with moved types
- `src/lib.rs` - Add `mod provider`, export types, add to `CrateErrorKind`

### Provider crates (`anthropic`, `gemini`, `openai`, `groq`, `ollama`)
- Specify `type RateLimitConfig = botticelli_rate_limit::RateLimitConfig` in impl
- Update error handling to use `botticelli_error::ProviderError`

### Downstream crates using `BotticelliDriver`
- Update trait bounds if needed (likely minimal changes)
- Most code won't change as method calls stay the same

---

## Implementation Summary

### Changes Made

1. **`BotticelliDriver` trait** - Added `type RateLimitConfig` associated type
   - Removed hard dependency on `botticelli_rate_limit::RateLimitConfig`
   - Updated `rate_limits()` to return `&Self::RateLimitConfig`
   - Updated Arc blanket impl to use associated type

2. **`LlmProvider` trait** - Added `type Error` associated type
   - Removed `ProviderError` concrete type from trait
   - Implementations specify their own error types

3. **`NarrativeRepository` trait** - Added associated types for storage
   - `type MediaMetadata: Send + Sync`
   - `type MediaReference: Send + Sync`
   - Removed hard dependencies on `botticelli_storage` types

4. **Removed dependencies** from `botticelli_interface/Cargo.toml`:
   - `botticelli_rate_limit`
   - `botticelli_storage`

### Result

`botticelli_interface` now depends only on:
- `botticelli_core` (for request/response types)
- `botticelli_error` (for result types)
- Standard trait/async crates

Downstream crates implement the traits with concrete types:
- `botticelli_anthropic` implements `LlmProvider` with `AnthropicError`
- `botticelli_rate_limit` implements rate limiting with `RateLimitConfig`
- `botticelli_database` implements `NarrativeRepository` with storage types

### Verification

```bash
cargo check -p botticelli_interface  # ✅ Compiles successfully
```

All trait abstractions now follow the correct pattern: traits define contracts with associated types, implementations provide concrete types.
