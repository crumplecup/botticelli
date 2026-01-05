# Botticelli Narrative Crate - Fresh Comprehensive Audit

**Date:** 2026-01-05
**Status:** ✅ Compiles Successfully

## Summary

The _narrative crate compiles cleanly but has significant observability and code quality gaps.

---

## Critical Issues

### 1. **Missing Instrumentation (CRITICAL)** ⚠️

**Impact:** Zero observability - impossible to debug production issues

**Findings:**
- ❌ **ZERO** public functions have `#[instrument]` attributes
- ❌ ~90 public functions across 17 files completely uninstrumented
- ❌ No span tracking for narrative execution flow
- ❌ No structured logging for database operations
- ❌ No error event logging before returns

**Files Requiring Instrumentation:**
- `executor.rs` (10 public functions)
- `carousel.rs` (10 public functions) 
- `provider.rs` (10 public functions)
- `state.rs` (11 public functions)
- `core.rs` (13 public functions)
- `toml_parser.rs` (6 public functions)
- `validator.rs` (10 public functions)
- `processor.rs` (6 public functions)
- `extraction.rs` (4 public functions) - **Has manual tracing but no `#[instrument]`**
- `content_generation.rs` (1 public function)
- `history_retention.rs` (3 public functions)
- `storage_actor.rs` (1 public function)
- `filesystem_storage.rs` (1 public function)
- `multi_narrative.rs` (3 public functions)
- `in_memory_repository.rs` (4 public functions)
- `table_reference.rs` (2 public functions)

**Action Items:**
- [ ] Add `#[instrument]` to ALL public functions
- [ ] Skip large params (connections, data): `#[instrument(skip(conn, data))]`
- [ ] Add context fields: `#[instrument(fields(narrative_id, step_index))]`
- [ ] Emit debug/info events at key points
- [ ] Log errors before returning
- [ ] Track SQL queries at debug level

---

### 2. **expect/unwrap Usage** ⚠️

**Findings:**
- ✅ Only 1 file has expect/unwrap: `extraction.rs`
  - Lines 31, 109, 224 in doctests (acceptable in examples)

**Status:** ✅ Good - no production code uses expect/unwrap

---

### 3. **Error Handling**

**Findings:**
- ✅ Uses `BotticelliResult` appropriately
- ❌ `extraction.rs` creates errors from strings instead of capturing source errors:
  - Line 79-83: Loses context by creating BackendError from string
  - Line 125-129: Loses context by creating BackendError from string
  - Line 322-326: Loses context by creating BackendError from string
- ⚠️ Should capture `serde_json::Error` and `toml::de::Error` in error variants

**Action Items:**
- [ ] Add variants to capture serde_json and toml parse errors
- [ ] Update extraction.rs to use proper error variants

---

### 4. **Field Access Patterns**

**Good Examples:**
- ✅ `CarouselConfig` - private fields + derive_getters + derive_setters
- ✅ `NarrativeState` - private fields + derive_getters
- ✅ `TableReference` - private fields + derive_getters

**Issues:**
- ❌ Some structs may still have public fields (needs file-by-file review)

**Action Items:**
- [ ] Audit all struct definitions for public fields
- [ ] Apply derive_getters/setters where appropriate

---

### 5. **Builder Patterns**

**Findings:**
- ✅ Uses derive_builder appropriately
- ⚠️ Need to verify all builder `.build()` calls handle errors (no `.expect()`)

**Action Items:**
- [ ] Grep for `.build().expect` patterns
- [ ] Convert to proper error handling with `?`

---

### 6. **Module Organization**

**Findings:**
- ✅ `lib.rs` contains only `mod` and `pub use` statements
- ✅ Good separation of concerns across modules
- ✅ Clear module boundaries

**Status:** ✅ Excellent

---

### 7. **Documentation**

**Findings:**
- ✅ Most public items documented
- ✅ Good module-level docs
- ⚠️ Some functions could use more context

**Action Items:**
- [ ] Review docs for completeness
- [ ] Add examples where helpful

---

### 8. **Testing**

**Findings:**
- ⚠️ No test files found in audit
- ❌ Likely missing comprehensive test coverage

**Action Items:**
- [ ] Create test coverage audit document
- [ ] Identify testing gaps
- [ ] Add unit tests for core functionality
- [ ] Add integration tests for narrative execution

---

### 9. **Dependencies**

**Findings:**
- ✅ Uses workspace dependencies appropriately
- ✅ No duplicate/conflicting versions detected

**Status:** ✅ Good

---

## Priority Actions (Ordered)

### P0 - Critical (Do First)
1. **Add instrumentation to ALL public functions** - This is the biggest gap
   - Start with `executor.rs` (core execution flow)
   - Then `provider.rs` (database operations)
   - Then remaining modules
2. **Fix error handling in extraction.rs** - Capture source errors properly

### P1 - High Priority  
3. **Audit struct fields** - Ensure all use private fields + derives
4. **Test coverage audit** - Create comprehensive testing document
5. **Review builder error handling** - No `.expect()` on `.build()` calls

### P2 - Medium Priority
6. **Documentation review** - Enhance where needed
7. **Add missing tests** - Based on test coverage audit

---

## Instrumentation Pattern Example

```rust
// Before
pub fn execute_narrative(&mut self, narrative_id: &str) -> BotticelliResult<()> {
    // ... implementation
}

// After
#[instrument(skip(self), fields(narrative_id))]
pub fn execute_narrative(&mut self, narrative_id: &str) -> BotticelliResult<()> {
    debug!("Starting narrative execution");
    
    // ... implementation
    
    match result {
        Ok(_) => {
            info!("Narrative execution completed successfully");
            Ok(())
        }
        Err(e) => {
            error!(error = ?e, "Narrative execution failed");
            Err(e)
        }
    }
}
```

---

## Metrics

- **Total Source Files:** 17
- **Public Functions (approx):** ~90
- **Instrumented Functions:** 0 ❌
- **Instrumentation Coverage:** 0% ❌
- **Files with expect/unwrap:** 1 (doctests only) ✅
- **Compilation Status:** ✅ Passes

---

## Conclusion

The _narrative crate is **structurally sound** but has **ZERO observability**. This is a critical gap that must be addressed before production use. Without instrumentation, debugging narrative execution issues will be nearly impossible.

**Recommended Action:** Systematically add instrumentation following the pattern above, starting with core execution paths.
