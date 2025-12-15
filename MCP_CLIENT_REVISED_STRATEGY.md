# MCP Client Integration - Revised Strategy

## Audit Results

### Existing MCP Client Crate Status

**Location:** `crates/botticelli_mcp_client`  
**Purpose:** Agentic loop orchestrator (self-driving Botticelli)  
**Current State:** Partial implementation (~1,200 lines)

#### What It Does
- Manages agentic conversation loops
- Orchestrates LLM + tool interactions
- Handles tool result feedback
- Circuit breaking, retries, metrics

#### What It Doesn't Do
- ❌ Connect to external MCP servers (filesystem, git, etc.)
- ❌ Use pmcp SDK
- ❌ Implement MCP protocol client

### Architecture Confusion

We have **two different "MCP client" concepts**:

**1. Self-Driving Client (Existing)**
```
McpClient (in botticelli_mcp_client)
    ├─→ Takes LLM backend
    ├─→ Manages conversation loop
    ├─→ Executes tools (TODO: needs MCP integration)
    └─→ Returns final response
```

**Purpose:** Make Botticelli self-driving by using its own MCP tools

**2. External Server Client (New - What We Want)**
```
ExternalMcpClient (needs implementation)
    ├─→ Connects to external MCP servers (filesystem, git, etc.)
    ├─→ Uses pmcp::Client
    ├─→ Discovers tools from external servers
    └─→ Routes tool calls to external servers
```

**Purpose:** Access MCP ecosystem (50+ servers)

---

## Revised Integration Plan

### Strategy: Complete Both Clients

**Phase A:** Complete the self-driving client (finish existing work)  
**Phase B:** Add external server client (new pmcp-based functionality)

---

## Phase A: Complete Self-Driving Client (1-2 days)

### Goal
Finish the existing `botticelli_mcp_client` so it can call our MCP server tools.

### Current TODOs in Code
```rust
// tool_executor.rs line 46-48:
// TODO: Actual tool execution will integrate with MCP server
// For now, return placeholder
```

### Step A.1: Connect to Own MCP Server
**Effort:** 4 hours

```rust
// Update tool_executor.rs to actually call MCP server

use botticelli_mcp::tools::McpTool;  // Our internal tools

pub struct ToolExecutor {
    tools: HashMap<String, Arc<dyn McpTool>>,  // Store actual tool instances
}

impl ToolExecutor {
    pub fn with_internal_tools() -> Self {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        
        // Register all internal tools
        tools.insert("echo".to_string(), Arc::new(EchoTool));
        tools.insert("server_info".to_string(), Arc::new(ServerInfoTool));
        tools.insert("create_narrative".to_string(), Arc::new(CreateNarrativeTool));
        // ... etc
        
        Self { tools }
    }
    
    pub async fn execute(&self, tool_name: &str, arguments: Value) -> McpClientResult<Value> {
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| McpClientError::new(McpClientErrorKind::ToolNotFound(tool_name.to_string())))?;
        
        // Actually execute the tool
        tool.execute(arguments)
            .await
            .map_err(|e| McpClientError::new(McpClientErrorKind::ToolExecutionFailed(e.to_string())))
    }
}
```

### Step A.2: Test Self-Driving Loop
**Effort:** 2 hours

```rust
#[tokio::test]
async fn test_self_driving_loop() {
    let backend = get_test_llm();
    let client = McpClient::builder()
        .max_iterations(5)
        .build()
        .with_internal_tools();  // Connect to our tools
    
    let messages = vec![
        Message::builder()
            .role(Role::User)
            .content(vec![Input::Text("Create a simple narrative about AI".to_string())])
            .build()
            .unwrap()
    ];
    
    let result = client.execute(&backend, messages).await;
    assert!(result.is_ok());
}
```

### Success Criteria
- ✅ ToolExecutor actually calls MCP tools
- ✅ Agentic loop works end-to-end
- ✅ Self-driving tests pass

---

## Phase B: Add External Server Client (1-2 weeks)

This is the **NEW** functionality from our original strategy document.

### Goal
Add `ExternalMcpClient` that uses pmcp::Client to connect to external servers.

### Architecture
```
crates/botticelli_mcp_client/
├── src/
│   ├── client.rs           # Existing: Self-driving orchestrator
│   ├── external_client.rs  # NEW: pmcp-based external server client
│   ├── tool_executor.rs    # Updated: Route to internal OR external
│   └── ...
```

### Step B.1: Add External Client Module
**Effort:** 6 hours

```rust
// src/external_client.rs - NEW FILE

use pmcp::{Client, ClientCapabilities, StdioTransport};
use serde_json::Value;

/// Client for connecting to external MCP servers (filesystem, git, etc.)
pub struct ExternalMcpClient {
    name: String,
    client: Client,
    tools: Vec<ToolDefinition>,
    allowed_tools: Option<Vec<String>>,
}

impl ExternalMcpClient {
    /// Connect to external MCP server
    pub async fn connect(config: ExternalServerConfig) -> McpClientResult<Self> {
        // Create pmcp client
        let transport = StdioTransport::with_command(&config.command, &config.args)?;
        let mut client = Client::new(transport);
        
        // Initialize
        let server_info = client.initialize(ClientCapabilities::minimal()).await?;
        
        // Discover tools
        let tools_result = client.list_tools(None).await?;
        
        Ok(Self {
            name: config.name,
            client,
            tools: tools_result.tools,
            allowed_tools: config.allowed_tools,
        })
    }
    
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> McpClientResult<Value> {
        // Validate tool exists
        if !self.has_tool(tool_name) {
            return Err(McpClientError::new(
                McpClientErrorKind::ToolNotFound(format!("{}:{}", self.name, tool_name))
            ));
        }
        
        // Call via pmcp
        let result = self.client
            .call_tool(tool_name.to_string(), arguments)
            .await?;
        
        Ok(result.content)
    }
}
```

### Step B.2: Update ToolExecutor to Route
**Effort:** 4 hours

```rust
// Update tool_executor.rs

pub struct ToolExecutor {
    // Internal tools (our MCP server tools)
    internal_tools: HashMap<String, Arc<dyn McpTool>>,
    
    // External MCP servers (filesystem, git, etc.)
    external_clients: Vec<ExternalMcpClient>,
}

impl ToolExecutor {
    pub async fn execute(&self, tool_name: &str, arguments: Value) -> McpClientResult<Value> {
        // Check if it's a namespaced external tool (e.g., "filesystem:read_file")
        if let Some((server_name, tool)) = tool_name.split_once(':') {
            // Route to external server
            if let Some(client) = self.external_clients.iter_mut()
                .find(|c| c.name() == server_name)
            {
                return client.call_tool(tool, arguments).await;
            }
        }
        
        // Fall back to internal tool
        if let Some(tool) = self.internal_tools.get(tool_name) {
            return tool.execute(arguments).await
                .map_err(|e| McpClientError::new(
                    McpClientErrorKind::ToolExecutionFailed(e.to_string())
                ));
        }
        
        Err(McpClientError::new(McpClientErrorKind::ToolNotFound(tool_name.to_string())))
    }
}
```

### Step B.3: Add Cargo.toml Dependencies
**Effort:** 1 hour

```toml
[dependencies]
# Existing...
botticelli_mcp = { path = "../botticelli_mcp", version = "0.2.0" }

# NEW: Add pmcp for external server connections
pmcp = { version = "1.8", features = ["validation"] }
```

### Step B.4: Integration Tests
**Effort:** 3 hours

```rust
#[tokio::test]
#[ignore] // Requires external server
async fn test_external_filesystem_server() {
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
    
    let result = client.call_tool("read_file", json!({
        "path": "/tmp/test.txt"
    })).await;
    
    assert!(result.is_ok());
}
```

### Success Criteria
- ✅ ExternalMcpClient connects to external servers
- ✅ ToolExecutor routes correctly (internal vs external)
- ✅ Tests pass with real filesystem server
- ✅ pmcp dependency integrated

---

## Phase C: Narrative Integration (2-3 days)

### Goal
Enable narratives to configure and use external servers.

### TOML Configuration
```toml
[metadata]
title = "Code Review"

[external_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "./src"]

[external_servers.git]
command = "mcp-server-git"
args = ["--repo", "."]

[[acts]]
model_name = "claude-3-5-sonnet"
user_prompt = "Review changes in src/"
```

### Update Narrative Executor
```rust
// In botticelli_narrative/src/executor.rs

impl NarrativeExecutor {
    pub fn with_external_servers(
        mut self,
        configs: Vec<ExternalServerConfig>,
    ) -> Result<Self> {
        for config in configs {
            let client = ExternalMcpClient::connect(config).await?;
            self.tool_executor.add_external_client(client);
        }
        Ok(self)
    }
}
```

---

## Summary: Two Clients, One System

### McpClient (Existing - Self-Driving)
**Purpose:** Agentic loop orchestration  
**Connects to:** Botticelli's own MCP tools  
**Status:** 80% complete, needs tool integration  
**Effort:** 1-2 days to complete

### ExternalMcpClient (New - Ecosystem Access)
**Purpose:** Connect to external MCP servers  
**Uses:** pmcp::Client SDK  
**Status:** Not started  
**Effort:** 1-2 weeks to implement

### Combined Architecture
```
Narrative Executor
    │
    ├─→ McpClient (self-driving orchestrator)
    │       ├─→ LLM Backend
    │       └─→ ToolExecutor
    │               ├─→ Internal Tools (our MCP server)
    │               └─→ ExternalMcpClient (pmcp-based)
    │                       ├─→ Filesystem Server
    │                       ├─→ Git Server
    │                       └─→ [User servers]
    │
    └─→ Direct LLM calls (simple cases)
```

---

## Recommended Approach

### Option 1: Sequential (Recommended)
1. **Week 1:** Complete Phase A (finish self-driving client)
2. **Week 2-3:** Implement Phase B (add external client)
3. **Week 4:** Phase C (narrative integration)

### Option 2: Parallel
- One developer on Phase A
- Another on Phase B
- Merge both when complete

### Option 3: External First
Skip Phase A for now, focus on external client (Phase B)
- More strategic value
- Can return to Phase A later

---

## Decision Point

**Question for you:** Which approach do you prefer?

1. **Complete self-driving first** (finish what was started)
2. **External client first** (highest strategic value)
3. **Both in parallel** (if you want to work on both)

The original strategy document assumed a clean slate. Now that we know there's existing work, we need to decide how to integrate with it.

**My Recommendation:** **External client first (Option 3)**
- Highest strategic value (ecosystem access)
- Can integrate with existing client later
- More aligned with what we just discussed
- pmcp SDK is fresh in our minds

What do you think?
