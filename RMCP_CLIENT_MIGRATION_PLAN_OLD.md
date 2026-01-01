# Complete pmcp → rmcp Migration Plan

## Executive Summary

**Goal**: Remove pmcp dependency entirely, migrate all 3 crates to use rmcp's client features.

**Current State**:
- ✅ botticelli_mcp: Uses `rmcp` with `features = ["server"]`
- ❌ botticelli_mcp_client: Uses `pmcp` for external MCP server connections (8 files)
- ❌ botticelli_tui: Uses `pmcp::Client` (1 file)
- ❌ botticelli_chat: Uses `pmcp::Client` (5 files)

**Target State**:
- All crates use `rmcp` exclusively
- Single unified MCP SDK across the codebase
- Clean separation: rmcp server features for botticelli_mcp, rmcp client features for consumers

## Type Mapping: pmcp → rmcp

Based on analysis of rmcp docs and botticelli_mcp's usage:

| pmcp Type | rmcp Equivalent | Notes |
|-----------|-----------------|-------|
| `pmcp::Client<T>` | `rmcp::Service` (trait) | rmcp uses service pattern |
| `pmcp::Transport` | Transport abstraction | Built into rmcp service |
| `pmcp::types::TransportMessage` | Internal to rmcp | Not exposed in public API |
| `pmcp::Content` | `rmcp::model::Content` | Similar structure: Text, Image, Resource |
| `pmcp::ToolInfo` | `rmcp::model::Tool` | Tool metadata structure |
| `pmcp::ClientCapabilities` | `rmcp::model::ClientInfo` | Client initialization |
| `pmcp::StdioTransport` | `rmcp::TokioChildProcess` | Child process communication |
| `pmcp::Error` | `rmcp::Error` | Error type |
| `pmcp::Result<T>` | `Result<T, rmcp::Error>` | Result type |

## Phase 1: botticelli_mcp_client (Foundation)

**Complexity**: HIGH
**Estimated Effort**: 16-24 hours
**Risk**: MEDIUM-HIGH (affects all downstream crates)

### Files to Migrate

#### 1.1: external_client.rs (Core Client)

**Current Pattern**:
```rust
use pmcp::types::TransportMessage;
use pmcp::{Client, ClientCapabilities, Transport};

struct ChildProcessTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
}

#[async_trait::async_trait]
impl Transport for ChildProcessTransport {
    async fn send(&mut self, message: TransportMessage) -> pmcp::Result<()> { ... }
    async fn receive(&mut self) -> pmcp::Result<TransportMessage> { ... }
    async fn close(&mut self) -> pmcp::Result<()> { ... }
}

pub struct ExternalMcpClient {
    client: Client<ChildProcessTransport>,
    // ...
}
```

**Target Pattern**:
```rust
use rmcp::Service;
use rmcp::transport::TokioChildProcess;

pub struct ExternalMcpClient {
    service: Box<dyn Service>,
    // ...
}

impl ExternalMcpClient {
    pub async fn connect(config: ExternalServerConfig) -> McpClientResult<Self> {
        // Spawn child process
        let mut child = Command::new(&config.command)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        // Use rmcp's built-in child process transport
        let transport = TokioChildProcess::new(child)?;

        // Create service (rmcp's equivalent of Client)
        let service = ().serve(transport).await?;

        // List tools
        let tools_result = service.list_tools(Default::default()).await?;

        Ok(Self {
            service: Box::new(service),
            tools: tools_result.tools,
            // ...
        })
    }

    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> McpClientResult<Value> {
        use rmcp::model::CallToolRequestParam;

        let result = self.service.call_tool(CallToolRequestParam {
            name: tool_name.into(),
            arguments: arguments.as_object().cloned(),
        }).await?;

        // Extract result from Content
        content_to_json(&result.content)
    }
}
```

**Migration Steps**:
1. Update Cargo.toml: Add rmcp client features
   ```toml
   rmcp = { version = "0.12.0", features = ["client", "transport-stdio"] }
   ```
2. Remove custom ChildProcessTransport (use rmcp::TokioChildProcess)
3. Replace `Client<T>` with `Box<dyn Service>`
4. Update initialization to use `.serve()` pattern
5. Map `client.call_tool()` to service pattern
6. Update error handling to use rmcp::Error

**Dependencies**:
- Need to understand rmcp::Service trait API
- May need to check if TokioChildProcess handles our use case

#### 1.2: client.rs (Main Client Facade)

**Current Usage**:
```rust
use pmcp::Content;

// Convert pmcp::Content for responses
fn content_to_json(content: &[Content]) -> Value
```

**Target**:
```rust
use rmcp::model::Content;

// Same function signature, different import
fn content_to_json(content: &[Content]) -> Value
```

**Migration**:
- Update imports
- Verify Content enum structure matches

#### 1.3: orchestrator.rs (Tool Orchestration)

**Current Usage**:
```rust
use pmcp::{Content, ToolInfo};

fn tool_info_to_schema(info: &ToolInfo) -> ToolSchema { ... }
fn content_to_json(content: &[Content]) -> Value { ... }
```

**Target**:
```rust
use rmcp::model::{Content, Tool};

fn tool_info_to_schema(info: &Tool) -> ToolSchema { ... }
fn content_to_json(content: &[Content]) -> Value { ... }
```

**Migration**:
- Replace `ToolInfo` with `rmcp::model::Tool`
- Update field access (may have different names)
- Verify Content structure

#### 1.4: tool_registry.rs

**Current Usage**:
```rust
use pmcp::{Content, ToolInfo};
```

**Migration**:
- Same as orchestrator.rs
- Update type references

#### 1.5-1.8: tools/*.rs (Tool Implementations)

**Files**:
- tools/narrative.rs
- tools/registry_ops.rs
- tools/elicitation.rs
- tools/database.rs

**Pattern**:
```rust
use pmcp::{Content, ToolInfo};

// Helper functions using these types
```

**Migration**:
- Bulk find/replace imports
- Verify no pmcp-specific API calls

### Cargo.toml Changes

```toml
[dependencies]
# Remove this line:
# pmcp = { version = "1.8", features = ["validation"] }

# Add rmcp with client features:
rmcp = { version = "0.12.0", features = ["client"] }
```

### Testing Strategy

1. Unit tests for ExternalMcpClient
2. Integration test with a simple MCP server (e.g., echo server)
3. Test external server connection (filesystem, git)
4. Verify tool calling works end-to-end

### Risk Mitigation

**Risk**: rmcp's client API may be different from expectations
**Mitigation**:
- Create proof-of-concept first
- Test with simple external server before full migration

**Risk**: TokioChildProcess may not support our use case
**Mitigation**:
- Review rmcp source code
- Fall back to custom transport if needed (implement rmcp's transport trait)

## Phase 2: botticelli_tui (Low Complexity)

**Complexity**: LOW
**Estimated Effort**: 2-4 hours
**Risk**: LOW

### File: minimal_loop.rs

**Current**:
```rust
use pmcp::{Client, Transport};

async fn show_warning(
    _client: &pmcp::Client<botticelli_mcp::InProcTransport>,
    _message: &str,
) -> BotticelliResult<()> {
    Ok(())
}
```

**Analysis**:
- Function parameter is unused (`_client`)
- No actual pmcp operations performed
- Likely legacy parameter

**Target Option 1 (Minimal Change)**:
```rust
// Remove pmcp import entirely
async fn show_warning(_message: &str) -> BotticelliResult<()> {
    Ok(())
}
```

**Target Option 2 (Use rmcp if needed)**:
```rust
use rmcp::Service;

async fn show_warning(
    _service: &dyn rmcp::Service,
    _message: &str,
) -> BotticelliResult<()> {
    Ok(())
}
```

**Migration**:
1. Check if `_client` parameter is actually used anywhere
2. If unused: Remove parameter entirely
3. If used: Update to rmcp::Service
4. Update Cargo.toml to remove pmcp

### Cargo.toml Changes

```toml
[dependencies]
# Remove this line:
# pmcp = "1.8.6"

# Add rmcp if needed (check if actually used):
# rmcp = "0.12.0"
```

## Phase 3: botticelli_chat (Medium Complexity)

**Complexity**: MEDIUM
**Estimated Effort**: 8-12 hours
**Risk**: MEDIUM

### Files Overview

All in `src/elicitation/`:
- infrastructure.rs - pmcp::Client setup
- acts.rs - Tool calling
- carousel.rs - Tool calling
- inputs.rs - Tool calling
- validation.rs - Tool calling

### 3.1: infrastructure.rs (Client Setup)

**Current**:
```rust
use pmcp::types::TransportMessage;
use pmcp::{Client, ClientCapabilities, Transport};

// Custom transport implementation
struct CustomTransport { ... }

impl Transport for CustomTransport { ... }

// Create client
let transport = CustomTransport::new(...);
let client = Client::new(transport);
client.initialize(ClientCapabilities::minimal()).await?;
```

**Target**:
```rust
use rmcp::Service;
use rmcp::transport::...;

// Use rmcp's built-in transport or implement rmcp transport trait
let service = ().serve(transport).await?;
```

**Migration**:
1. Identify transport type being used (in-process? HTTP? stdio?)
2. Map to appropriate rmcp transport
3. Replace Client::new() with .serve() pattern
4. Update initialization flow

### 3.2-3.5: Tool Files (acts, carousel, inputs, validation)

**Current Pattern**:
```rust
use pmcp::{Content, ToolInfo};

pub async fn elicit_act(
    client: &pmcp::Client<impl pmcp::Transport>,
    // ...
) -> Result<...> {
    let result = client.call_tool("elicit_act", args).await?;
    // Process Content result
}
```

**Target Pattern**:
```rust
use rmcp::model::{Content, Tool, CallToolRequestParam};
use rmcp::Service;

pub async fn elicit_act(
    service: &dyn Service,
    // ...
) -> Result<...> {
    let result = service.call_tool(CallToolRequestParam {
        name: "elicit_act".into(),
        arguments: args.as_object().cloned(),
    }).await?;
    // Process Content result
}
```

**Migration**:
1. Replace `pmcp::Client<impl Transport>` with `&dyn Service`
2. Update tool calling syntax
3. Update Content processing
4. Verify error handling

### Cargo.toml Changes

```toml
[dependencies]
# Remove this line:
# pmcp = { version = "1.8", features = ["streamable-http"] }

# Add rmcp:
rmcp = "0.12.0"
```

## Implementation Order

### Stage 1: Research & Proof of Concept (4-6 hours)

1. Create minimal example using rmcp client API
2. Test TokioChildProcess with external MCP server
3. Verify type mappings (Content, Tool, etc.)
4. Document API differences

**Deliverable**: Working POC that spawns MCP server and calls tool

### Stage 2: botticelli_mcp_client Migration (16-24 hours)

1. Update Cargo.toml
2. Migrate external_client.rs (core client)
   - Create new ExternalMcpClient using rmcp
   - Test with simple server
3. Migrate orchestrator.rs (type conversions)
4. Migrate tool_registry.rs
5. Migrate tools/*.rs (bulk import updates)
6. Run integration tests
7. Fix any issues

**Deliverable**: botticelli_mcp_client compiles and tests pass

### Stage 3: botticelli_tui Migration (2-4 hours)

1. Analyze _client parameter usage
2. Remove or update to rmcp
3. Update Cargo.toml
4. Test TUI functionality

**Deliverable**: botticelli_tui compiles and runs

### Stage 4: botticelli_chat Migration (8-12 hours)

1. Update Cargo.toml
2. Migrate infrastructure.rs (client setup)
3. Migrate tool files (acts, carousel, inputs, validation)
4. Test elicitation flow
5. Fix issues

**Deliverable**: botticelli_chat compiles and tests pass

### Stage 5: Workspace Verification (2-4 hours)

1. `cargo check --workspace`
2. `cargo test --workspace`
3. `just check-all`
4. Integration testing across crates
5. Documentation updates

**Deliverable**: All checks pass, documentation updated

## Total Effort Estimate

- **Stage 1 (POC)**: 4-6 hours
- **Stage 2 (mcp_client)**: 16-24 hours
- **Stage 3 (tui)**: 2-4 hours
- **Stage 4 (chat)**: 8-12 hours
- **Stage 5 (verification)**: 2-4 hours

**Total: 32-50 hours** (4-6 days)

## Risk Assessment

### High Risk Areas

1. **rmcp client API differences**
   - Mitigation: POC first, read source code
   - Fallback: Custom transport implementation

2. **TokioChildProcess limitations**
   - Mitigation: Test early with real external servers
   - Fallback: Implement custom transport using rmcp traits

3. **Type structure differences (Content, Tool)**
   - Mitigation: Create mapping layer if needed
   - Fallback: Wrapper types for compatibility

### Medium Risk Areas

1. **botticelli_chat transport type**
   - May need custom implementation
   - Check if in-process or HTTP

2. **Error handling differences**
   - Map pmcp::Error to rmcp::Error
   - May need error conversion layer

### Low Risk Areas

1. **botticelli_tui** - minimal usage
2. **Import updates** - mechanical changes

## Success Criteria

- [ ] All crates compile without pmcp dependency
- [ ] All tests pass: `cargo test --workspace`
- [ ] External MCP server connection works (filesystem, git)
- [ ] In-process communication works (tui, chat)
- [ ] Tool calling works end-to-end
- [ ] All checks pass: `just check-all`
- [ ] Documentation updated
- [ ] No performance regression

## Rollback Plan

If migration proves too difficult:

1. Keep pmcp for external clients (original plan)
2. Only migrate tui/chat to rmcp for in-process
3. Document architectural decision

## Next Steps

1. **Review this plan** - Get approval on approach
2. **POC** - Validate rmcp client API works for our use case
3. **Start Stage 2** - Begin botticelli_mcp_client migration

## Questions to Resolve

1. Does rmcp::TokioChildProcess support our external server spawning pattern?
2. Are Content/Tool types structurally compatible between pmcp and rmcp?
3. What is the exact Service trait API for tool calling?
4. Does botticelli_chat use in-process or HTTP transport?

## References

- [rmcp crates.io](https://crates.io/crates/rmcp)
- [rmcp docs.rs](https://docs.rs/rmcp)
- [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
