# Botticelli MCP CLAUDE.md Compliance Audit

**Date:** 2026-01-18  
**Crate:** botticelli_mcp  
**Status:** SIGNIFICANT NON-COMPLIANCE DETECTED

## Executive Summary

This is the THIRD audit requested. A comprehensive audit reveals **widespread violations** of CLAUDE.md standards across multiple categories. The crate requires extensive refactoring to meet project standards.

### Critical Issues Found:

1. **Public Fields Everywhere** - 50+ DTO structs with public fields (should use getters/setters)
2. **Zero Error Types** - No error types in crate (violates error architecture)
3. **Error Chain Loss** - 20+ locations casting errors to string with `.to_string()`
4. **Missing Instrumentation** - 149+ public methods, only 23 instrumented (~15%)
5. **Traits in Crate** - 1 trait found (should be in _interface)
6. **No Getters/Setters** - Only 4 uses of derive_getters/setters (should be 50+)

---

## Violation Category 1: Public Fields (CRITICAL)

**CLAUDE.md Rule:** "Private fields + derive-based access"

### Violations Found: 50+ structs

All parameter and result types have public fields instead of using derive_getters/derive_setters:

**Files with violations:**
- `conversation.rs` - Attachment, ConversationTurn
- `create_narrative.rs` - CreateNarrativeParams
- `discord_tools.rs` - ALL Discord params/results (10+ structs)
- `echo.rs` - EchoParams, EchoResult
- `elicit_*.rs` - All elicitation params/results (8 structs)
- `execution.rs` - ExecuteActParams, ExecuteNarrativeParams, GenerateParams, etc.
- `export_metrics.rs` - ExportMetricsParams
- `modify_narrative.rs` - ModifyNarrativeParams, ModifyNarrativeResult
- `query_content.rs` - QueryContentParams, QueryContentResult
- `save_narrative.rs` - SaveNarrativeParams, SaveNarrativeResult
- `scene.rs` - All scene params/results (8 structs)
- `session_tools.rs` - ALL session tool params/results (16+ structs)
- `validate_narrative.rs` - ValidateNarrativeParams, ValidationError, ValidationWarning

**Example violation:**
```rust
// ❌ BAD: Public fields
pub struct CreateNarrativeParams {
    pub description: String,
    pub name: String,
    pub default_model: Option<String>,
    pub default_temperature: Option<f64>,
}

// ✅ GOOD: Private fields + derives
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, derive_getters::Getters)]
pub struct CreateNarrativeParams {
    description: String,
    name: String,
    default_model: Option<String>,
    default_temperature: Option<f64>,
}
```

**Impact:** Breaking API change for ALL tools. Every struct needs:
1. Make fields private
2. Add `#[derive(derive_getters::Getters)]`
3. For mutable structs, add `#[derive(derive_setters::Setters)]` with `#[setters(prefix = "with_")]`
4. Update ALL usage sites (50+ files)

---

## Violation Category 2: Error Types (CRITICAL)

**CLAUDE.md Rule:** "Error types need to live in _error"

### Violations Found: ZERO error types in crate

The crate has NO error types. All errors are either:
- Using `rmcp::ErrorData` directly
- Using `BotticelliError` from _error crate
- Casting to strings and losing error chain

**Missing error types:**
- No `McpError` / `McpErrorKind` (should exist for MCP-specific errors)
- No `ToolExecutionError` (should wrap tool failures)
- No `ValidationError` types (using raw strings instead)

**What should exist:**

```rust
// In botticelli_error/src/mcp.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum McpErrorKind {
    #[display("Tool not found: {}", _0)]
    ToolNotFound(String),
    
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),
    
    #[display("Execution failed: {}", _0)]
    ExecutionFailed(String),
    
    // Should capture source errors, not strings!
}

#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("MCP: {} at {}:{}", kind, file, line)]
pub struct McpError {
    kind: McpErrorKind,
    line: u32,
    file: &'static str,
}
```

---

## Violation Category 3: Error Chain Loss (CRITICAL)

**CLAUDE.md Rule:** "Never convert Surf → Surf" and "Capture source errors"

### Violations Found: 20+ locations

**File: tools/mod.rs** (ToolRegistry execute method)
```rust
// ❌ BAD: Loses error chain
.map_err(|e| McpError::invalid_input(e.to_string()))?

// ❌ BAD: Loses error chain  
.map_err(|e| McpError::execution_failed(e.message.to_string()))
```

**File: tools/elicitation/registry.rs**
```rust
// ❌ BAD: Loses error chain
.map_err(|e| McpError::execution_failed(e.to_string()))?

// ❌ BAD: Double conversion
botticelli_error::BotticelliError::from(McpError::execution_failed(e.to_string()))
```

**Impact:** 
- All error context lost at tool boundaries
- Debugging becomes impossible
- Violates "Social Mobility" error pattern

**Fix:** Capture source errors properly:
```rust
// ✅ GOOD: Capture source
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub struct SerdeJsonError {
    source: Box<serde_json::Error>,
    line: u32,
    file: &'static str,
}
```

---

## Violation Category 4: Missing Instrumentation (CRITICAL)

**CLAUDE.md Rule:** "All functions have `#[instrument]`"

### Statistics:
- **Public methods:** 149+
- **Instrumented:** 23
- **Coverage:** ~15%
- **Target:** 100%

### Files with ZERO instrumentation:
- `create_narrative.rs`
- `discord_tools.rs`
- `echo.rs`
- `elicit_bool.rs`, `elicit_number.rs`, `elicit_select.rs`, `elicit_text.rs`
- `execution.rs`
- `export_metrics.rs`
- `modify_narrative.rs`
- `query_content.rs`
- `save_narrative.rs`
- `scene.rs`
- `server_info.rs`
- `session_tools.rs`

**These files ONLY contain type definitions** - but the types are used in `rmcp_server/` which IS instrumented. However, CLAUDE.md is clear: if there are functions (even simple constructors), they need instrumentation.

**Impact:** Severely limits observability and debugging capability.

---

## Violation Category 5: Traits in Crate (MINOR)

**CLAUDE.md Rule:** "Traits if there are any need to move to _interface"

### Violations Found: 1 trait

**File: resources/mod.rs**
```rust
pub trait McpResource: Send + Sync {
    fn uri(&self) -> String;
    fn name(&self) -> String;
    fn description(&self) -> Option<String>;
    fn mime_type(&self) -> Option<String>;
    fn contents(&self) -> Result<Vec<u8>, std::io::Error>;
}
```

**Fix:** Move to `botticelli_interface/src/mcp_resource.rs`

---

## Violation Category 6: Manual Impls vs Derives (MINOR)

**CLAUDE.md Rule:** "Use derive_more for Display, Error"

### Good news: No manual impls found!

Searched for:
- `impl Display for` - 0 results
- `impl Error for` - 0 results

This is compliant. ✅

---

## Additional Issues

### Issue 7: Missing #[track_caller] on constructors

Many `new()` methods don't have `#[track_caller]` for location tracking in errors.

### Issue 8: Inconsistent derive order

Some structs have derives in random order instead of standard:
```rust
// ✅ GOOD: Standard order
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
```

### Issue 9: Missing documentation

Several public types lack doc comments (though `#![warn(missing_docs)]` should catch this).

---

## Prioritized Fix Plan

### Phase 1: Error Architecture (CRITICAL - 2 days)

1. Create `McpError` / `McpErrorKind` in `botticelli_error`
2. Add variants for all MCP-specific errors
3. Create proper source error wrappers (SerdeJsonError, RmcpError, etc.)
4. Update umbrella error to include MCP errors

### Phase 2: Fix Error Chains (CRITICAL - 1 day)

1. Audit all `.to_string()` on errors
2. Replace with proper error capture
3. Update `tools/mod.rs` ToolRegistry error handling
4. Update `tools/elicitation/registry.rs`

### Phase 3: Add Getters/Setters (CRITICAL - 3 days)

1. Add derives to ALL param/result structs (50+ files)
2. Make fields private
3. Update ALL usage sites
4. Run tests to verify no breakage

### Phase 4: Add Instrumentation (HIGH - 2 days)

1. Add `#[instrument]` to all public methods
2. Add appropriate `skip()` and `fields()` attributes
3. Verify spans appear in logs

### Phase 5: Move Trait (LOW - 1 hour)

1. Move `McpResource` to `botticelli_interface`
2. Update imports

### Total Estimated Time: 8-9 days

---

## Test Before/After

Before fix:
```bash
cargo clippy -- -D warnings  # Should show issues
cargo check                  # Compiles but incorrect
```

After fix:
```bash
cargo clippy -- -D warnings  # Clean
cargo check                  # Compiles correctly
just test-package botticelli_mcp  # All tests pass
```

---

## Compliance Scorecard

| Category | Status | Count | Target | %
|----------|--------|-------|--------|---
| Public Fields | ❌ FAIL | 50+ | 0 | 0%
| Error Types | ❌ FAIL | 0 | 1+ | 0%
| Error Chains | ❌ FAIL | 20+ | 0 | 0%
| Instrumentation | ❌ FAIL | 23/149 | 149/149 | 15%
| Traits in Interface | ❌ FAIL | 1 | 0 | 0%
| Manual Impls | ✅ PASS | 0 | 0 | 100%
| **OVERALL** | **❌ FAIL** | | | **~19%**

---

## Conclusion

The crate is **severely non-compliant** with CLAUDE.md standards. This is the third audit request, and the issues are systemic rather than isolated. A comprehensive refactor is required across:

- Error handling architecture
- Type encapsulation  
- Observability
- Architectural boundaries

Estimated 8-9 days of focused work to bring to full compliance.
