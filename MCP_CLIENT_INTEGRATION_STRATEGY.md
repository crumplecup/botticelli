# MCP Client Integration Strategy (pmcp-based)

**Document Version:** 1.0  
**Date:** 2025-12-15  
**Status:** Planning  
**Effort Estimate:** 1-2 weeks

## Executive Summary

Integrate pmcp's `Client` capability into Botticelli to enable connection to external MCP servers. This unlocks the MCP ecosystem (filesystem, git, search, etc.) for narrative execution and makes Botticelli a complete platform: **MCP server + MCP client + narrative orchestrator**.

**Key Innovation:** Leverage pmcp SDK we already integrated - minimal new code, maximum capability.

---

## Strategic Value

### 1. Ecosystem Access
- **50+ existing MCP servers** available immediately
- Filesystem server (safe file operations)
- Git server (repo management)
- Search servers (Brave, Exa)
- Cloud APIs (AWS, Google Cloud)
- Development tools (GitHub, Linear, Slack)

### 2. Narrative Superpowers
```toml
# Narratives can now orchestrate external tools!
[[acts]]
model_name = "claude-3-5-sonnet"
system_prompt = "You are a code review assistant"
user_prompt = "Review src/ directory for bugs"

# Executor automatically provides:
# - Botticelli's tools (create_narrative, etc.)
# - Filesystem tools (read_file, list_directory)
# - Git tools (git_diff, git_log)
```

### 3. Security Boundary
- External servers handle dangerous operations (file I/O, exec)
- They implement proper sandboxing/permissions
- Botticelli stays safe - just routes requests

### 4. Competitive Positioning
**Most frameworks are ONE thing:**
- Function calling only (no MCP)
- MCP server only (us currently)

**Botticelli becomes ALL:**
- ✅ MCP Server (our tools)
- ✅ MCP Client (ecosystem tools) ← NEW
- ✅ Narrative Orchestrator (unique)
- ✅ Multi-model LLM framework

---

## Architecture Overview

### Current State
```
Narrative Executor
    ├─→ Botticelli Tools (internal)
    └─→ LLM Backends (Anthropic, Gemini, etc.)
```

### Target State
```
Narrative Executor
    ├─→ Internal Tools (botticelli_mcp server)
    ├─→ External MCP Servers (via pmcp::Client) ← NEW
    │    ├─ Filesystem Server
    │    ├─ Git Server
    │    ├─ Search Server
    │    └─ [User-provided servers]
    └─→ LLM Backends (botticelli_models)
```

### Tool Discovery & Routing
```rust
// Narrative executor discovers ALL tools
let tools = executor.discover_all_tools().await?;
// Returns: ["echo", "create_narrative", ..., "read_file", "git_diff", ...]

// LLM gets combined tool list
let response = llm.generate_with_tools(&messages, &tools).await?;

// Executor routes tool calls appropriately
for tool_call in response.tool_calls {
    if internal_tool(&tool_call.name) {
        // Execute internal tool
    } else if let Some(client) = find_external_client(&tool_call.name) {
        // Route to external server
        client.call_tool(&tool_call.name, &tool_call.args).await?
    }
}
```

---

## Phase 1: Client Wrapper (2-3 days)

### Goal
Create `ExternalMcpClient` that wraps pmcp::Client with Botticelli-specific features.

### Step 1.1: Create botticelli_mcp_client crate
**Effort:** 2 hours

```toml
# Create new crate
[package]
name = "botticelli_mcp_client"
version = "0.2.0"

[dependencies]
pmcp = { version = "1.8", features = ["validation"] }
botticelli_error = { path = "../botticelli_error" }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
```

**Success Criteria:**
- ✅ New crate compiles
- ✅ Depends on pmcp SDK
- ✅ Uses botticelli_error for errors

**Rollback:** Delete crate directory

---

### Step 1.2: Implement ExternalMcpClient
**Effort:** 6 hours

```rust
// crates/botticelli_mcp_client/src/client.rs

use pmcp::{Client, ClientCapabilities, StdioTransport};
use serde_json::Value;
use tracing::{debug, info, instrument, warn};

/// Configuration for connecting to external MCP server
#[derive(Debug, Clone)]
pub struct ExternalServerConfig {
    /// Server identifier (e.g., "filesystem", "git")
    pub name: String,
    
    /// Command to execute (e.g., "npx", "mcp-server-git")
    pub command: String,
    
    /// Arguments to pass to command
    pub args: Vec<String>,
    
    /// Optional: Restrict which tools can be used
    pub allowed_tools: Option<Vec<String>>,
    
    /// Optional: Timeout for operations
    pub timeout_seconds: Option<u64>,
}

/// Client for connecting to external MCP servers
pub struct ExternalMcpClient {
    /// Server identifier
    name: String,
    
    /// pmcp client instance
    client: Client,
    
    /// Available tools from this server
    tools: Vec<ToolDefinition>,
    
    /// Allowed tools (if restricted)
    allowed_tools: Option<Vec<String>>,
    
    /// Metrics
    call_count: AtomicU64,
}

impl ExternalMcpClient {
    /// Connect to external MCP server
    #[instrument(skip(config))]
    pub async fn connect(config: ExternalServerConfig) -> Result<Self> {
        info!(
            "Connecting to external MCP server: {} ({})",
            config.name, config.command
        );

        // Create stdio transport to external process
        let transport = StdioTransport::with_command(&config.command, &config.args)?;
        let mut client = Client::new(transport);

        // Initialize with minimal capabilities
        let capabilities = ClientCapabilities::minimal();
        let server_info = client.initialize(capabilities).await
            .map_err(|e| McpClientError::connection_failed(
                format!("Failed to initialize {}: {}", config.name, e)
            ))?;

        info!(
            "Connected to {} (version {})",
            server_info.server_info.name,
            server_info.server_info.version
        );

        // Discover available tools
        let tools_result = client.list_tools(None).await
            .map_err(|e| McpClientError::discovery_failed(
                format!("Failed to list tools from {}: {}", config.name, e)
            ))?;

        info!(
            "Discovered {} tools from {}",
            tools_result.tools.len(),
            config.name
        );

        for tool in &tools_result.tools {
            debug!("  - {}", tool.name);
        }

        Ok(Self {
            name: config.name,
            client,
            tools: tools_result.tools,
            allowed_tools: config.allowed_tools,
            call_count: AtomicU64::new(0),
        })
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get available tools
    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    /// Check if server has a specific tool
    pub fn has_tool(&self, tool_name: &str) -> bool {
        // Check allowed list first
        if let Some(allowed) = &self.allowed_tools {
            if !allowed.contains(&tool_name.to_string()) {
                return false;
            }
        }

        self.tools.iter().any(|t| t.name == tool_name)
    }

    /// Call a tool on the external server
    #[instrument(skip(self, arguments), fields(server = %self.name, tool = %tool_name))]
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        // Verify tool exists and is allowed
        if !self.has_tool(tool_name) {
            return Err(McpClientError::tool_not_found(
                format!("Tool '{}' not available on server '{}'", tool_name, self.name)
            ));
        }

        debug!(
            "Calling tool '{}' on server '{}' with args: {}",
            tool_name, self.name, arguments
        );

        // Call via pmcp client
        let result = self.client
            .call_tool(tool_name.to_string(), arguments)
            .await
            .map_err(|e| McpClientError::tool_execution_failed(
                format!("Tool '{}' failed on server '{}': {}", tool_name, self.name, e)
            ))?;

        // Track metrics
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Extract content from result
        Ok(result.content)
    }

    /// Get call statistics
    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::SeqCst)
    }
}
```

**Success Criteria:**
- ✅ Connects to external MCP servers via stdio
- ✅ Discovers tools on initialization
- ✅ Routes tool calls to external server
- ✅ Proper error handling with McpClientError
- ✅ Instrumentation with tracing spans
- ✅ Tool allowlist support

**Testing:**
```bash
# Test with filesystem server
cargo test --test external_client_test -- --ignored
```

**Rollback:** Delete client.rs

---

### Step 1.3: Error Types
**Effort:** 1 hour

```rust
// crates/botticelli_mcp_client/src/error.rs

use derive_more::{Display, Error};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum McpClientErrorKind {
    #[display("Connection failed: {}", _0)]
    ConnectionFailed(String),

    #[display("Tool discovery failed: {}", _0)]
    DiscoveryFailed(String),

    #[display("Tool not found: {}", _0)]
    ToolNotFound(String),

    #[display("Tool execution failed: {}", _0)]
    ToolExecutionFailed(String),

    #[display("Server timeout: {}", _0)]
    Timeout(String),

    #[display("Invalid configuration: {}", _0)]
    InvalidConfiguration(String),
}

#[derive(Debug, Clone, Display, Error)]
#[display("MCP Client Error: {} at {}:{}", kind, file, line)]
pub struct McpClientError {
    pub kind: McpClientErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl McpClientError {
    #[track_caller]
    pub fn new(kind: McpClientErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }

    #[track_caller]
    pub fn connection_failed(msg: impl Into<String>) -> Self {
        Self::new(McpClientErrorKind::ConnectionFailed(msg.into()))
    }

    // ... other constructors
}

pub type Result<T> = std::result::Result<T, McpClientError>;
```

**Success Criteria:**
- ✅ Follows botticelli_error pattern
- ✅ Uses derive_more for Display/Error
- ✅ Track caller for location info

**Rollback:** Delete error.rs

---

### Step 1.4: Basic Tests
**Effort:** 3 hours

```rust
// crates/botticelli_mcp_client/tests/external_client_test.rs

#[tokio::test]
#[ignore] // Requires external server
async fn test_connect_to_filesystem_server() {
    let config = ExternalServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/tmp".to_string(),
        ],
        allowed_tools: None,
        timeout_seconds: Some(30),
    };

    let client = ExternalMcpClient::connect(config).await;
    assert!(client.is_ok(), "Should connect to filesystem server");

    let client = client.unwrap();
    assert!(client.has_tool("read_file"), "Should have read_file tool");
    assert!(client.has_tool("list_directory"), "Should have list_directory tool");
}

#[tokio::test]
#[ignore]
async fn test_call_tool_on_external_server() {
    let config = ExternalServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/tmp".to_string(),
        ],
        allowed_tools: None,
        timeout_seconds: Some(30),
    };

    let mut client = ExternalMcpClient::connect(config).await.unwrap();

    // List directory
    let result = client.call_tool("list_directory", json!({
        "path": "/tmp"
    })).await;

    assert!(result.is_ok(), "Should list directory successfully");
}
```

**Success Criteria:**
- ✅ Tests connect to real external server
- ✅ Tests discover and call tools
- ✅ Ignored by default (require --ignored flag)

**Rollback:** Delete test file

---

## Phase 2: Narrative Integration (3-4 days)

### Goal
Integrate external clients into narrative execution engine.

### Step 2.1: Update NarrativeExecutor
**Effort:** 4 hours

```rust
// crates/botticelli_narrative/src/executor.rs

pub struct NarrativeExecutor {
    // Existing fields
    llm_backend: Arc<dyn LlmBackend>,
    
    // NEW: External MCP clients
    external_clients: Vec<ExternalMcpClient>,
}

impl NarrativeExecutor {
    /// Add an external MCP server for this execution
    pub fn with_external_server(
        mut self,
        config: ExternalServerConfig,
    ) -> Result<Self> {
        let client = ExternalMcpClient::connect(config).await?;
        self.external_clients.push(client);
        Ok(self)
    }

    /// Discover all available tools (internal + external)
    async fn discover_all_tools(&self) -> Result<Vec<ToolDefinition>> {
        let mut tools = Vec::new();

        // Add internal Botticelli tools
        tools.extend(self.discover_internal_tools());

        // Add tools from external servers
        for client in &self.external_clients {
            for tool in client.tools() {
                // Namespace external tools
                let namespaced_name = format!("{}:{}", client.name(), tool.name);
                tools.push(ToolDefinition {
                    name: namespaced_name,
                    description: format!(
                        "[{}] {}",
                        client.name(),
                        tool.description.as_deref().unwrap_or("")
                    ),
                    input_schema: tool.input_schema.clone(),
                });
            }
        }

        Ok(tools)
    }

    /// Route tool call to appropriate handler
    async fn execute_tool_call(
        &mut self,
        tool_call: &ToolCall,
    ) -> Result<Value> {
        // Check if it's a namespaced external tool
        if let Some((server_name, tool_name)) = tool_call.name.split_once(':') {
            // Route to external server
            if let Some(client) = self.external_clients
                .iter_mut()
                .find(|c| c.name() == server_name)
            {
                return client.call_tool(tool_name, tool_call.args.clone()).await;
            }
        }

        // Fall back to internal tools
        self.execute_internal_tool(tool_call).await
    }
}
```

**Success Criteria:**
- ✅ Executor can load external clients
- ✅ Tool discovery includes external tools
- ✅ Tool routing works (internal vs external)
- ✅ Namespacing prevents conflicts

**Testing:**
```bash
cargo test -p botticelli_narrative --test executor_external_test
```

**Rollback:** Revert executor.rs changes

---

### Step 2.2: TOML Configuration
**Effort:** 3 hours

```toml
# Example narrative with external servers
[metadata]
title = "Code Review with File Access"
description = "Review code using filesystem tools"

# Configure external MCP servers
[external_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
allowed_tools = ["read_file", "list_directory"]  # Optional whitelist

[external_servers.git]
command = "mcp-server-git"
args = ["--repo", "."]

[[acts]]
model_name = "claude-3-5-sonnet"
system_prompt = """
You are a code review assistant with access to:
- filesystem:read_file - Read source files
- filesystem:list_directory - List directories
- git:git_diff - Show git changes
- git:git_log - Show commit history
"""
user_prompt = "Review the changes in src/ directory"
```

```rust
// Update narrative parser
#[derive(Debug, Clone, Deserialize)]
pub struct NarrativeToml {
    pub metadata: MetadataSection,
    
    // NEW: External servers configuration
    #[serde(default)]
    pub external_servers: HashMap<String, ExternalServerToml>,
    
    pub acts: Vec<ActSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalServerToml {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}
```

**Success Criteria:**
- ✅ TOML parser handles external_servers section
- ✅ Backward compatible (optional section)
- ✅ Proper validation

**Rollback:** Revert parser changes

---

### Step 2.3: End-to-End Integration
**Effort:** 4 hours

```rust
// Update narrative execution
pub async fn execute_narrative(
    toml_content: &str,
) -> Result<ExecutionOutput> {
    // Parse narrative
    let narrative: NarrativeToml = toml::from_str(toml_content)?;

    // Create executor
    let mut executor = NarrativeExecutor::new(llm_backend);

    // Load external servers
    for (name, config) in narrative.external_servers {
        let server_config = ExternalServerConfig {
            name,
            command: config.command,
            args: config.args,
            allowed_tools: config.allowed_tools,
            timeout_seconds: config.timeout_seconds,
        };
        executor = executor.with_external_server(server_config).await?;
    }

    // Execute narrative with combined tools
    executor.execute(&narrative).await
}
```

**Success Criteria:**
- ✅ Narratives can use external tools
- ✅ LLM receives combined tool list
- ✅ Tool calls route correctly
- ✅ Errors propagate cleanly

**Testing:**
```bash
# Integration test with real external server
cargo test --test narrative_external_integration -- --ignored
```

**Rollback:** Revert integration code

---

## Phase 3: Enhanced Features (2-3 days)

### Step 3.1: Connection Pooling
**Effort:** 4 hours

Multiple narratives can share external clients:

```rust
pub struct ExternalClientPool {
    clients: Arc<RwLock<HashMap<String, ExternalMcpClient>>>,
}

impl ExternalClientPool {
    pub async fn get_or_connect(
        &self,
        config: &ExternalServerConfig,
    ) -> Result<ExternalMcpClient> {
        // Check if client exists
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&config.name) {
                return Ok(client.clone());
            }
        }

        // Connect new client
        let client = ExternalMcpClient::connect(config.clone()).await?;
        
        // Store in pool
        {
            let mut clients = self.clients.write().await;
            clients.insert(config.name.clone(), client.clone());
        }

        Ok(client)
    }
}
```

---

### Step 3.2: Tool Validation
**Effort:** 3 hours

Validate tool calls before sending to external servers:

```rust
impl ExternalMcpClient {
    pub fn validate_tool_call(
        &self,
        tool_name: &str,
        arguments: &Value,
    ) -> Result<()> {
        // Find tool definition
        let tool = self.tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| McpClientError::tool_not_found(tool_name))?;

        // Validate against JSON schema
        if !tool.input_schema.is_null() {
            jsonschema::validate(&tool.input_schema, arguments)
                .map_err(|e| McpClientError::invalid_arguments(e))?;
        }

        Ok(())
    }
}
```

---

### Step 3.3: Metrics & Observability
**Effort:** 2 hours

```rust
pub struct ExternalClientMetrics {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub average_latency_ms: f64,
}

impl ExternalMcpClient {
    pub fn metrics(&self) -> ExternalClientMetrics {
        // Return aggregated metrics
    }
}
```

---

## Phase 4: Testing & Documentation (2-3 days)

### Step 4.1: Integration Tests
**Effort:** 1 day

```bash
# Test with real external servers
cargo test --test filesystem_integration -- --ignored
cargo test --test git_integration -- --ignored
cargo test --test narrative_with_external_tools -- --ignored
```

---

### Step 4.2: Documentation
**Effort:** 1 day

- Update README with external server examples
- Document TOML configuration format
- Add troubleshooting guide
- Create example narratives

---

### Step 4.3: Examples
**Effort:** 1 day

Create example narratives:
- `examples/narrative_with_filesystem.toml`
- `examples/narrative_with_git.toml`
- `examples/narrative_with_multiple_servers.toml`

---

## Success Criteria

**Phase 1 Complete:**
- ✅ ExternalMcpClient connects to external servers
- ✅ Tool discovery works
- ✅ Tool calling works
- ✅ Basic tests pass

**Phase 2 Complete:**
- ✅ Narrative executor uses external clients
- ✅ TOML configuration works
- ✅ End-to-end narrative execution works

**Phase 3 Complete:**
- ✅ Connection pooling implemented
- ✅ Tool validation added
- ✅ Metrics collection works

**Phase 4 Complete:**
- ✅ Integration tests pass
- ✅ Documentation complete
- ✅ Example narratives work

---

## Risk Assessment

### Low Risk ✅
- **pmcp SDK mature**: Already proven in server implementation
- **Clear API**: Examples show exactly how to use Client
- **Isolated changes**: New crate, minimal changes to existing code
- **Backward compatible**: External servers are optional

### Medium Risk ⚠️
- **Process management**: Starting/stopping external processes
- **Error handling**: External servers can crash/hang
- **Configuration complexity**: Users must specify servers

### Mitigation
- Use pmcp's built-in process management
- Comprehensive error handling with timeouts
- Provide sensible defaults and examples
- Clear error messages for configuration issues

---

## Timeline

**Week 1:**
- Days 1-2: Phase 1 (Client wrapper)
- Days 3-4: Phase 2 Part 1 (Executor integration)
- Day 5: Phase 2 Part 2 (TOML configuration)

**Week 2:**
- Days 1-2: Phase 2 Part 3 (End-to-end) + Phase 3 (Enhanced features)
- Days 3-5: Phase 4 (Testing & documentation)

**Total:** 10 days (~2 weeks)

---

## Future Enhancements

### Beyond Scope (For Later)
- HTTP/WebSocket transport for remote servers
- Authentication/authorization for external servers
- Resource limits (CPU, memory) per server
- Server health monitoring
- Hot-reload configuration
- Server marketplace/registry

---

## Related Documents

- PMCP_MIGRATION_STRATEGY.md (completed)
- NARRATIVE_TOML_SPEC.md (needs update for external_servers)
- Examples in `/tmp/rust-mcp-sdk/examples/` (reference)

---

## Appendix A: Example Use Cases

### Use Case 1: Code Review
```toml
[external_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "./src"]

[external_servers.git]
command = "mcp-server-git"
args = ["--repo", "."]

[[acts]]
model_name = "claude-3-5-sonnet"
user_prompt = "Review recent changes for bugs"
```

### Use Case 2: Content Generation
```toml
[external_servers.search]
command = "mcp-server-brave-search"

[[acts]]
model_name = "gpt-4"
user_prompt = "Research latest AI trends and write article"
```

### Use Case 3: Data Analysis
```toml
[external_servers.database]
command = "mcp-server-postgres"
args = ["--connection-string", "$DATABASE_URL"]

[[acts]]
model_name = "claude-3-opus"
user_prompt = "Analyze sales data and generate report"
```

---

## Appendix B: pmcp Client API Reference

```rust
// Key pmcp types we'll use
use pmcp::{
    Client,
    ClientCapabilities,
    StdioTransport,
    types::{
        ToolDefinition,
        CallToolResult,
    },
};

// Connection
let transport = StdioTransport::with_command("command", &["args"])?;
let mut client = Client::new(transport);

// Initialize
let server_info = client.initialize(ClientCapabilities::minimal()).await?;

// Discover tools
let tools = client.list_tools(None).await?;

// Call tool
let result = client.call_tool("tool_name".to_string(), arguments).await?;
```

---

## Summary

This strategy leverages our existing pmcp integration to add MCP client capability with minimal effort. The result is a complete platform where narratives can orchestrate both internal Botticelli tools and external ecosystem tools.

**Estimated Effort:** 10 days (2 weeks)  
**Risk:** Low (pmcp SDK proven, isolated changes)  
**Value:** High (ecosystem access, competitive advantage)  
**Status:** Ready to implement

**Next Step:** Begin Phase 1, Step 1.1 - Create botticelli_mcp_client crate
