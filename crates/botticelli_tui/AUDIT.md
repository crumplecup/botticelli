# Botticelli TUI Crate Audit

## Overview
Audit of `botticelli_tui` against CLAUDE.md coding standards.

## Critical Issues

### 1. ❌ Error handling re-exports in error.rs
**Location:** `src/error.rs`
**Issue:** This crate re-exports error types from `botticelli_error` instead of defining its own errors.
**Fix:** According to workspace policy, each crate should define its own error types. TUI-specific errors should be defined here, not re-exported.
**Priority:** CRITICAL

### 2. ❌ Missing derives on enums
**Location:** `src/app.rs` - `AppMode`, `EditField`
**Issue:** Fieldless enums missing `strum::EnumIter` derive
**Fix:** Add `#[derive(strum::EnumIter)]` to both enums
**Priority:** CRITICAL

### 3. ❌ Missing derives on structs
**Location:** `src/app.rs` - `ContentRow`, `EditBuffer`, `App`
**Issue:** Data structures missing standard derives
- `ContentRow`: Has `Debug, Clone` but could add `PartialEq`
- `EditBuffer`: Has `Debug, Clone` but could add `PartialEq, Eq`
- `App`: Only has implicit derives, missing `Debug`
**Fix:** Add all applicable derives per CLAUDE.md policy
**Priority:** CRITICAL

### 4. ❌ Missing observability
**Location:** All public functions in `src/app.rs`
**Issue:** No `#[instrument]` attributes on public functions, minimal tracing
**Fix:** Add `#[instrument]` to all public methods and functions per CLAUDE.md requirements
**Priority:** CRITICAL

### 5. ❌ Missing documentation
**Location:** Multiple locations
**Issue:** Several public types and methods lack documentation
- `AppMode` variants need docs
- `EditField` variants need docs
- `ContentRow` fields need docs
- `EditBuffer` needs better docs
- Many public methods lack documentation
**Fix:** Add comprehensive documentation
**Priority:** CRITICAL

## High Priority Issues

### 6. ❌ Events enum missing derives
**Location:** `src/events.rs` - `Event` enum
**Issue:** Only has `Debug`, missing `Clone`, `PartialEq`, `Eq`
**Fix:** Add all applicable derives
**Priority:** HIGH

### 7. ❌ EventHandler missing derives
**Location:** `src/events.rs` - `EventHandler` struct
**Issue:** Missing `Debug`, `Clone` derives
**Fix:** Add derives
**Priority:** HIGH

### 8. ❌ Error handling uses manual construction
**Location:** Throughout `src/app.rs`, `src/events.rs`
**Issue:** Manual error construction instead of using `?` operator cleanly
**Example:** `BotticelliError::from(TuiError::new(...))`
**Fix:** Should use proper From implementations and `?` operator
**Priority:** HIGH

## Medium Priority Issues

### 9. ⚠️ Views module is empty
**Location:** `src/views.rs`
**Issue:** Module exists but contains only a comment
**Fix:** Either populate with view-specific logic or remove if not needed
**Priority:** MEDIUM

### 10. ⚠️ Incomplete edit mode implementation
**Location:** `src/app.rs` line 419
**Issue:** TODO comment for text input handling in edit mode
**Fix:** Implement or document as future work
**Priority:** MEDIUM

### 11. ⚠️ PgConnection not wrapped
**Location:** `src/app.rs` - `App` struct
**Issue:** Direct use of `diesel::PgConnection` instead of abstraction
**Fix:** Consider using database trait from `botticelli_database`
**Priority:** MEDIUM

## Low Priority Issues

### 12. ℹ️ Inconsistent import style
**Location:** `src/app.rs`
**Issue:** Mix of crate-level and module imports
**Fix:** Use consistent crate-level imports per CLAUDE.md
**Priority:** LOW

### 13. ℹ️ Magic numbers
**Location:** `src/app.rs` line 122 (limit of 1000)
**Issue:** Hard-coded limits without constants
**Fix:** Define constants for magic numbers
**Priority:** LOW

## Compliance Summary

### ✅ Compliant
- Module organization (lib.rs only has mod/use statements)
- Private mod declarations
- Public type exports
- No wildcard imports
- No unsafe code

### ❌ Non-Compliant
- Error handling pattern (re-exports instead of crate-specific)
- Missing derives on enums and structs
- Missing `#[instrument]` attributes
- Incomplete documentation
- Manual error construction

### ⚠️ Partial Compliance
- Observability (basic but not comprehensive)
- Import patterns (mostly correct)

## Recommended Action Plan

1. **Define TUI-specific error types** (remove re-exports)
2. **Add all missing derives** to enums and structs
3. **Add `#[instrument]` to all public functions**
4. **Complete documentation** for all public items
5. **Add missing derives** to Event and EventHandler
6. **Clean up error handling** to use proper From impls
7. **Consider removing or populating** views.rs module
8. **Define constants** for magic numbers
