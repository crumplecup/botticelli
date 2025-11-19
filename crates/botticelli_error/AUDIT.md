# Botticelli Error Audit Report

**Date:** 2025-11-19
**Auditor:** Claude (following CLAUDE.md guidelines)  
**Crate:** botticelli_error v0.2.0

## Executive Summary

The `botticelli_error` crate needs **significant refactoring** to meet CLAUDE.md standards. While the error patterns are good, the entire crate is in a single 771-line `lib.rs` file, violating the fundamental organizational principle that lib.rs should only contain mod declarations and exports.

**Overall Status:** ❌ **NON-COMPLIANT** - Requires major reorganization

## ❌ Critical Issues

### 1. **lib.rs Contains All Types** (Priority: CRITICAL)

**CLAUDE.md says:**
> lib.rs should ONLY contain `mod` declarations and `pub use` exports, never type definitions, trait definitions, or impl blocks.

**Current state:**
- Single file with 771 lines
- Contains 10 error struct types
- Contains 6 error enum types
- Contains trait definitions
- Contains impl blocks
- Contains feature-gated code

**Required action:**
Complete restructuring into focused modules (see recommended structure below).

### 2. **Missing Safety Lints** (Priority: CRITICAL)

**CLAUDE.md requirement:**
```rust
#![forbid(unsafe_code)]
#![warn(missing_docs)]
```

**Current state:**
- Missing `#![forbid(unsafe_code)]`
- Missing `#![warn(missing_docs)]`

**Action:** Add both lints to lib.rs

### 3. **Unused Dependencies** (Priority: HIGH)

**Cargo.toml:**
```toml
derive_more = { workspace = true }
derive-new = { workspace = true }
```

**Current usage:**
- `derive_more`: ✅ Used (derive_more::From on BotticelliErrorKind)
- `derive-new`: ❌ **NOT USED ANYWHERE**

**Action:** Remove `derive-new` dependency

## ⚠️ Medium Priority Issues

### 4. **Incomplete Derive Policies** (Priority: MEDIUM)

**CLAUDE.md says:**
> Data structures should derive Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, and Hash if possible.

**Current state:**

| Type | Derives | Missing |
|------|---------|---------|
| HttpError | Debug | Clone, PartialEq (can't be Copy due to String) |
| JsonError | Debug | Clone, PartialEq |
| ConfigError | Debug | Clone, PartialEq |
| NotImplementedError | Debug | Clone, PartialEq |
| BackendError | Debug | Clone, PartialEq |
| StorageErrorKind | Debug, Clone, PartialEq | Eq, Hash (can add) |
| StorageError | Debug, Clone | PartialEq, Eq (transitive) |
| GeminiErrorKind | Debug, Clone, PartialEq, Eq, Hash | ✅ Complete |
| GeminiError | Debug, Clone | PartialEq, Eq, Hash (transitive) |
| DatabaseErrorKind | Debug, Clone, PartialEq, Eq, Hash | ✅ Complete |
| DatabaseError | Debug, Clone | PartialEq, Eq, Hash (transitive) |
| NarrativeErrorKind | Debug, Clone, PartialEq, Eq, Hash | ✅ Complete |
| NarrativeError | Debug, Clone | PartialEq, Eq, Hash (transitive) |
| TuiErrorKind | Debug, Clone, PartialEq, Eq | Hash (can add) |
| TuiError | Debug, Clone | PartialEq, Eq, Hash (transitive) |

**Action:** 
- Add Eq and Hash to *ErrorKind enums where possible
- Consider adding PartialEq, Eq, Hash to wrapper structs

###5. **No Documentation Examples** (Priority: MEDIUM)

**Current state:**
- 771 lines of code
- 0 doctests
- No usage examples for any error type
- No examples of error conversion patterns
- No examples of RetryableError trait usage

**Action:** Add doctests demonstrating:
- Error creation with location tracking
- Error conversion (`?` operator)
- RetryableError trait usage
- Feature-gated conversions

### 6. **Missing derive_more Usage** (Priority: LOW)

**CLAUDE.md says:**
> Use derive_more to derive Display, FromStr, From, Deref, DerefMut, AsRef, and AsMut when appropriate.

**Current state:**
- All error types manually implement Display
- Manual Display implementations are repetitive
- Could use derive_more::Display on enums

**Example improvement:**
```rust
// Current
impl std::fmt::Display for StorageErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageErrorKind::NotFound(path) => write!(f, "Media not found: {}", path),
            // ... 6 more variants
        }
    }
}

// Better with derive_more
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum StorageErrorKind {
    #[display("Media not found: {_0}")]
    NotFound(String),
    // ...
}
```

## 📋 Recommended Module Structure

Based on the 771-line lib.rs, here's the recommended organization:

```
crates/botticelli_error/src/
├── lib.rs                  # Only mod/pub use (target: ~50 lines)
├── http.rs                 # HttpError
├── json.rs                 # JsonError
├── config.rs               # ConfigError
├── backend.rs              # BackendError
├── not_implemented.rs      # NotImplementedError
├── storage.rs              # StorageError + StorageErrorKind
├── gemini.rs               # GeminiError + GeminiErrorKind + RetryableError
├── database.rs             # DatabaseError + DatabaseErrorKind (#[cfg(feature = "database")])
├── narrative.rs            # NarrativeError + NarrativeErrorKind
├── tui.rs                  # TuiError + TuiErrorKind
└── wrapper.rs              # BotticelliError + BotticelliErrorKind + BotticelliResult
```

**lib.rs structure (final):**
```rust
//! Error types for the Botticelli library.
//!
//! This crate provides the foundation error types used throughout the Botticelli ecosystem.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod http;
mod json;
mod config;
mod backend;
mod not_implemented;
mod storage;
mod gemini;
mod database;
mod narrative;
mod tui;
mod wrapper;

pub use http::HttpError;
pub use json::JsonError;
pub use config::ConfigError;
pub use backend::BackendError;
pub use not_implemented::NotImplementedError;
pub use storage::{StorageError, StorageErrorKind};
pub use gemini::{GeminiError, GeminiErrorKind, RetryableError};
pub use database::{DatabaseError, DatabaseErrorKind};
pub use narrative::{NarrativeError, NarrativeErrorKind};
pub use tui::{TuiError, TuiErrorKind};
pub use wrapper::{BotticelliError, BotticelliErrorKind, BotticelliResult};
```

## 🎯 Implementation Plan

### Phase 1: Critical Fixes (Required)

1. **Add safety lints to lib.rs**
   - Add `#![forbid(unsafe_code)]`
   - Add `#![warn(missing_docs)]`

2. **Remove unused dependency**
   - Remove `derive-new` from Cargo.toml

3. **Reorganize into modules**
   - Create 11 focused module files
   - Move types from lib.rs to appropriate modules
   - Update lib.rs to only mod/pub use

### Phase 2: Derive Improvements (Recommended)

1. **Add missing derives to *ErrorKind enums**
   - StorageErrorKind: add Eq, Hash
   - TuiErrorKind: add Hash

2. **Consider wrapper struct derives**
   - Add PartialEq to error wrappers where possible
   - Document why Eq/Hash are difficult (due to line/file tracking)

### Phase 3: Documentation (Recommended)

1. **Add doctests to each module**
   - Error creation examples
   - Error conversion examples
   - Feature-gated examples

2. **Add module-level documentation**
   - Explain error hierarchy
   - Show common usage patterns
   - Document retry strategies

### Phase 4: derive_more Usage (Optional)

1. **Use derive_more::Display on enums**
   - Replace manual Display impls with derives
   - Cleaner, less repetitive code

## 🔍 Positive Aspects

Despite organizational issues, the crate has some good qualities:

1. **Excellent error structure** - ErrorKind enum + wrapper struct pattern
2. **Location tracking** - All errors use `#[track_caller]`
3. **Feature gates** - Proper use of `#[cfg(feature = "database")]`
4. **Retry logic** - RetryableError trait is well-designed
5. **Blanket From impl** - Smart use of `From<T> where T: Into<ErrorKind>`

## ✅ Verification Checklist

After refactoring:

```bash
# 1. Check compilation
cargo check

# 2. Check with database feature
cargo check --features database

# 3. Run clippy
cargo clippy --all-targets

# 4. Run doctests
cargo test --doc

# 5. Verify workspace integration
cd ../.. && just test-all
```

## 📊 Current Metrics

```
Total Lines: 771 (all in lib.rs)
Modules: 0 (everything in lib.rs)
Public Error Types: 16 (10 wrappers + 6 enums)
Doctests: 0
Features: 1 (database)
Dependencies: 2 (1 unused)
```

## 📈 Target Metrics (After Refactoring)

```
lib.rs Lines: ~50 (only mod/pub use)
Module Files: 11 (focused responsibility)
Public Error Types: 16 (properly organized)
Doctests: ~20 (usage examples)
Features: 1 (database)
Dependencies: 1 (derive_more only)
```

## 🚦 Compliance Status

| Category | Status | Notes |
|----------|--------|-------|
| Module Organization | ❌ FAIL | All types in lib.rs |
| Safety Lints | ❌ FAIL | Missing forbid/warn |
| Dependencies | ⚠️ PARTIAL | One unused |
| Derive Policies | ⚠️ PARTIAL | Some missing derives |
| Documentation | ❌ FAIL | No doctests |
| derive_more | ⚠️ PARTIAL | Only From, not Display |
| Testing | ❌ FAIL | No tests |

## 🎯 Priority Order

1. **CRITICAL:** Add safety lints (5 minutes)
2. **CRITICAL:** Restructure into modules (2-3 hours)
3. **HIGH:** Remove unused dependency (1 minute)
4. **MEDIUM:** Add missing derives (30 minutes)
5. **MEDIUM:** Add doctests (1-2 hours)
6. **LOW:** Use derive_more::Display (1 hour)

**Estimated total work:** 4-6 hours for full compliance

## 📚 Next Steps

1. Review this audit with the team
2. Create REFACTOR.md planning document
3. Implement Phase 1 (critical fixes)
4. Test thoroughly after each phase
5. Update this audit as a FIXES.md report

---

**Audit Completed:** 2025-11-19  
**Status:** Requires major refactoring  
**Recommendation:** High priority - sets foundation for all error handling
