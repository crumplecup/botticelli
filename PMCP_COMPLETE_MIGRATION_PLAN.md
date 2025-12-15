# Complete pmcp Migration Plan

**Status**: Phase 1 Complete - Dead Code Removed ✅  
**Branch**: `pmcp-cleanup`  
**Goal**: Replace all custom MCP implementations with pmcp library

## Progress Summary

### ✅ Phase 0: Discovery (Complete)
- Audited existing codebase
- Found `ExternalMcpClient` already using pmcp properly
- Identified `transport.rs` as dead code with TODOs

### ✅ Phase 1: Dead Code Removal (Complete)
- Removed unused `transport.rs` module (196 lines)
- Updated `lib.rs` exports
- All tests passing ✅
- Workspace compiles cleanly ✅

## Decision: Full pmcp Adoption

**Rationale**:
- pmcp is battle-tested with TypeScript SDK compatibility
- Currently at v1.8.6 on crates.io
- Eliminates maintenance burden of custom protocol implementation
- Provides transport layer, protocol handling, and type system
- Better long-term support from ecosystem

**What We're Replacing**:
- Custom `botticelli_mcp` types with `pmcp::protocol`
- Custom transport implementations with `pmcp::transport`
- Custom protocol handling with `pmcp::client`

**What We're Keeping**:
- Our orchestration layer (`McpClient`, `UnifiedMcpClient`)
- Our business logic (approval, retry, metrics, LLM adapter)
- Our tool execution framework

---

## Phase 1: Add pmcp Dependency & Analyze

**Goal**: Add pmcp and understand what it provides

### Step 1.1: Add Dependency
```toml
# crates/botticelli_mcp_client/Cargo.toml
[dependencies]
pmcp = "1.8"
```

**Success Criteria**:
- `cargo check -p botticelli_mcp_client` passes
- No version conflicts

### Step 1.2: Document pmcp API Surface
Create analysis document covering:
- Protocol types (`pmcp::protocol::*`)
- Transport traits and implementations
- Client APIs
- Error types
- Serialization format

**Success Criteria**:
- Document created: `PMCP_API_ANALYSIS.md`
- Mapping table: botticelli type → pmcp equivalent

---

## Phase 2: Deprecate botticelli_mcp Crate

**Goal**: Stop using custom protocol implementation

### Step 2.1: Replace Type Imports

**Current**:
```rust
use botticelli_mcp::{Request, Response, Tool, Resource};
```

**New**:
```rust
use pmcp::protocol::{JsonRpcRequest, JsonRpcResponse, Tool, Resource};
```

**Files to Update**:
- `crates/botticelli_mcp_client/src/client.rs`
- `crates/botticelli_mcp_client/src/external_client.rs`
- `crates/botticelli_mcp_client/src/unified_client.rs`
- `crates/botticelli_mcp_client/src/transport.rs`
- `crates/botticelli_mcp_client/src/tool_executor.rs`
- `crates/botticelli_mcp_client/src/schema.rs`

**Success Criteria**:
- All `use botticelli_mcp` removed from mcp_client crate
- `cargo check -p botticelli_mcp_client` passes
- Tests compile (may not pass yet)

### Step 2.2: Remove botticelli_mcp Dependency
```toml
# Remove from Cargo.toml
# botticelli_mcp = { path = "../botticelli_mcp", version = "0.2.0" }
```

**Success Criteria**:
- Dependency removed
- No compilation errors (business logic still works)

---

## Phase 3: Replace Transport Layer

**Goal**: Use pmcp's transport implementations

### Step 3.1: Replace StdioTransport

**Current**: Custom `StdioTransport` in `transport.rs`

**New**: Use `pmcp::transport::StdioTransport`

**Migration**:
```rust
// Old
pub struct StdioTransport { /* custom */ }

// New - adapter pattern
pub struct StdioTransport(pmcp::transport::StdioTransport);

impl Transport for StdioTransport {
    async fn send(&mut self, request: JsonRpcRequest) -> McpClientResult<JsonRpcResponse> {
        self.0.send(request)
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::Transport(e.to_string())))
    }
}
```

**Success Criteria**:
- `StdioTransport` uses pmcp internally
- Our `Transport` trait still works
- Tests pass

### Step 3.2: Replace HttpTransport

Similar pattern for HTTP transport.

**Success Criteria**:
- `HttpTransport` uses pmcp internally
- SSE support maintained
- Tests pass

---

## Phase 4: Replace Client Implementation

**Goal**: Use pmcp's client for external servers

### Step 4.1: Update ExternalMcpClient

**Current**: Custom protocol handling

**New**: Delegate to `pmcp::Client`

```rust
use pmcp::Client as PmcpClient;

pub struct ExternalMcpClient {
    client: PmcpClient,
    config: ExternalServerConfig,
    metrics: Arc<McpClientMetrics>,
    retry: RetryConfig,
}

impl ExternalMcpClient {
    pub async fn new(config: ExternalServerConfig) -> McpClientResult<Self> {
        let transport = match config.transport {
            TransportType::Stdio => {
                let stdio = pmcp::transport::StdioTransport::new(/* ... */);
                Box::new(stdio) as Box<dyn pmcp::transport::Transport>
            }
            TransportType::Http => {
                let http = pmcp::transport::HttpTransport::new(/* ... */);
                Box::new(http) as Box<dyn pmcp::transport::Transport>
            }
        };

        let client = PmcpClient::new(transport);
        
        Ok(Self {
            client,
            config,
            metrics: Arc::new(McpClientMetrics::default()),
            retry: RetryConfig::default(),
        })
    }

    pub async fn call_tool(&mut self, name: &str, args: serde_json::Value) 
        -> McpClientResult<serde_json::Value> 
    {
        // Use pmcp client
        let result = self.client.call_tool(name, args).await?;
        self.metrics.record_tool_call(name, true);
        Ok(result)
    }
}
```

**Success Criteria**:
- External server calls work via pmcp
- Metrics still recorded
- Retry logic still applied
- Tests pass

---

## Phase 5: Update Schema & Tool Definitions

**Goal**: Use pmcp's schema types

### Step 5.1: Replace Tool Definitions

**Current**: Custom `ToolDefinition` struct

**New**: Use `pmcp::protocol::Tool` directly or wrap it

```rust
pub type ToolDefinition = pmcp::protocol::Tool;

// Or if we need additional fields:
pub struct ToolDefinition {
    pub tool: pmcp::protocol::Tool,
    pub approval_required: bool,
    pub rate_limit: Option<RateLimit>,
}
```

**Success Criteria**:
- Schema compatible with MCP spec
- JSON serialization works
- Tool registration unchanged

---

## Phase 6: Integration Testing

**Goal**: Verify everything works end-to-end

### Step 6.1: External Server Test

Test connecting to real MCP server (filesystem server):

```rust
#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_external_filesystem_server() {
    let config = ExternalServerConfig::builder()
        .name("filesystem")
        .command("npx")
        .args(vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
        .build()
        .unwrap();

    let mut client = ExternalMcpClient::new(config).await.unwrap();
    
    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());
    
    // Try calling a tool
    let result = client.call_tool("read_file", json!({
        "path": "/tmp/test.txt"
    })).await;
    
    assert!(result.is_ok() || matches!(result, Err(McpClientError { kind: McpClientErrorKind::NotFound(_), .. })));
}
```

**Success Criteria**:
- Can connect to external server
- Can list tools
- Can call tools
- Errors handled gracefully

### Step 6.2: Update All Tests

Update tests to use pmcp types:

```bash
just test-package botticelli_mcp_client
```

**Success Criteria**:
- All tests pass
- No warnings
- Coverage maintained

---

## Phase 7: Update Documentation

**Goal**: Document the new architecture

### Step 7.1: Update Module Docs

Update `src/lib.rs` to reflect pmcp usage:

```rust
//! MCP client for agentic orchestration and external server connections.
//!
//! Built on the [`pmcp`](https://docs.rs/pmcp) protocol library.
//!
//! # Architecture
//!
//! - **Protocol Layer**: pmcp handles JSON-RPC, types, serialization
//! - **Transport Layer**: pmcp provides stdio/HTTP, we add adapters
//! - **Client Layer**: pmcp Client for external servers
//! - **Orchestration Layer**: Our UnifiedMcpClient coordinates everything
//!
//! # External Server Support
//!
//! Connect to any MCP-compliant server:
//!
//! ```rust,no_run
//! use botticelli_mcp_client::{ExternalMcpClient, ExternalServerConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ExternalServerConfig::builder()
//!     .name("filesystem")
//!     .command("npx")
//!     .args(vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
//!     .build()?;
//!
//! let mut client = ExternalMcpClient::new(config).await?;
//! let tools = client.list_tools().await?;
//! # Ok(())
//! # }
//! ```
```

**Success Criteria**:
- Doctests pass
- Examples accurate
- Architecture clear

### Step 7.2: Create Migration Guide

Document for users:
- What changed
- How to update code
- Breaking changes
- New capabilities

**Success Criteria**:
- `PMCP_MIGRATION_GUIDE.md` created
- Covers all breaking changes
- Includes migration examples

---

## Phase 8: Cleanup & Verification

**Goal**: Remove dead code and verify clean state

### Step 8.1: Remove Old Code

Mark `botticelli_mcp` crate as deprecated (don't delete yet - other crates may use):

```toml
# crates/botticelli_mcp/Cargo.toml
[package]
name = "botticelli_mcp"
version = "0.2.0"
description = "DEPRECATED: Use pmcp crate instead"
```

**Success Criteria**:
- `botticelli_mcp_client` no longer depends on `botticelli_mcp`
- Deprecation warnings in place

### Step 8.2: Final Checks

```bash
just check-package botticelli_mcp_client
just test-package botticelli_mcp_client
just check-features
```

**Success Criteria**:
- Zero warnings
- All tests pass
- No clippy issues
- Features work

---

## Phase 9: Commit & Document

**Goal**: Clean commits with clear history

### Step 9.1: Commit Strategy

Each phase = separate commit:
1. `feat(mcp): Add pmcp dependency and analysis`
2. `refactor(mcp): Replace type imports with pmcp`
3. `refactor(mcp): Replace transport layer with pmcp`
4. `refactor(mcp): Replace client implementation with pmcp`
5. `refactor(mcp): Update schema to use pmcp types`
6. `test(mcp): Add external server integration tests`
7. `docs(mcp): Update documentation for pmcp migration`
8. `chore(mcp): Deprecate botticelli_mcp crate`

**Success Criteria**:
- Clean git history
- Each commit compiles
- Commits follow conventional format

### Step 9.2: Update Planning Docs

Update tracking documents:
- Mark phases complete in this document
- Update `PLANNING_INDEX.md`
- Create summary in `PMCP_MIGRATION_COMPLETE.md`

**Success Criteria**:
- All tracking updated
- Summary document created
- Ready for merge review

---

## Success Metrics

**Completion Criteria**:
- ✅ `botticelli_mcp_client` uses pmcp exclusively
- ✅ `botticelli_mcp` dependency removed from mcp_client
- ✅ All tests pass
- ✅ External server connectivity works
- ✅ Documentation updated
- ✅ Zero warnings/errors
- ✅ Migration guide created

**Performance**:
- No regression in latency
- Memory usage similar or better
- Error handling improved

**Code Quality**:
- Less code to maintain (removed custom protocol)
- Better type safety (pmcp types)
- Clearer separation of concerns

---

## Rollback Plan

If migration fails:
1. Revert commits (clean history makes this easy)
2. Keep pmcp dependency for future attempt
3. Document blockers in issues
4. Continue with custom implementation

**Blockers to Watch For**:
- pmcp API incompatibilities
- Missing features we need
- Performance issues
- Serialization problems

---

## Next Steps

Ready to proceed? Let's start with Phase 1:

```bash
# Create branch
git checkout -b feat/pmcp-complete-migration

# Add dependency
# (you edit Cargo.toml)

# Verify
cargo check -p botticelli_mcp_client
```

Then I'll analyze pmcp's API and create the mapping document.
