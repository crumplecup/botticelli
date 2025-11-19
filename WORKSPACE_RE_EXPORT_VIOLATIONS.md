# Workspace Re-Export Violations

**Date:** 2025-11-19  
**Issue:** Multiple crates re-exporting types from dependencies, creating ambiguous import paths

## The Problem

**CRITICAL:** In a workspace, re-exporting dependency types creates multiple import paths for the same type, violating the "single import path" principle.

### Example Violation

```rust
// botticelli_database/src/lib.rs
pub use botticelli_error::{DatabaseError, DatabaseErrorKind};

// Now users can import the same type TWO ways:
use botticelli_error::DatabaseError;     // ❌ Original source
use botticelli_database::DatabaseError;  // ❌ Re-exported

// Which one should they use? Ambiguous!
```

## Violations Found

### 1. botticelli_database ❌

**File:** `crates/botticelli_database/src/lib.rs:50`

```rust
pub use botticelli_error::{DatabaseError, DatabaseErrorKind};
```

**Impact:** DatabaseError can be imported from 2 places

**Fix:**
```rust
// Remove the re-export line entirely
// Users import directly:
use botticelli_error::{DatabaseError, DatabaseErrorKind};
```

**Keep the type alias:**
```rust
/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;  // ✅ This is OK
```

---

### 2. botticelli_models ❌

**File:** `crates/botticelli_models/src/lib.rs:59`

```rust
pub use botticelli_error::{GeminiError, GeminiErrorKind};
```

**Impact:** GeminiError can be imported from 2 places

**Fix:**
```rust
// Remove the re-export
// Users import directly:
use botticelli_error::{GeminiError, GeminiErrorKind};
```

---

### 3. botticelli_narrative ❌

**File:** `crates/botticelli_narrative/src/lib.rs:65,71`

```rust
pub use botticelli_interface::{
    Act, ActExecution, ActInput, ActInputSource, ActOutput, GenerationBackend, Narrative,
    NarrativeExecution, NarrativeRepository, ToolCall,
};

pub use botticelli_error::{NarrativeError, NarrativeErrorKind};
```

**Impact:** 
- 10 interface types can be imported from 2 places
- NarrativeError can be imported from 2 places

**Fix:**
```rust
// Remove both re-export blocks
// Users import directly from the source crates
```

---

### 4. botticelli_storage ❌

**File:** `crates/botticelli_storage/src/lib.rs:46`

```rust
pub use botticelli_error::{StorageError, StorageErrorKind};
```

**Impact:** StorageError can be imported from 2 places

**Fix:**
```rust
// Remove the re-export
// Users import directly:
use botticelli_error::{StorageError, StorageErrorKind};
```

---

### 5. botticelli_tui ⚠️ (Special Case)

**File:** `crates/botticelli_tui/src/lib.rs:13`

```rust
pub use error::{TuiError, TuiErrorKind};
```

**Status:** ⚠️ **QUESTIONABLE**

**Analysis:** This appears to be re-exporting from `crate::error` (internal module), which is correct. However, the syntax is ambiguous - could be interpreted as external crate.

**Recommendation:** Keep but clarify with comment:
```rust
// Re-export from internal error module
pub use error::{TuiError, TuiErrorKind};
```

**Better pattern:**
```rust
pub use crate::error::{TuiError, TuiErrorKind};  // Explicit crate path
```

---

### 6. botticelli (top-level) ✅ EXCEPTION

**File:** `crates/botticelli/src/lib.rs:63-81`

```rust
pub use botticelli_core::*;
pub use botticelli_error::*;
pub use botticelli_interface::*;
pub use botticelli_narrative::*;
pub use botticelli_rate_limit::*;
pub use botticelli_storage::*;
pub use botticelli_models::*;
pub use botticelli_database::*;
pub use botticelli_social::*;
pub use botticelli_tui::*;
```

**Status:** ✅ **ALLOWED**

**Rationale:** The top-level `botticelli` crate is the public API facade. It's the ONLY crate that should re-export workspace dependencies.

**Purpose:** Convenience for end users who want a single import:
```rust
// User can import everything from one place
use botticelli::{Narrative, NarrativeExecutor, GenerationBackend};
```

---

## Impact Analysis

### Ambiguity Examples

**Before (current state):**
```rust
// User imports DatabaseError - which path should they use?
use botticelli_database::DatabaseError;  // Re-exported
use botticelli_error::DatabaseError;     // Original

// Both work! This is confusing and violates single-path principle.
```

**After (fixed):**
```rust
// Only one way to import
use botticelli_error::DatabaseError;     // ✅ Only path
use botticelli_database::NarrativeRepository;  // Database's own types
```

### Breaking Changes

Removing re-exports is a **breaking change** for existing code:

```rust
// This will break:
use botticelli_database::DatabaseError;  // ❌ After fix, this won't compile

// Users must update to:
use botticelli_error::DatabaseError;     // ✅ Import from source
```

**Migration required:** Update all internal crates to import from source.

---

## Fix Strategy

### Phase 1: Update Internal Crates

Fix all internal workspace crates to import from source:

```bash
# Find all imports of re-exported types
grep -r "use botticelli_database::DatabaseError" crates/
grep -r "use botticelli_models::GeminiError" crates/
grep -r "use botticelli_narrative::Act" crates/
grep -r "use botticelli_storage::StorageError" crates/
```

Replace with direct imports:
```rust
// Change this:
use botticelli_database::DatabaseError;

// To this:
use botticelli_error::DatabaseError;
```

### Phase 2: Remove Re-Exports

Once all internal crates are updated, remove the re-export lines:

**botticelli_database/src/lib.rs:**
```rust
// Remove line 50:
// pub use botticelli_error::{DatabaseError, DatabaseErrorKind};
```

**botticelli_models/src/lib.rs:**
```rust
// Remove line 59:
// pub use botticelli_error::{GeminiError, GeminiErrorKind};
```

**botticelli_narrative/src/lib.rs:**
```rust
// Remove lines 65-70 and 71:
// pub use botticelli_interface::{ ... };
// pub use botticelli_error::{NarrativeError, NarrativeErrorKind};
```

**botticelli_storage/src/lib.rs:**
```rust
// Remove line 46:
// pub use botticelli_error::{StorageError, StorageErrorKind};
```

### Phase 3: Verify

```bash
# Ensure workspace compiles
cargo check --all-features

# Ensure tests pass
just test-all
```

---

## Type Aliases vs Re-Exports

### ✅ Type Aliases: OK

```rust
use botticelli_error::DatabaseError;

/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;
```

**Why this is fine:**
- Creates a NEW type name (DatabaseResult)
- Does NOT create an alias for DatabaseError itself
- Users can still only import DatabaseError from botticelli_error

### ❌ Re-Exports: NOT OK

```rust
pub use botticelli_error::DatabaseError;  // ❌ Creates duplicate import path
```

---

## Updated CLAUDE.md

Added new section: **"Cross-Crate Dependencies"**

**Key rules:**
1. **NO re-exports** of dependency types in workspace crates
2. **Import directly** from the type's home crate
3. **Type aliases OK** for convenience (e.g., DatabaseResult)
4. **Exception:** Top-level public API crate may re-export for user convenience

---

## Summary

### Violations Found: 4 crates

1. botticelli_database - 1 re-export
2. botticelli_models - 1 re-export  
3. botticelli_narrative - 2 re-export blocks
4. botticelli_storage - 1 re-export

### Total Re-Exported Types: ~15

- DatabaseError, DatabaseErrorKind
- GeminiError, GeminiErrorKind
- NarrativeError, NarrativeErrorKind
- StorageError, StorageErrorKind
- Act, ActExecution, ActInput, ActInputSource, ActOutput
- GenerationBackend, Narrative, NarrativeExecution
- NarrativeRepository, ToolCall

### Fix Required

**Phase 1:** Update internal crates to import from source  
**Phase 2:** Remove re-export lines  
**Phase 3:** Verify compilation and tests  

**Estimated time:** 30-60 minutes

**Breaking change:** Yes - but only for internal workspace usage (acceptable)

---

## Checklist

- [x] Update botticelli_database imports
- [x] Remove botticelli_database re-exports
- [x] Update botticelli_models imports  
- [x] Remove botticelli_models re-exports
- [x] Update botticelli_narrative imports
- [x] Remove botticelli_narrative re-exports
- [x] Update botticelli_storage imports
- [x] Remove botticelli_storage re-exports
- [x] Verify workspace compilation
- [x] Run all tests
- [x] Update documentation
- [ ] Commit changes

## Fix Applied - 2025-11-19

All re-export violations have been fixed:

### Changes Made

**botticelli_database/src/lib.rs:**
- Removed: `pub use botticelli_error::{DatabaseError, DatabaseErrorKind};`
- Changed to: `use botticelli_error::{DatabaseError, DatabaseErrorKind};` (private import for internal use)
- Kept type alias: `pub type DatabaseResult<T> = Result<T, DatabaseError>;`

**botticelli_models/src/lib.rs:**
- Removed: `pub use botticelli_error::{GeminiError, GeminiErrorKind};`

**botticelli_narrative/src/lib.rs:**
- Removed: `pub use botticelli_interface::{...};` (10 types)
- Removed: `pub use botticelli_error::{NarrativeError, NarrativeErrorKind};`

**botticelli_storage/src/lib.rs:**
- Removed: `pub use botticelli_error::{StorageError, StorageErrorKind};`

**Test files updated:**
- `botticelli_models/tests/gemini_mock_test.rs` - Import GeminiErrorKind from botticelli_error
- `botticelli_models/tests/test_utils/mock_gemini.rs` - Import GeminiError/Kind from botticelli_error
- `botticelli_models/tests/gemini.rs` - Import GeminiError/Kind from botticelli_error

### Verification

```bash
✅ cargo check --all-features  # Success
✅ cargo clippy --all-features --all-targets  # 0 warnings
✅ just test-all  # All tests pass
```

### Result

- **NO MORE RE-EXPORTS** across internal workspace crates
- Each type has exactly ONE import path
- Type aliases preserved for convenience
- All tests passing
- Zero clippy warnings

---

**Created:** 2025-11-19  
**Updated CLAUDE.md:** Yes (Cross-Crate Dependencies section)  
**Priority:** High (architectural issue)
