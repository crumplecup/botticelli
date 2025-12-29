# RMCP Implementation Plan - CLAUDE.md Compliance Critique

**Date:** 2024-12-29  
**Status:** Standards Review

## Critical Violations in Original Plan

### 1. ❌ Module Organization Violations

**Violation:** Step 2 creates `rmcp_types.rs` as a single types module

**CLAUDE.md Rule:**
- lib.rs: Only `mod` and `pub use` statements
- Module declarations: Private (not `pub mod`)
- Crate-level exports: Re-export all public types at crate root

**Problem:**
```rust
// ❌ Original plan:
// src/lib.rs
mod rmcp_types;
pub use rmcp_types::*;  // Wildcard re-export

// src/rmcp_types.rs - Single file with all types
pub struct EchoParams { }
pub struct EchoResult { }
pub struct GenerateParams { }
// ... 30+ types in one file
```

**Correct Approach:**
```rust
// ✅ Proper structure:
// src/lib.rs
mod echo;
mod generate;
mod server_info;

pub use echo::{EchoParams, EchoResult};
pub use generate::{GenerateParams, GenerateResult};
pub use server_info::ServerInfoResult;

// src/echo.rs - One file per domain/tool
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct EchoParams { }

#[derive(Serialize, JsonSchema)]
pub struct EchoResult { }

// src/generate.rs
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct GenerateParams { }

#[derive(Serialize, JsonSchema)]
pub struct GenerateResult { }
```

**Impact:**
- No wildcard imports
- Each domain has its own module
- Clear ownership boundaries
- Easy to navigate

---

### 2. ❌ Missing Instrumentation

**Violation:** No `#[instrument]` on any tool methods

**CLAUDE.md Rule:**
> All public functions have `#[instrument]`. Missing instrumentation is a defect.

**Problem:**
```rust
// ❌ Original plan:
#[tool]
async fn echo(&self, params: Parameters<EchoParams>) -> Result<...> {
    Ok(Json(EchoResult { ... }))
}
```

**Correct Approach:**
```rust
// ✅ With instrumentation:
#[tool(description = "Echoes back the input message")]
#[instrument(skip(self), fields(message = %params.0.message))]
async fn echo(
    &self,
    Parameters(params): Parameters<EchoParams>
) -> Result<Json<EchoResult>, String> {
    debug!("Processing echo request");
    
    let result = EchoResult {
        echo: params.message,
        timestamp: Utc::now().to_rfc3339(),
    };
    
    debug!(result = ?result, "Echo completed");
    Ok(Json(result))
}
```

**Impact:**
- Every tool call traced
- Parameters logged (with privacy controls)
- Errors automatically captured
- Performance monitoring enabled

---

### 3. ❌ Missing Standard Derives

**Violation:** Incomplete derive list on types

**CLAUDE.md Rule:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MyType { /* ... */ }
```

**Problem:**
```rust
// ❌ Original plan:
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EchoParams {
    pub message: String,
}
```

**Correct Approach:**
```rust
// ✅ Full standard derives:
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct EchoParams {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct EchoResult {
    pub echo: String,
    pub timestamp: String,
}
```

**When to omit:**
- `Hash` if contains floats
- `PartialOrd`/`Ord` if no natural ordering
- `PartialEq`/`Eq` if comparison doesn't make sense

---

### 4. ❌ Missing Documentation

**Violation:** Types lack proper documentation

**CLAUDE.md Rule:**
> Required: All public items (enforced by `#![warn(missing_docs)]`)

**Problem:**
```rust
// ❌ Original plan:
pub struct EchoParams {
    /// The message to echo back
    pub message: String,
}
```

**Correct Approach:**
```rust
// ✅ Complete documentation:
/// Parameters for the echo tool.
///
/// This tool echoes back the provided message with a timestamp,
/// useful for testing MCP connectivity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct EchoParams {
    /// The message to echo back.
    ///
    /// This can be any UTF-8 string. The server will return it
    /// unchanged along with a timestamp.
    pub message: String,
}

/// Result from the echo tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct EchoResult {
    /// The echoed message (same as input).
    pub echo: String,
    
    /// ISO 8601 timestamp when the echo was processed.
    pub timestamp: String,
}
```

---

### 5. ❌ Builder Pattern Not Used

**Violation:** Server construction uses `new()` instead of builder

**CLAUDE.md Rule:**
> Always use builders for struct construction. Never use struct literals.

**Problem:**
```rust
// ❌ Original plan:
impl BotticelliServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
    
    pub fn with_database(mut self, db: ...) -> Self {
        self.db_ops = Some(db);
        self
    }
}

// Usage:
let server = BotticelliServer::new()
    .with_database(db)
    .with_dialog(dialog);
```

**Correct Approach:**
```rust
// ✅ Builder pattern:
use derive_builder::Builder;

#[derive(Clone, Builder)]
#[builder(pattern = "owned")]
pub struct BotticelliServer {
    #[builder(default = "Self::tool_router()")]
    tool_router: ToolRouter<Self>,
    
    #[builder(default)]
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
    
    #[builder(default)]
    dialog: Option<Arc<DialogResource>>,
}

// Usage:
let server = BotticelliServerBuilder::default()
    .db_ops(Some(db))
    .dialog(Some(dialog))
    .build()
    .expect("Valid server config");
```

**Problem with builder on this type:**
- `tool_router` needs `Self::tool_router()` which requires type to exist
- Circular dependency with `#[tool_router]` macro

**Best Approach:**
```rust
// ✅ Manual builder for this case:
impl BotticelliServer {
    /// Create a builder for configuring the server.
    pub fn builder() -> BotticelliServerBuilder {
        BotticelliServerBuilder::default()
    }
}

/// Builder for BotticelliServer.
#[derive(Default)]
pub struct BotticelliServerBuilder {
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
    dialog: Option<Arc<DialogResource>>,
}

impl BotticelliServerBuilder {
    #[cfg(feature = "database")]
    pub fn database(mut self, db: Arc<dyn DatabaseRegistryOperations>) -> Self {
        self.db_ops = Some(db);
        self
    }
    
    pub fn dialog(mut self, dialog: Arc<DialogResource>) -> Self {
        self.dialog = Some(dialog);
        self
    }
    
    pub fn build(self) -> BotticelliServer {
        BotticelliServer {
            tool_router: BotticelliServer::tool_router(),
            #[cfg(feature = "database")]
            db_ops: self.db_ops,
            dialog: self.dialog,
        }
    }
}
```

---

### 6. ❌ Workflow Violations

**Violation:** No commit strategy per step

**CLAUDE.md Rule:**
> For each step:
> - Generate code
> - Fix cargo check errors/warnings
> - Run all checks
> - Commit with audit-friendly message
> - Push to branch

**Problem:**
- Plan says "Step 1: Add dependency" but doesn't specify commit
- No validation commands per step
- No "fix all warnings" reminder

**Correct Approach:**

Each step should include:

```markdown
### Step X: [Name]

**Actions:**
[code to write]

**Validation:**
```bash
just check botticelli_mcp
just test-package botticelli_mcp
# Fix all warnings before proceeding
```

**Commit:**
```bash
git add -A
git commit -m "feat(mcp): [description]

- Bullet points
- Of changes

Validation:
- cargo check passes
- All tests pass
- Zero clippy warnings"
```
```

---

### 7. ❌ Missing Error Handling Standards

**Violation:** Tools return `String` errors

**CLAUDE.md Rule:**
> Use derive_more::Display + derive_more::Error on all errors

**Problem:**
```rust
// ❌ Original plan:
#[tool]
async fn echo(...) -> Result<Json<EchoResult>, String> {
    // ...
}
```

**Correct Approach:**
```rust
// ✅ Proper error type:
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
pub enum ToolError {
    #[display("Dialog not configured")]
    DialogNotConfigured,
    
    #[display("Database not configured")]
    DatabaseNotConfigured,
    
    #[display("Elicitation failed: {}", _0)]
    ElicitationFailed(String),
}

#[tool]
async fn echo(...) -> Result<Json<EchoResult>, ToolError> {
    // ...
}
```

**OR use existing error types:**
```rust
use botticelli_error::McpError;

#[tool]
async fn echo(...) -> Result<Json<EchoResult>, McpError> {
    // ...
}
```

---

### 8. ❌ Testing Violations

**Violation:** Tests in wrong location

**CLAUDE.md Rule:**
> `#[cfg(test)] mod tests` in source files is not allowed. All tests go in `tests/` directory.

**Problem:**
```rust
// ❌ Original plan:
// Step 4 suggests inline test:
#[tokio::test]
async fn test_rmcp_echo() {
    let server = BotticelliServer::new();
    // ...
}
```

**Correct Approach:**
```rust
// ✅ In tests/ directory:
// tests/echo_tool_test.rs
use botticelli_mcp::{BotticelliServer, EchoParams};

#[tokio::test]
async fn test_echo_tool_basic() {
    let server = BotticelliServer::builder().build();
    // ...
}

#[tokio::test]
async fn test_echo_tool_with_special_chars() {
    // ...
}
```

---

### 9. ❌ Feature Flag Handling

**Violation:** No feature gate validation

**CLAUDE.md Rule:**
> Verification:
> ```bash
> just check-features  # All combinations
> ```

**Problem:**
- Plan doesn't mention testing feature combinations
- Step 9 adds `#[cfg(feature = "database")]` but doesn't validate

**Correct Approach:**

Add to each step that touches feature-gated code:

```markdown
**Validation:**
```bash
just check botticelli_mcp
just check botticelli_mcp --features database
just check botticelli_mcp --no-default-features
just check-features  # Before final commit
```
```

---

### 10. ❌ Import Organization

**Violation:** Doesn't specify import patterns

**CLAUDE.md Rule:**
> Imports: Always `use crate::{Type}`, never `use crate::module::Type`

**Problem:**
```rust
// ❌ What plan might produce:
use crate::rmcp_types::{EchoParams, EchoResult};
```

**Correct Approach:**
```rust
// ✅ Crate-level imports:
use crate::{EchoParams, EchoResult};
```

**Impact on step 2:**
- Must export types at crate level in lib.rs
- All internal imports use crate-level paths
- No module paths in imports

---

## Corrected Implementation Plan Structure

### Required Changes

**1. Split rmcp_types.rs into domain modules**
```
src/
├── lib.rs           # Only mod + pub use
├── server.rs        # BotticelliServer struct
├── echo.rs          # EchoParams, EchoResult
├── generate.rs      # GenerateParams, GenerateResult
├── server_info.rs   # ServerInfoResult
└── errors.rs        # ToolError types
```

**2. Add instrumentation to all tool methods**
```rust
#[instrument(skip(self), fields(...))]
async fn tool_name(...) -> ... {
    debug!("Starting...");
    // ... implementation
    debug!("Completed");
}
```

**3. Use proper error types**
- Define `ToolError` enum
- Use `derive_more::Display` + `derive_more::Error`
- Include `#[track_caller]` on constructors

**4. Complete documentation**
- Type-level docs
- Field-level docs
- Method-level docs
- Examples where helpful

**5. Standard derives everywhere**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
```

**6. Builder for BotticelliServer**
- Manual builder implementation
- Validate configuration in `build()`

**7. Validation per step**
```bash
just check botticelli_mcp
just test-package botticelli_mcp
# Fix all warnings
```

**8. Commit per step**
- Descriptive message
- Bullet points of changes
- Validation confirmation

**9. Tests in tests/ directory**
- No inline `#[cfg(test)]`
- One file per domain: `echo_tool_test.rs`
- Import from crate root: `use crate::{Type}`

**10. Feature flag validation**
```bash
just check-features
```

---

## Updated Step Example

**Step 2: Create Echo Tool Types**

**Create:** `crates/botticelli_mcp/src/echo.rs`

```rust
//! Echo tool types and implementation.
//!
//! The echo tool provides a simple test endpoint that returns
//! the input message with a timestamp.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Parameters for the echo tool.
///
/// This tool echoes back the provided message with a timestamp,
/// useful for testing MCP connectivity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct EchoParams {
    /// The message to echo back.
    ///
    /// This can be any UTF-8 string. The server will return it
    /// unchanged along with a timestamp.
    pub message: String,
}

/// Result from the echo tool.
///
/// Contains the echoed message and the timestamp when it was processed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct EchoResult {
    /// The echoed message (same as input).
    pub echo: String,
    
    /// ISO 8601 timestamp when the echo was processed.
    pub timestamp: String,
}

impl EchoResult {
    /// Create a new echo result with current timestamp.
    pub fn new(message: String) -> Self {
        Self {
            echo: message,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}
```

**Update:** `crates/botticelli_mcp/src/lib.rs`

```rust
mod echo;

pub use echo::{EchoParams, EchoResult};
```

**Validation:**
```bash
just check botticelli_mcp
# Should compile, no warnings
```

**Commit:**
```bash
git add src/echo.rs src/lib.rs
git commit -m "feat(mcp): Add echo tool type definitions

Add strongly-typed parameters and results for echo tool:
- EchoParams: Input message parameter
- EchoResult: Output with message and timestamp
- Helper constructor for timestamp generation

Standards compliance:
- Full standard derives (Debug, Clone, PartialEq, Eq, Hash)
- Complete documentation on all public items
- Crate-level exports in lib.rs
- Module per domain pattern

Validation:
- cargo check passes
- Zero clippy warnings
- Follows CLAUDE.md guidelines"
```

---

## Timeline Impact

**Original estimate:** 2-3 days

**With standards compliance:** 3-4 days
- +1 day for proper documentation
- +0.5 day for proper module structure
- +0.5 day for instrumentation and validation

**Worth it because:**
- Code is maintainable from day one
- No tech debt accumulation
- Easier review and merge
- Sets pattern for future tools

---

## Recommendation

**Before proceeding:**
1. Update implementation plan with corrected structure
2. Add validation steps to each stage
3. Include commit messages per step
4. Add checklist for CLAUDE.md compliance
5. Then begin Step 1

**This ensures:**
- No refactoring needed after migration
- Clean, maintainable codebase
- Standards enforced from start
- Audit trail in commits

---

*Document created: 2024-12-29*  
*Status: Standards Compliance Review*  
*Recommendation: Update plan before implementation*
