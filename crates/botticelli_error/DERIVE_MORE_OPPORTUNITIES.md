# derive_more Opportunities in botticelli_error

**Date:** 2025-11-19  
**Analysis:** Identifying additional derive_more usage opportunities

## Executive Summary

All error modules currently use **manual implementations** for `Display` and `Error` traits. We can replace these with `derive_more` to reduce boilerplate and improve maintainability.

## Current Pattern Analysis

### Pattern 1: Simple Error Structs (5 instances)

**Files:** `http.rs`, `json.rs`, `config.rs`, `backend.rs`, `not_implemented.rs`

**Current code:**
```rust
#[derive(Debug, Clone)]
pub struct HttpError {
    pub message: String,
    pub line: u32,
    pub file: &'static str,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HTTP Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for HttpError {}
```

**Proposed improvement:**
```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("HTTP Error: {} at line {} in {}", message, line, file)]
pub struct HttpError {
    pub message: String,
    pub line: u32,
    pub file: &'static str,
}
```

**Savings:** ~8 lines of boilerplate per file × 5 files = **40 lines**

### Pattern 2: ErrorKind Enums (5 instances)

**Files:** `storage.rs`, `gemini.rs`, `database.rs`, `narrative.rs`, `tui.rs`

#### Example: StorageErrorKind

**Current code:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageErrorKind {
    NotFound(String),
    PermissionDenied(String),
    Io(String),
    InvalidConfig(String),
    Unavailable(String),
    HashMismatch { expected: String, actual: String },
    Other(String),
}

impl std::fmt::Display for StorageErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageErrorKind::NotFound(path) => write!(f, "Media not found: {}", path),
            StorageErrorKind::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            StorageErrorKind::Io(msg) => write!(f, "I/O error: {}", msg),
            StorageErrorKind::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            StorageErrorKind::Unavailable(msg) => write!(f, "Storage unavailable: {}", msg),
            StorageErrorKind::HashMismatch { expected, actual } => {
                write!(f, "Content hash mismatch: expected {}, got {}", expected, actual)
            }
            StorageErrorKind::Other(msg) => write!(f, "{}", msg),
        }
    }
}
```

**Proposed improvement:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum StorageErrorKind {
    #[display("Media not found: {}", _0)]
    NotFound(String),
    #[display("Permission denied: {}", _0)]
    PermissionDenied(String),
    #[display("I/O error: {}", _0)]
    Io(String),
    #[display("Invalid configuration: {}", _0)]
    InvalidConfig(String),
    #[display("Storage unavailable: {}", _0)]
    Unavailable(String),
    #[display("Content hash mismatch: expected {}, got {}", expected, actual)]
    HashMismatch { expected: String, actual: String },
    #[display("{}", _0)]
    Other(String),
}
```

**Savings:** ~20 lines of boilerplate per file × 5 files = **100 lines**

#### Example: GeminiErrorKind (most complex)

**Current:** 38 lines of manual Display impl
**Proposed:** derive_more with #[display(...)] attributes per variant

**Special considerations:**
- GeminiErrorKind has additional methods (`is_retryable`, `retry_strategy_params`)
- These methods can coexist with derive_more
- **Savings:** ~38 lines

### Pattern 3: Wrapper Error Structs (5 instances)

**Files:** `storage.rs`, `gemini.rs`, `database.rs`, `narrative.rs`, `tui.rs`

**Current code:**
```rust
#[derive(Debug, Clone)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Storage Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for StorageError {}
```

**Proposed improvement:**
```rust
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Storage Error: {} at line {} in {}", kind, line, file)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub line: u32,
    pub file: &'static str,
}
```

**Savings:** ~8 lines of boilerplate per file × 5 files = **40 lines**

## Total Opportunities

| Pattern | Files | Lines Saved | Priority |
|---------|-------|-------------|----------|
| Simple error structs | 5 | 40 | HIGH |
| Wrapper error structs | 5 | 40 | HIGH |
| ErrorKind enums | 5 | 100+ | MEDIUM |
| **Total** | **15** | **180+** | - |

## Implementation Plan

### Phase 1: Simple Error Structs (Easy wins)

Files to update:
1. `http.rs` - HttpError
2. `json.rs` - JsonError
3. `config.rs` - ConfigError
4. `backend.rs` - BackendError
5. `not_implemented.rs` - NotImplementedError

**Action:** Replace manual Display and Error impls with derive_more

### Phase 2: Wrapper Error Structs (Easy wins)

Files to update:
1. `storage.rs` - StorageError
2. `gemini.rs` - GeminiError
3. `database.rs` - DatabaseError
4. `narrative.rs` - NarrativeError
5. `tui.rs` - TuiError

**Action:** Replace manual Display and Error impls with derive_more

### Phase 3: ErrorKind Enums (More involved)

Files to update:
1. `storage.rs` - StorageErrorKind
2. `database.rs` - DatabaseErrorKind
3. `narrative.rs` - NarrativeErrorKind
4. `tui.rs` - TuiErrorKind
5. `gemini.rs` - GeminiErrorKind (most complex)

**Action:** Replace manual Display impls with derive_more and #[display(...)] attributes

## Benefits

1. **Less boilerplate:** ~180 lines removed
2. **More declarative:** Error messages visible in derive attributes
3. **Easier to maintain:** Format changes are simpler
4. **Consistent patterns:** All errors use same derive approach
5. **CLAUDE.md compliance:** "Use derive_more when appropriate"

## Considerations

### GeminiErrorKind Complexity

The GeminiErrorKind has the most complex Display impl (13 variants, some with multiple fields). Using derive_more here requires:
- Individual #[display(...)] attributes on each variant
- More verbose than simple forwarding
- But still cleaner than manual match statements

**Recommendation:** Still worth it for consistency and maintainability

### Error Type Coexistence

All ErrorKind enums can have derive_more::Display even though they don't implement `std::error::Error` themselves. The wrapper structs implement `Error` and delegate to the kind's `Display`.

This pattern is already proven in `BotticelliErrorKind` and `BotticelliError`.

## Verification Checklist

After each phase:

```bash
cd crates/botticelli_error

# 1. Check compilation
cargo check

# 2. Run doctests
cargo test --doc

# 3. Run clippy
cargo clippy --all-targets

# 4. Verify workspace integration
cd ../.. && just test-all
```

## Expected Outcome

After all phases:
- ✅ ~180 lines of boilerplate removed
- ✅ Consistent derive_more usage throughout
- ✅ All 15 doctests still pass
- ✅ 0 compiler warnings
- ✅ 0 clippy warnings
- ✅ Full CLAUDE.md compliance

---

**Analysis Date:** 2025-11-19  
**Status:** Ready for implementation
