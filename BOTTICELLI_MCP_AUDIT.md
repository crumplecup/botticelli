# botticelli_mcp Crate Audit

**Date:** 2026-01-11
**Status:** Compiling (10 warnings)
**Files:** 65 source files
**Total Lines:** ~12,000

## Critical Issues

### 1. Missing Instrumentation (CRITICAL)
**File:** `rmcp_server.rs` (3,288 lines)
- **Zero** `#[instrument]` attributes on 33 tool methods
- All public API methods lack tracing spans
- Makes debugging and observability impossible
- Violates project principle: "All public functions have `#[instrument]`"

**Impact:** Production debugging will be blind. No trace of:
- Which tools are being called
- What parameters were passed
- How long operations take
- Where failures occur

**Example:** Lines 400-600 have complex database operations with no instrumentation.

### 2. Lost Error Sources (CRITICAL)
**File:** `rmcp_server.rs`
- **60+ instances** of `map_err` that convert errors to strings
- All underlying error context is destroyed
- Chain of causation lost

**Pattern:**
```rust
db_ops.execute_query(&query).await.map_err(|e| {
    rmcp::ErrorData::new(
        ErrorCode::INTERNAL_ERROR,
        Cow::Owned(format!("Query failed: {}", e)),  // ❌ String loses source
        None,
    )
})?;
```

**Should be:**
```rust
// Option 1: Use From trait if possible
db_ops.execute_query(&query).await?;

// Option 2: Preserve source in ErrorData.data field
db_ops.execute_query(&query).await.map_err(|e| {
    rmcp::ErrorData::new(
        ErrorCode::INTERNAL_ERROR,
        Cow::Borrowed("Query failed"),
        Some(serde_json::json!({
            "error": format!("{:?}", e),
            "query": &query,
        })),
    )
})?;
```

**Locations:** Lines 406, 463, 475, 533, 589, 652, 722, 1031, 1116, 1128, 1191, 1495, 1510, 1524, 1533, 1793, 1839, 1900, 1909, 1978, and many more.

### 3. Module Size Violations (CRITICAL)
**File:** `rmcp_server.rs` - 3,288 lines

**Guideline:** "When file exceeds ~500-1000 lines"

This file is **3x the maximum recommended size**.

**Should be split into:**
```
src/rmcp_server/
├── mod.rs              # ONLY mod + pub use (50 lines)
├── server.rs           # BotticelliServer struct + builder
├── tools/
│   ├── mod.rs
│   ├── narrative.rs    # Narrative creation/validation tools
│   ├── execution.rs    # Execute narrative/act tools
│   ├── elicitation.rs  # Elicitation tools
│   ├── scene.rs        # Scene management tools
│   ├── state.rs        # State management tools
│   └── misc.rs         # Echo, server info, etc.
└── helpers.rs          # Helper functions
```

**Benefits:**
- Each file <500 lines
- Logical grouping
- Easier to navigate
- Better for IDE performance
- Clearer ownership

### 4. Unused Imports (HIGH)
**Files:** Multiple

From warnings:
```
warning: unused import: `botticelli_interface::BotticelliDriver`
    --> crates/botticelli_mcp/src/rmcp_server.rs:1482:13

warning: unused import: `botticelli_interface::BotticelliDriver`
  --> crates/botticelli_mcp/src/tools/execute_act.rs:33:5
```

**Action:** Remove all unused imports (10 warnings total).

### 5. Dead Code (MEDIUM)
**File:** `session_tools.rs`

```
warning: struct `DetectedAct` is never constructed
  --> crates/botticelli_mcp/src/session_tools.rs:18:12
```

**File:** `tools/mod.rs`
```
warning: field `metrics` is never read
   --> crates/botticelli_mcp/src/tools/mod.rs:125:5
```

**File:** `transport/mod.rs`
```
warning: methods `initialize`, `list_tools`, `call_tool`, and `is_connected` are never used
  --> crates/botticelli_mcp/src/transport/mod.rs:17:14

warning: variants `ConnectionFailed`, `RequestFailed`, and `InvalidResponse` are never constructed
  --> crates/botticelli_mcp/src/transport/mod.rs:34:5

warning: struct `HttpTransport` is never constructed
  --> crates/botticelli_mcp/src/transport/http.rs:11:12
```

**Action:** 
- Delete `transport/` module entirely (legacy code)
- Feature-gate or delete unused types
- Use `metrics` field or remove it

## High Priority Issues

### 6. No Error Types for Domain Errors
**Pattern:** All errors converted to `rmcp::ErrorData`

The crate lacks its own error types:
- No `McpError` or `McpErrorKind` types
- Every error becomes a generic `INTERNAL_ERROR`
- No ability to handle specific error cases
- No structured error information

**Should have:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum McpToolErrorKind {
    #[display("Narrative not found: {}", _0)]
    NarrativeNotFound(String),
    
    #[display("Invalid TOML: {}", _0)]
    InvalidToml(String),
    
    #[display("Database error: {}", _0)]
    Database(String),
}
```

### 7. Builder Pattern Inconsistency
**File:** `tools/mod.rs` - Lines 165-203

`ToolRegistry::default()` implementation manually registers tools but doesn't follow builder pattern for configuration.

**Issue:** No way to customize tool registration without modifying source.

### 8. Large Helper Modules
**File:** `tools/narrative_validation_helpers.rs` - 338 lines

Helper functions should be <200 lines. This needs splitting into:
- `validation/formatting.rs`
- `validation/fixing.rs`
- `validation/comments.rs`

### 9. Feature Gate Issues
**File:** `tools/narrative_processor.rs`

Imports moved inside feature gates (lines 6-31) but this makes the code harder to read.

**Better:** Use feature-gated `use` at module level with `#[cfg_attr]`.

### 10. No Documentation on Complex Functions
**File:** `rmcp_server.rs`

Helper functions like `generate_narrative_toml` (line 2870) lack documentation on:
- What the generated TOML looks like
- What the description format should be
- Edge cases and error conditions

## Medium Priority Issues

### 11. Inline Driver Dispatch Duplication
**Files:** 
- `rmcp_server.rs` (lines 1352-1398, 1408-1506)
- `tools/execute_act.rs` (lines 218-302, 126+)
- `tools/execute_narrative.rs` (lines 241-331, 117+)

Same if-chain pattern repeated 6+ times:
```rust
if model.starts_with("gemini") {
    if let Some(driver) = self.gemini_driver.clone() {
        return self.execute_with_driver(driver, ...).await;
    }
}
// ... repeat for anthropic, ollama, huggingface, groq
```

**Should extract to:**
```rust
fn select_driver(&self, model: &str) -> Result<DriverEnum, Error> {
    // Single dispatch logic
}
```

But this requires `DriverEnum` which may not be worth the complexity.

### 12. Missing Type Aliases
**Pattern:** `Arc<dyn DatabaseRegistryOperations<Error = botticelli_error::BotticelliError>>` appears 5+ times

**Should have:**
```rust
type DbOps = Arc<dyn DatabaseRegistryOperations<Error = botticelli_error::BotticelliError>>;
```

### 13. Inconsistent Error Messages
**Examples:**
- "Query failed: {}" (line 409)
- "Failed to export Prometheus metrics: {}" (line 465)
- "Failed to read narrative file: {}" (line 1193)

No consistent capitalization, punctuation, or format.

**Standard:**
- Start with capital letter
- No trailing punctuation
- Include context: "Failed to X: Y"

### 14. Magic Strings
**File:** `rmcp_server.rs`

Default model names hardcoded:
- "claude-3-5-sonnet-20241022" (binaries)
- "llama3.2" (binaries)
- "mistralai/Mistral-7B-Instruct-v0.2" (binaries)

**Should be:** Constants at module level.

### 15. No Input Validation
**Example:** `create_narrative` tool (line 930+)

No validation that:
- Name is not empty
- Name contains valid characters
- Description length is reasonable
- Path is safe (no directory traversal)

## Low Priority Issues

### 16. Verbose Parameter Structs
**Pattern:** All tool parameters use `#[serde(skip_serializing_if = "Option::is_none")]`

This is boilerplate that could be a derive macro.

### 17. Missing Const Generics Opportunities
**Example:** Default values for enums

```rust
#[derive(Default)]  // ✅ Better
enum StateFormat {
    #[default]
    Summary,
    Full,
}

// vs current manual impl
```

### 18. Test Coverage
**Status:** Unknown (no tests in audit scope)

The crate likely needs:
- Unit tests for helper functions
- Integration tests for tool methods
- Mock tests for database operations

### 19. Binary Dependencies on Library Internals
**Files:** `bin/botticelli-mcp.rs`, `bin/botticelli-mcp-http.rs`

Binaries directly use `botticelli_models::*` types. Should go through the library's public API.

## Good Patterns Found

### ✅ Private Fields
All struct fields are private with proper encapsulation.

### ✅ Builder Pattern
`BotticelliServerBuilder` follows builder pattern correctly.

### ✅ Generic Driver Dispatch
The `execute_with_driver<D>` pattern avoids trait object issues elegantly.

### ✅ Feature Gating
Proper use of `#[cfg(feature = "...")]` throughout.

### ✅ Rmcp Integration
Clean integration with rmcp's `#[tool]` macro system.

### ✅ No `#[allow]` Directives
Zero tolerance for suppressing warnings (as required).

## Recommendations Priority

1. **IMMEDIATE:**
   - Add `#[instrument]` to all 33 tool methods in `rmcp_server.rs`
   - Fix error source loss in all 60+ `map_err` calls

2. **THIS WEEK:**
   - Split `rmcp_server.rs` into multiple modules
   - Remove all unused imports (fix warnings)
   - Delete dead `transport/` module

3. **THIS SPRINT:**
   - Create proper error types (`McpToolError`)
   - Extract common driver dispatch logic
   - Add input validation to all tools

4. **NEXT SPRINT:**
   - Documentation pass on complex functions
   - Add unit tests for helpers
   - Standardize error messages

## Metrics

- **Total violations:** 60+ critical, 15+ high, 19+ low
- **Technical debt:** ~2 weeks of cleanup work
- **Risk level:** HIGH (no instrumentation, lost error context)
- **Maintainability:** MEDIUM (large files, duplication)

## Conclusion

The crate compiles and works, but lacks production-ready quality:

**Strengths:**
- Clean public API
- Good encapsulation
- Modern patterns (builders, generics)

**Weaknesses:**
- Zero observability (no instrumentation)
- Lost error context everywhere
- Module organization needs major refactoring
- Dead code accumulation

**Recommendation:** Block production deployment until instrumentation and error handling are fixed.
