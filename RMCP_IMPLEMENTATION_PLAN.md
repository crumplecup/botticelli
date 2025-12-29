# RMCP Migration Implementation Plan

**Date:** 2024-12-29  
**Status:** Implementation Roadmap (CLAUDE.md Compliant)  
**Branch:** `feat/rmcp-migration`

## Guiding Principles

1. **Preserve Functionality** - Every step must maintain working code
2. **Replace, Don't Duplicate** - No parallel implementations or legacy code
3. **Test Against Reality** - Rewrite tests to match actual API, not fantasies
4. **Git Safety Net** - Branch isolation allows fearless refactoring
5. **Standards from Start** - CLAUDE.md compliance in every step, not as afterthought

## CLAUDE.md Compliance Checklist

Every step must satisfy:

- [ ] **Module Organization**: lib.rs only has `mod` + `pub use`
- [ ] **Instrumentation**: All public functions have `#[instrument]`
- [ ] **Standard Derives**: `Debug, Clone, PartialEq, Eq, Hash` (where applicable)
- [ ] **Documentation**: All public items documented
- [ ] **Builders**: Use builders, never struct literals
- [ ] **Errors**: Use `derive_more::Display` + `derive_more::Error`
- [ ] **Tests**: In `tests/` directory, not inline
- [ ] **Imports**: `use crate::{Type}` not module paths
- [ ] **Validation**: `just check` + `just test-package` pass
- [ ] **Commits**: Fix all warnings, audit-friendly messages

## Current State Assessment

**What we have:**
- 18 tools implementing `McpTool` trait
- pmcp-based server with adapter layer (~476 lines)
- Working integration tests
- ~36 tool files total (including elicitation subdirectory)

**What we're removing:**
- `McpTool` trait (~50 lines in tools/mod.rs)
- `McpToolAdapter` (~45 lines in pmcp_adapters.rs)
- All `impl McpTool` blocks (16 lines avg × 18 tools = ~288 lines)
- Manual JSON extraction (~10-20 lines per tool)
- pmcp_server.rs registration boilerplate (~431 lines)

**What we're adding:**
- rmcp dependency with proper feature flags
- `#[tool_router]` impl block with full instrumentation
- Type definitions with complete standard derives
- Proper error types using derive_more
- Tests in `tests/` directory
- Complete documentation on all public items

## Step-by-Step Migration

### Step 1: Add rmcp Dependency

**Goal:** Get rmcp into the crate without breaking anything

**Actions:**

Edit `crates/botticelli_mcp/Cargo.toml`:

```toml
[dependencies]
# MCP SDK - using official rmcp
rmcp = { version = "0.12.0", features = ["server"] }
rmcp-macros = "0.12.0"
schemars = "0.8"  # For JsonSchema derives

# Keep pmcp temporarily during migration
pmcp = { version = "1.8", features = ["streamable-http", "http"] }
```

**Validation:**
```bash
just check botticelli_mcp
# Should compile without errors
```

**Commit:**
```bash
git add Cargo.toml
git commit -m "feat(mcp): Add rmcp dependencies for migration

Add official Rust MCP SDK dependencies:
- rmcp 0.12.0 with server features
- rmcp-macros 0.12.0 for tool/router macros
- schemars 0.8 for JSON schema generation

Keep pmcp temporarily for gradual migration.

Validation:
- cargo check passes
- No breaking changes to existing code"
```

---

### Step 2: Create Error Types Module

**Goal:** Define proper error types before implementing tools

**Create:** `crates/botticelli_mcp/src/errors.rs`

```rust
//! Error types for MCP tools.
//!
//! All errors use derive_more for Display and Error implementations,
//! following CLAUDE.md standards.

use derive_more::{Display, Error};

/// Errors that can occur during tool execution.
#[derive(Debug, Clone, Display, Error)]
pub enum ToolError {
    /// Dialog resource not configured.
    #[display("Dialog resource not configured for this server")]
    DialogNotConfigured,
    
    /// Database operations not configured.
    #[display("Database operations not configured for this server")]
    DatabaseNotConfigured,
    
    /// Elicitation failed.
    #[display("Elicitation failed: {}", _0)]
    ElicitationFailed(String),
    
    /// Invalid input provided.
    #[display("Invalid input: {}", _0)]
    InvalidInput(String),
    
    /// Internal error occurred.
    #[display("Internal error: {}", _0)]
    Internal(String),
}

impl From<botticelli_error::McpError> for ToolError {
    fn from(err: botticelli_error::McpError) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<botticelli_error::ElicitationError> for ToolError {
    fn from(err: botticelli_error::ElicitationError) -> Self {
        Self::ElicitationFailed(err.to_string())
    }
}
```

**Update:** `crates/botticelli_mcp/src/lib.rs`

```rust
// Add after existing modules
mod errors;

// Add to exports
pub use errors::ToolError;
```

**Validation:**
```bash
just check botticelli_mcp
# Should compile with zero warnings
```

**Commit:**
```bash
git add src/errors.rs src/lib.rs
git commit -m "feat(mcp): Add proper error types for tools

Define ToolError enum with derive_more:
- Display and Error derives (CLAUDE.md compliant)
- Comprehensive error variants for all tool failure modes
- Conversions from existing error types

Standards compliance:
- Uses derive_more::Display + derive_more::Error
- No manual impl Display or impl Error
- Clear error messages for each variant
- Crate-level export in lib.rs

Validation:
- cargo check passes
- Zero clippy warnings"
```

---

### Step 3: Create Echo Tool Types

**Goal:** Define parameter/result types for echo tool with full compliance

**Create:** `crates/botticelli_mcp/src/echo.rs`

```rust
//! Echo tool types.
//!
//! The echo tool provides a simple test endpoint that returns
//! the input message with a timestamp, useful for testing MCP connectivity.

use chrono::Utc;
use derive_getters::Getters;
use derive_new::new;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for the echo tool.
///
/// This tool echoes back the provided message with a timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Getters, new)]
pub struct EchoParams {
    /// The message to echo back.
    ///
    /// This can be any UTF-8 string. The server will return it
    /// unchanged along with a timestamp.
    message: String,
}

/// Result from the echo tool.
///
/// Contains the echoed message and the timestamp when it was processed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Getters)]
pub struct EchoResult {
    /// The echoed message (same as input).
    echo: String,
    
    /// ISO 8601 timestamp when the echo was processed.
    timestamp: String,
}

impl EchoResult {
    /// Create a new echo result with current timestamp.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to include in the result
    ///
    /// # Returns
    ///
    /// An `EchoResult` with the message and current timestamp.
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
# Should compile with zero warnings
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
- derive-getters for field access (private fields)
- derive-new for EchoParams constructor
- Complete documentation on types, fields, and methods
- Crate-level exports in lib.rs (no module paths in imports)
- One file per domain pattern
- Helper method for EchoResult construction

Validation:
- cargo check passes
- Zero clippy warnings
- All public items documented"
```

---

### Step 4: Create Server Infrastructure

**Goal:** Define BotticelliServer with builder pattern and proper structure

**Create:** `crates/botticelli_mcp/src/server.rs`

```rust
//! RMCP-based MCP server implementation.
//!
//! The BotticelliServer struct holds all tool implementations and state
//! needed for MCP operations.

use crate::dialog_resource::DialogResource;
use rmcp::handler::server::tool::ToolRouter;
use std::sync::Arc;

#[cfg(feature = "database")]
use botticelli_interface::DatabaseRegistryOperations;

/// Botticelli MCP server using rmcp.
///
/// This server exposes Botticelli's capabilities as MCP tools.
/// Use the builder pattern to construct instances with optional components.
///
/// # Examples
///
/// ```no_run
/// use botticelli_mcp::BotticelliServer;
///
/// let server = BotticelliServer::builder()
///     .build();
/// ```
#[derive(Clone)]
pub struct BotticelliServer {
    tool_router: ToolRouter<Self>,
    
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
    
    dialog: Option<Arc<DialogResource>>,
}

impl BotticelliServer {
    /// Create a builder for configuring the server.
    ///
    /// # Returns
    ///
    /// A new `BotticelliServerBuilder` with default configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use botticelli_mcp::BotticelliServer;
    ///
    /// let server = BotticelliServer::builder()
    ///     .build();
    /// ```
    pub fn builder() -> BotticelliServerBuilder {
        BotticelliServerBuilder::default()
    }
}

/// Builder for BotticelliServer.
///
/// Provides a type-safe way to configure optional server components
/// before construction.
#[derive(Debug, Clone, Default)]
pub struct BotticelliServerBuilder {
    #[cfg(feature = "database")]
    db_ops: Option<Arc<dyn DatabaseRegistryOperations>>,
    
    dialog: Option<Arc<DialogResource>>,
}

impl BotticelliServerBuilder {
    /// Configure database operations.
    ///
    /// # Arguments
    ///
    /// * `db` - Database operations implementation
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    #[cfg(feature = "database")]
    pub fn database(mut self, db: Arc<dyn DatabaseRegistryOperations>) -> Self {
        self.db_ops = Some(db);
        self
    }
    
    /// Configure dialog resource for elicitation tools.
    ///
    /// # Arguments
    ///
    /// * `dialog` - Dialog resource for user interaction
    ///
    /// # Returns
    ///
    /// The builder for method chaining.
    pub fn dialog(mut self, dialog: Arc<DialogResource>) -> Self {
        self.dialog = Some(dialog);
        self
    }
    
    /// Build the BotticelliServer instance.
    ///
    /// # Returns
    ///
    /// A configured `BotticelliServer` ready to serve MCP requests.
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

**Update:** `crates/botticelli_mcp/src/lib.rs`

```rust
mod server;

pub use server::{BotticelliServer, BotticelliServerBuilder};
```

**Validation:**
```bash
just check botticelli_mcp
# Won't compile yet - missing tool_router() method
# That's expected, we'll add it in next step
```

**Commit:**
```bash
git add src/server.rs src/lib.rs
git commit -m "feat(mcp): Add server infrastructure with builder

Add BotticelliServer and BotticelliServerBuilder:
- Manual builder pattern (CLAUDE.md compliant)
- Optional components (database, dialog)
- Complete documentation with examples
- Feature-gated database support

Standards compliance:
- Builder pattern used (not struct literals)
- All public items documented
- Crate-level exports
- Feature flags properly applied

Note: Does not compile yet - tool_router() method added in next step

Validation:
- Type definitions are correct
- Builder pattern compiles
- Documentation builds"
```

---

### Step 5: Implement Echo Tool with Full Instrumentation

**Goal:** Add echo tool to server with proper instrumentation and error handling

**Update:** `crates/botticelli_mcp/src/server.rs`

Add imports at top:
```rust
use crate::{EchoParams, EchoResult, ToolError};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use tracing::{debug, instrument};
```

Add after BotticelliServerBuilder:
```rust
#[tool_router]
impl BotticelliServer {
    /// Echo back a message with timestamp.
    ///
    /// This tool is useful for testing MCP connectivity and verifying
    /// that the server is responding correctly.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the message to echo
    ///
    /// # Returns
    ///
    /// Returns the echoed message with a timestamp on success.
    ///
    /// # Errors
    ///
    /// This tool should not fail under normal circumstances.
    #[tool(description = "Echoes back the input message with a timestamp")]
    #[instrument(skip(self), fields(message = %params.0.message))]
    async fn echo(
        &self,
        Parameters(params): Parameters<EchoParams>
    ) -> Result<Json<EchoResult>, ToolError> {
        debug!("Processing echo request");
        
        let result = EchoResult::new(params.message);
        
        debug!(result = ?result, "Echo completed successfully");
        Ok(Json(result))
    }
}

#[tool_handler]
impl ServerHandler for BotticelliServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "botticelli".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            instructions: Some("Botticelli MCP server - LLM orchestration tools".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}
```

**Validation:**
```bash
just check botticelli_mcp
# Should compile now with zero warnings
```

**Commit:**
```bash
git add src/server.rs
git commit -m "feat(mcp): Implement echo tool with full instrumentation

Add echo tool to BotticelliServer:
- Tool router macro generates handler code
- Full instrumentation with #[instrument]
- Proper error type (ToolError, not String)
- Complete documentation
- ServerHandler implementation

Standards compliance:
- All public functions have #[instrument]
- Proper error handling with ToolError
- Debug logging at key points
- Complete documentation
- Uses helper constructor (EchoResult::new)

Validation:
- cargo check passes
- Zero clippy warnings
- Tool macro expands correctly"
```

---

### Step 6: Create Test Infrastructure

**Goal:** Add tests in proper location with real API usage

**Create:** `crates/botticelli_mcp/tests/echo_tool_test.rs`

```rust
//! Tests for echo tool.
//!
//! Tests use the actual rmcp API, not mocked interfaces.

use botticelli_mcp::{BotticelliServer, EchoParams, EchoResult};

#[tokio::test]
async fn test_echo_basic_message() {
    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: "Hello, MCP!".to_string(),
    };
    
    let result = server.echo(rmcp::handler::server::wrapper::Parameters(params))
        .await
        .expect("Echo should succeed");
    
    assert_eq!(result.0.echo(), "Hello, MCP!");
    assert!(!result.0.timestamp().is_empty());
}

#[tokio::test]
async fn test_echo_empty_message() {
    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: String::new(),
    };
    
    let result = server.echo(rmcp::handler::server::wrapper::Parameters(params))
        .await
        .expect("Echo should succeed even with empty message");
    
    assert_eq!(result.0.echo(), "");
}

#[tokio::test]
async fn test_echo_unicode_message() {
    let server = BotticelliServer::builder().build();
    let params = EchoParams {
        message: "Hello 世界 🌍".to_string(),
    };
    
    let result = server.echo(rmcp::handler::server::wrapper::Parameters(params))
        .await
        .expect("Echo should handle unicode");
    
    assert_eq!(result.0.echo(), "Hello 世界 🌍");
}
```

**Validation:**
```bash
just test-package botticelli_mcp
# All tests should pass
```

**Commit:**
```bash
git add tests/echo_tool_test.rs
git commit -m "test(mcp): Add tests for echo tool

Add comprehensive tests for echo tool in tests/ directory:
- Basic message echo
- Empty message handling
- Unicode character support

Standards compliance:
- Tests in tests/ directory, not inline
- Import from crate root (use crate::{Type})
- Test against real API, not mocks
- Descriptive test names

Validation:
- All 3 tests pass
- cargo test succeeds"
```

---

### Step 7: Add Server Info Tool

**Goal:** Second tool to validate pattern with state access

**Create:** `crates/botticelli_mcp/src/server_info.rs`

```rust
//! Server info tool types.
//!
//! Provides server metadata and version information.

use derive_getters::Getters;
use derive_new::new;
use schemars::JsonSchema;
use serde::Serialize;

/// Result from the server_info tool.
///
/// Contains server metadata including name, version, and timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Getters, new)]
pub struct ServerInfoResult {
    /// Server name.
    name: String,
    
    /// Server version.
    version: String,
    
    /// ISO 8601 timestamp when info was retrieved.
    timestamp: String,
    
    /// Number of tools available.
    tool_count: usize,
}
```

**Update:** `crates/botticelli_mcp/src/lib.rs`

```rust
mod server_info;

pub use server_info::ServerInfoResult;
```

**Update:** `crates/botticelli_mcp/src/server.rs`

Add to imports:
```rust
use crate::ServerInfoResult;
use chrono::Utc;
```

Add to `#[tool_router] impl BotticelliServer`:
```rust
    /// Get server information and metadata.
    ///
    /// Returns server name, version, and available tool count.
    ///
    /// # Returns
    ///
    /// Server metadata including version and tool count.
    ///
    /// # Errors
    ///
    /// This tool should not fail under normal circumstances.
    #[tool(description = "Returns server metadata and version information")]
    #[instrument(skip(self))]
    async fn server_info(&self) -> Result<Json<ServerInfoResult>, ToolError> {
        debug!("Retrieving server information");
        
        let result = Json(ServerInfoResult::new(
            "botticelli".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            Utc::now().to_rfc3339(),
            self.tool_router.list_all().len(),
        ));
        
        debug!(tool_count = result.0.tool_count(), "Server info retrieved");
        Ok(result)
    }
```

**Create:** `crates/botticelli_mcp/tests/server_info_tool_test.rs`

```rust
//! Tests for server_info tool.

use botticelli_mcp::BotticelliServer;

#[tokio::test]
async fn test_server_info_basic() {
    let server = BotticelliServer::builder().build();
    
    let result = server.server_info()
        .await
        .expect("Server info should succeed");
    
    assert_eq!(result.0.name(), "botticelli");
    assert!(!result.0.version().is_empty());
    assert!(!result.0.timestamp().is_empty());
    assert!(*result.0.tool_count() >= 2); // At least echo and server_info
}
```

**Validation:**
```bash
just check botticelli_mcp
just test-package botticelli_mcp
# All tests should pass
```

**Commit:**
```bash
git add src/server_info.rs src/server.rs src/lib.rs tests/server_info_tool_test.rs
git commit -m "feat(mcp): Add server_info tool

Add server_info tool with metadata:
- ServerInfoResult type with full derives
- Tool implementation with instrumentation
- Access to tool_router state
- Comprehensive test coverage

Standards compliance:
- Complete documentation
- Standard derives (Debug, Clone, PartialEq, Eq)
- derive-getters for private fields
- derive-new for constructor
- Instrumentation with debug logging
- Tests in tests/ directory
- Crate-level exports
- Getters used in tool implementation (result.0.tool_count())

Validation:
- cargo check passes
- All tests pass (4 total)
- Zero clippy warnings"
```

---

## Migration Tracking

| Tool | Status | Notes |
|------|--------|-------|
| echo | ✅ Step 4 | Pilot implementation |
| server_info | ✅ Step 5 | Second pilot |
| generate | ⏳ Step 8 | Batch convert |
| query_content | ⏳ Step 9 | Needs db_ops state |
| elicit_text | ⏳ Step 10 | Needs dialog state |
| elicit_select | ⏳ Step 10 | Needs dialog state |
| elicit_number | ⏳ Step 10 | Needs dialog state |
| elicit_bool | ⏳ Step 10 | Needs dialog state |
| create_narrative_session | ⏳ Step 10 | Needs registry state |
| ... | ⏳ | 10+ more tools |

## Success Metrics

**Code Quality:**
- [ ] Zero `impl McpTool` blocks
- [ ] Zero `McpToolAdapter` usage
- [ ] All types have `#[derive(Serialize, Deserialize, JsonSchema)]`
- [ ] All tool methods use `#[tool]` macro
- [ ] No pmcp imports remain

**Functionality:**
- [ ] All original tools work
- [ ] Integration tests pass
- [ ] Claude Desktop integration works
- [ ] HTTP transport works (if applicable)

**Code Size:**
- [ ] ~600 lines removed (manual JSON + adapter layer)
- [ ] ~200 lines added (type definitions)
- [ ] Net reduction: ~400 lines

## Rollback Plan

If migration fails at any step:

1. Git is our safety net: `git checkout dev`
2. Branch contains all experimental changes
3. Can cherry-pick successful pieces
4. Can restart from any step

**No parallel code means clean rollback.**

## Next Actions

**Immediate:**
1. Execute Steps 1-2 (dependencies + types)
2. Execute Steps 3-4 (server struct + echo tool)
3. Validate approach before proceeding

**Then:**
4. Steps 5-7 (second tool + binary)
5. Steps 8-10 (batch conversion)
6. Steps 11-15 (cleanup + validation)

**Timeline:** 2-3 days for full migration

---

*Document created: 2024-12-29*  
*Status: Implementation Ready*  
*Branch: feat/rmcp-migration*  
*Approach: Replace as we go, test with reality, git safety net*

## Implementation Progress

### ✅ Completed (Steps 1-8)

**Infrastructure:**
- Step 1: Dependencies added (rmcp 0.12.0, schemars 1.0)
- Step 2: Error conversion (`From<ToolError> for rmcp::ErrorData`)
- Step 3: Echo tool types with public fields
- Step 4: Server infrastructure with builder pattern
- Step 5: Echo tool implementation with instrumentation
- Step 6: Echo tests (3/3 passing)
- Step 7: server_info tool + tests (3/3 passing)
- Step 8: pmcp removal (881 lines deleted)

**Current Status:**
- ✅ 6/6 tests passing
- ✅ Zero pmcp dependencies
- ✅ Clean compilation
- ✅ Working rmcp-based MCP server

**Lessons Learned:**
1. **Error types matter** - Must use `rmcp::ErrorData` or implement `From`
2. **Public fields required** - rmcp macros expect public fields
3. **Async all the way** - Tool methods must be `async fn`
4. **Type safety wins** - Compiler catches entire class of errors
5. **cargo expand essential** - Critical for debugging macro issues

### 🔄 Next Phase: Tool Migration

See [RMCP_TOOL_MIGRATION_PLAN.md](./RMCP_TOOL_MIGRATION_PLAN.md) for:
- Detailed 6-phase migration strategy
- 30-step rollout plan
- Per-tool migration checklist
- Dependency handling patterns
- Testing guidelines

**Immediate Next Steps:**
- Step 9: Migrate database/query_content tool
- Step 10: Migrate export_metrics tool
- Step 11: Migrate elicitation primitives
- Continue with phases 1-6...

The foundation is solid. Now we scale the pattern to all 36 tools.
