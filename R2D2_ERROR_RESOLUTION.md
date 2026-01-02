# R2D2 Error Type Resolution Issue

## The Problem

Database tests fail to compile with:
```
error[E0277]: the trait bound `botticelli_error::DatabaseError: From<r2d2::Error>` is not satisfied
```

But the compiler also says:
```
= help: the following other types implement trait `From<T>`:
          `botticelli_error::DatabaseError` implements `From<diesel::r2d2::Error>`
```

## Root Cause

1. **Type Identity Problem**: `diesel::r2d2::Error` is a re-export of `r2d2::Error`
2. **Different Import Paths**: Tests see the error as `r2d2::Error`, not `diesel::r2d2::Error`
3. **Type Resolution**: Any type annotation or inference resolves `diesel::r2d2::PoolError` to underlying `r2d2::Error`

## Why Helper Functions Failed

Every approach with closures or type annotations failed because:
- `|e: diesel::r2d2::PoolError|` → compiler resolves to `r2d2::Error`
- `fn convert(err: diesel::r2d2::PoolError)` → compiler resolves to `r2d2::Error`
- The `From<diesel::r2d2::Error>` implementation never matches

## The Correct Solution

**Add `r2d2` as a direct dependency to `botticelli_error` and implement `From<r2d2::Error>`**

This is the right approach because:
1. `diesel::r2d2::Error` IS `r2d2::Error` - they're the same type
2. Tests naturally use `r2d2::Error` (via `pool.get()` return type)
3. Matches our error handling pattern (wrap external errors)
4. Enables proper `R2d2Error` wrapper with source tracking

## Implementation

1. Add `r2d2` dependency to `botticelli_error/Cargo.toml`:
   ```toml
   r2d2 = { workspace = true, optional = true }
   ```

2. Update `database` feature:
   ```toml
   database = ["dep:diesel", "dep:serde_json", "dep:r2d2"]
   ```

3. Add `From<r2d2::Error>` implementation in `database.rs`:
   ```rust
   #[cfg(feature = "database")]
   impl From<r2d2::Error> for DatabaseError {
       #[track_caller]
       fn from(err: r2d2::Error) -> Self {
           DatabaseError::new(DatabaseErrorKind::R2d2(R2d2Error::new(err)))
       }
   }
   ```

4. Update `R2d2Error::new()` to accept `r2d2::Error`:
   ```rust
   pub fn new(err: r2d2::Error) -> Self {
       let location = std::panic::Location::caller();
       Self {
           source: Box::new(err),
           line: location.line(),
           file: location.file(),
       }
   }
   ```

5. Update `R2d2Error` source field type:
   ```rust
   source: Box<r2d2::Error>,
   ```

6. Tests can then use simple `.map_err(DatabaseError::from)`

## Why This Is Better

- **Explicit dependency**: Clear that we handle r2d2 errors
- **Type correctness**: Matches actual types from diesel API
- **Less verbose**: No helper functions needed in tests
- **Maintainable**: Single source of truth for r2d2 error conversion
- **Pattern consistency**: Same as other external error wrappers
