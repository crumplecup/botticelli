# rmcp Client Migration Plan (Revised)

## Goal

Migrate `botticelli_mcp_client` from pmcp to rmcp, using rmcp's `RunningService` directly without unnecessary wrappers.

## Key Insight

**Use rmcp's types directly!** rmcp provides:
- `RunningService<RoleClient, S>` - The client service with all methods we need
- `.list_tools()`, `.call_tool()`, `.list_resources()`, etc. - Built-in methods
- No need for custom wrappers - rmcp's design is already correct

## Type Mapping: pmcp → rmcp

| pmcp Type | rmcp Equivalent | Notes |
|-----------|-----------------|-------|
| `pmcp::Client<T>` | `rmcp::service::RunningService<RoleClient, S>` | Use directly |
| `pmcp::Transport` | `rmcp::transport::*` | TokioChildProcess, etc. |
| `pmcp::Content` | `rmcp::model::Content` | Part of CallToolResult |
| `pmcp::ToolInfo` | `rmcp::model::Tool` | From list_tools() |
| Custom ChildProcessTransport | `rmcp::transport::TokioChildProcess` | Built-in |

## Architecture Overview

### Current (pmcp-based)

```
botticelli_mcp_client/
├── external_client.rs
│   └── ExternalMcpClient { client: pmcp::Client<Transport> }
├── client.rs
│   └── McpHost { external_clients: HashMap<String, ExternalMcpClient> }
├── orchestrator.rs
│   └── Uses pmcp::Content, pmcp::ToolInfo
└── tool_registry.rs (legacy)
```

### Target (rmcp-based)

```
botticelli_mcp_client/
├── connection.rs
│   └── Helper functions that return RunningService
├── client.rs
│   └── McpHost { services: HashMap<String, RunningService<...>> }
├── orchestrator.rs
│   └── Uses rmcp::model types directly
└── (delete tool_registry.rs, external_client.rs)
```

## Phase 1: Add rmcp Dependency & Connection Helpers

**Status**: ✅ COMPLETE - Compiles successfully

**File**: `Cargo.toml`

```toml
# Keep pmcp temporarily for backward compatibility
pmcp = { version = "1.8", features = ["validation"] }

# Add rmcp with client features
rmcp = { version = "0.12.0", features = ["client", "transport-child-process"] }
```

**File**: `src/connection.rs` ✅ CREATED

```rust
//! Connection helpers for MCP servers using rmcp.

use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::ServiceExt;
use tokio::process::Command;
use tracing::instrument;

/// Connect to an MCP server via child process (stdio).
#[instrument(skip_all, fields(command, args_count = args.len()))]
pub async fn connect_stdio(
    command: &str,
    args: Vec<String>,
) -> McpClientResult<RunningService<RoleClient, ()>> {
    let service = ()
        .serve(TokioChildProcess::new(Command::new(command).configure(
            |cmd| {
                for arg in args {
                    cmd.arg(arg);
                }
            },
        ))?)
        .await
        .map_err(|e| {
            McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                "Failed to connect via stdio: {}",
                e
            )))
        })?;

    tracing::info!(command, "Connected to MCP server via stdio");
    Ok(service)
}
```

**Effort**: 1 hour

## Phase 2: Update McpHost to Use RunningService

**File**: `src/client.rs`

**Current**:
```rust
pub struct McpHost {
    internal_registry: ToolRegistry,
    external_clients: HashMap<String, ExternalMcpClient>,
    // ...
}
```

**Target**:
```rust
use rmcp::service::{RoleClient, RunningService};
use std::collections::HashMap;

/// Manages multiple MCP server connections.
pub struct McpHost {
    /// Connected MCP services by name
    services: HashMap<String, RunningService<RoleClient, ()>>,

    /// Approval manager for tool execution
    approval_manager: ApprovalManager,

    /// Retry configuration
    retry_config: RetryConfig,

    /// Optional metrics
    metrics: Option<McpClientMetrics>,
}

impl McpHost {
    /// Add a connected service.
    #[instrument(skip(self, service), fields(name = %name))]
    pub fn add_service(&mut self, name: String, service: RunningService<RoleClient, ()>) {
        self.services.insert(name, service);
    }

    /// List all tools from all connected services.
    #[instrument(skip(self))]
    pub async fn list_all_tools(&mut self) -> McpClientResult<Vec<ToolDefinition>> {
        let mut all_tools = Vec::new();

        for (name, service) in &mut self.services {
            let tools_result = service
                .list_tools(Default::default())
                .await
                .map_err(|e| {
                    McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                        "Failed to list tools from {}: {}",
                        name, e
                    )))
                })?;

            for tool in tools_result.tools {
                all_tools.push(ToolDefinition::new(
                    tool.name,
                    tool.description.unwrap_or_default(),
                    tool.input_schema,
                ));
            }
        }

        Ok(all_tools)
    }

    /// Call a tool on the appropriate service.
    #[instrument(skip(self, arguments), fields(tool_name))]
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> McpClientResult<serde_json::Value> {
        // Find which service has this tool
        for (name, service) in &mut self.services {
            let tools_result = service.list_tools(Default::default()).await.map_err(|e| {
                McpClientError::new(McpClientErrorKind::ConnectionError(format!(
                    "Failed to list tools from {}: {}",
                    name, e
                )))
            })?;

            if tools_result.tools.iter().any(|t| t.name == tool_name) {
                let result = service
                    .call_tool(rmcp::model::CallToolRequestParam {
                        name: tool_name.into(),
                        arguments: arguments.as_object().cloned(),
                    })
                    .await
                    .map_err(|e| {
                        McpClientError::new(McpClientErrorKind::ToolExecutionFailed(format!(
                            "Tool '{}' execution failed: {}",
                            tool_name, e
                        )))
                    })?;

                // Convert Content to JSON
                return content_to_json(&result.content);
            }
        }

        Err(McpClientError::new(McpClientErrorKind::ToolNotFound(
            format!("No connected service has tool: {}", tool_name),
        )))
    }
}

/// Convert rmcp Content to JSON Value.
fn content_to_json(content: &[rmcp::model::Content]) -> McpClientResult<serde_json::Value> {
    use rmcp::model::Content;

    let mut items = Vec::new();

    for item in content {
        let json = match &item.data {
            rmcp::model::RawContent::Text(text_content) => serde_json::json!({
                "type": "text",
                "text": text_content.text
            }),
            rmcp::model::RawContent::Image(image_content) => serde_json::json!({
                "type": "image",
                "data": image_content.data,
                "mime_type": image_content.mime_type
            }),
            rmcp::model::RawContent::Resource(resource) => serde_json::json!({
                "type": "resource",
                "resource": resource.resource
            }),
            rmcp::model::RawContent::Audio(audio) => serde_json::json!({
                "type": "audio",
                "data": audio.data,
                "mime_type": audio.mime_type
            }),
            rmcp::model::RawContent::ResourceLink(link) => serde_json::json!({
                "type": "resource_link",
                "uri": link.uri
            }),
        };
        items.push(json);
    }

    if items.len() == 1 {
        Ok(items.into_iter().next().unwrap())
    } else {
        Ok(serde_json::json!(items))
    }
}
```

**Effort**: 3-4 hours

## Phase 3: Update Orchestrator

**File**: `src/orchestrator.rs`

**Changes**:
- Remove pmcp type imports
- Use rmcp::model types
- Update to work with RunningService via McpHost

The orchestrator doesn't need to change much - it uses McpHost's interface which we're maintaining.

**Effort**: 1-2 hours

## Phase 4: Delete Legacy Code

**Delete**:
- `src/external_client.rs` - Replaced by connection helpers + RunningService
- `src/tool_registry.rs` - Legacy trait-based registry
- `src/tools/*.rs` - Legacy trait-based tools (already migrated to rmcp in botticelli_mcp)
- `src/transport.rs` - Custom transport implementations

**Update**: `src/lib.rs`

Remove exports:
```rust
// DELETE these
pub use external_client::{ExternalMcpClient, ExternalServerConfig};
pub use tool_registry::{ToolHandler, ToolRegistry};
pub use transport::{HttpTransport, McpTransport, StdioTransport};
pub use tools::{...}; // All the legacy tool exports
```

Keep:
```rust
pub mod connection;
pub use client::McpHost;
pub use orchestrator::Orchestrator;
pub use llm_adapter::{LlmAdapter, ...};
pub use approval::{ApprovalManager, ...};
pub use retry::{RetryConfig, ...};
```

**Effort**: 2 hours

## Phase 5: Update botticelli_tui

**File**: `crates/botticelli_tui/src/minimal_loop.rs`

**Current**:
```rust
use pmcp::Client;

async fn show_warning(
    _client: &pmcp::Client<botticelli_mcp::InProcTransport>,
    _message: &str,
) { }
```

**Target**:
```rust
// If the parameter is truly unused, just remove it
async fn show_warning(_message: &str) { }

// OR if it's actually used:
use rmcp::service::{RoleClient, RunningService};

async fn show_warning(
    _service: &mut RunningService<RoleClient, ()>,
    _message: &str,
) { }
```

**Cargo.toml**:
```toml
# Remove pmcp
```

**Effort**: 1 hour

## Phase 6: Update botticelli_chat

**Files**: `crates/botticelli_chat/src/elicitation/*.rs`

**Pattern**:
```rust
// OLD
use pmcp::Client;

pub async fn elicit_act(
    client: &pmcp::Client<impl pmcp::Transport>,
    session_id: &str,
) -> Result<ElicitActResult> {
    let result = client.call_tool("elicit_act", args).await?;
}

// NEW
use rmcp::service::{RoleClient, RunningService};
use rmcp::model::CallToolRequestParam;

pub async fn elicit_act(
    service: &mut RunningService<RoleClient, ()>,
    session_id: &str,
) -> Result<ElicitActResult> {
    let result = service
        .call_tool(CallToolRequestParam {
            name: "elicit_act".into(),
            arguments: serde_json::json!({ "session_id": session_id }).as_object().cloned(),
        })
        .await?;

    // Convert Content to desired format
    Ok(serde_json::from_value(content_to_json(&result.content)?)?)
}
```

**Files to update**:
- `src/elicitation/infrastructure.rs`
- `src/elicitation/acts.rs`
- `src/elicitation/carousel.rs`
- `src/elicitation/inputs.rs`
- `src/elicitation/validation.rs`

**Cargo.toml**:
```toml
# Remove pmcp
```

**Effort**: 3-4 hours

## Phase 7: Remove pmcp Dependency

**All Cargo.toml files**:
```toml
# DELETE
# pmcp = { version = "1.8", features = [...] }
```

**Verify**:
```bash
cargo check --workspace
cargo test --workspace
just check-all
```

**Effort**: 1 hour

## Total Effort Estimate

| Phase | Task | Hours |
|-------|------|-------|
| 1 | Connection helpers | 1 |
| 2 | Update McpHost | 3-4 |
| 3 | Update orchestrator | 1-2 |
| 4 | Delete legacy code | 2 |
| 5 | Update botticelli_tui | 1 |
| 6 | Update botticelli_chat | 3-4 |
| 7 | Remove pmcp | 1 |
| **Total** | | **12-15 hours** |

## Key Advantages of This Approach

1. **Simpler**: Use rmcp's types directly, no unnecessary wrappers
2. **Maintainable**: Less code, fewer abstractions
3. **Correct**: Follows rmcp's intended design patterns
4. **Flexible**: Can still add thin convenience layers later if needed
5. **Testable**: rmcp provides testing infrastructure

## Example Usage

```rust
use botticelli_mcp_client::connection;

// Connect to MCP server
let mut service = connection::connect_stdio(
    "npx",
    vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
).await?;

// Use RunningService directly
let tools = service.list_tools(Default::default()).await?;
let result = service.call_tool(rmcp::model::CallToolRequestParam {
    name: "read_file".into(),
    arguments: serde_json::json!({"path": "foo.txt"}).as_object().cloned(),
}).await?;

// Or use McpHost for multiple services
let mut host = McpHost::builder().build();
host.add_service("filesystem".to_string(), service);
let result = host.call_tool("read_file", serde_json::json!({"path": "foo.txt"})).await?;
```

## Migration Order

1. ✅ Phase 1: Connection helpers (COMPLETE - compiles successfully)
2. Phase 2-4: botticelli_mcp_client core migration
3. Verify: `cargo check -p botticelli_mcp_client`
4. Phase 5: botticelli_tui
5. Verify: `cargo check -p botticelli_tui`
6. Phase 6: botticelli_chat
7. Verify: `cargo check -p botticelli_chat`
8. Phase 7: Remove pmcp
9. Final: `cargo test --workspace && just check-all`

## Success Criteria

- [ ] No pmcp dependencies in workspace
- [ ] All crates use rmcp's RunningService
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Can connect to external MCP servers
- [ ] All tool calling works end-to-end
- [ ] `just check-all` passes
