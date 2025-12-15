# PMCP Migration Strategy

**Status:** Planning  
**Created:** 2025-12-15  
**Target Completion:** TBD

## Executive Summary

Migrate Botticelli's MCP implementation from the basic `mcp-server` + `mcp-spec` crates to the battle-tested `pmcp` SDK. The pmcp crate provides production-grade features including zero-copy parsing, comprehensive type safety, multiple transport options, built-in authentication, advanced middleware, batch operations, and observability that will significantly enhance Botticelli's MCP capabilities.

**Current State:**
- ~10,500 lines of custom MCP code across 69 Rust files
- Uses `mcp-server` 0.1.0 and `mcp-spec` 0.1.0 (basic implementations)
- Custom router, tool registry, resource management
- Manual transport handling (stdio, HTTP)
- Basic tool execution without batching or middleware
- Limited observability beyond tracing

**Target State:**
- Leverage pmcp's production-grade SDK
- 16x faster performance, 50x lower memory usage
- Zero-copy parsing for high-throughput scenarios
- Built-in OAuth 2.0 and bearer token authentication
- Advanced middleware (batching, retries, circuit breaking)
- Multiple transports (stdio, HTTP/SSE, WebSocket)
- Enhanced observability (structured logging, metrics, health endpoints)

## Benefits Analysis

### Performance Gains
- **16x faster** than TypeScript SDK equivalents
- **50x lower memory** usage via Rust zero-cost abstractions
- **Zero-copy parsing** for message handling
- **Batch operations** for notification debouncing

### Reliability Improvements
- **Comprehensive type safety** prevents runtime errors
- **Built-in retry logic** with exponential backoff
- **Circuit breakers** for fault tolerance
- **Health endpoints** for monitoring

### Developer Experience
- **Extensive documentation** (27-chapter guide)
- **Proven in production** deployments
- **Active maintenance** by Pragmatic AI Labs
- **Compatible with TypeScript SDK** for interoperability

### Maintenance Reduction
- **Less custom code** to maintain (~10.5k → ~2-3k LOC)
- **Battle-tested patterns** reduce bugs
- **SDK updates** provide new features automatically
- **Standard abstractions** improve clarity

## Migration Scope

### In Scope

#### Phase 1: Server Migration
- ✅ Replace `BotticelliRouter` with `pmcp::Server`
- ✅ Migrate tool definitions to pmcp's `ToolHandler` trait
- ✅ Convert resource management to pmcp patterns
- ✅ Update transport layer (stdio, HTTP)
- ✅ Preserve all existing tool functionality

#### Phase 2: Client Migration
- ✅ Replace `botticelli_mcp_client` with pmcp's `Client`
- ✅ Migrate LLM adapters to pmcp patterns
- ✅ Update tool execution flow
- ✅ Preserve approval and retry logic

#### Phase 3: Enhanced Features
- ✅ Implement OAuth 2.0 authentication
- ✅ Add batch operation support
- ✅ Integrate middleware pipeline
- ✅ Enable WebSocket transport
- ✅ Expose health/metrics endpoints

#### Phase 4: Testing & Documentation
- ✅ Comprehensive test migration
- ✅ Update all documentation
- ✅ Performance benchmarks
- ✅ Migration guide for users

### Out of Scope
- ❌ Changing tool semantics or behavior
- ❌ Modifying narrative execution logic
- ❌ Altering database schemas
- ❌ Breaking changes to public APIs (where avoidable)

## Technical Analysis

### Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     botticelli_mcp                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  BotticelliRouter (Custom Router impl)                     │
│  │                                                          │
│  ├─ ToolRegistry (26 tools)                                │
│  │  ├─ Database tools (query_content)                      │
│  │  ├─ Narrative tools (create, validate, execute, etc.)   │
│  │  ├─ LLM tools (generate_*, sampling)                    │
│  │  ├─ Discord tools (channels, messages, posts)           │
│  │  └─ Utility tools (echo, metrics, server_info)          │
│  │                                                          │
│  ├─ ResourceRegistry (narratives, content)                 │
│  │                                                          │
│  └─ Transport Layer                                        │
│     ├─ ByteTransport (stdio)                               │
│     └─ HTTP (axum-based, optional)                         │
│                                                             │
│  Dependencies: mcp-server 0.1.0, mcp-spec 0.1.0            │
│                                                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  botticelli_mcp_client                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  McpClient                                                  │
│  │                                                          │
│  ├─ LlmAdapter trait                                        │
│  │  ├─ AnthropicAdapter                                    │
│  │  ├─ GeminiAdapter                                       │
│  │  ├─ GroqAdapter                                         │
│  │  └─ OllamaAdapter                                       │
│  │                                                          │
│  ├─ ToolExecutor                                            │
│  ├─ ApprovalManager                                         │
│  ├─ RetryConfig / CircuitBreaker                            │
│  ├─ ContextManager                                          │
│  └─ McpClientMetrics (prometheus)                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Target Architecture with PMCP

```
┌─────────────────────────────────────────────────────────────┐
│                     botticelli_mcp                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  pmcp::Server                                               │
│  │                                                          │
│  ├─ ServerCapabilities                                      │
│  │  ├─ Tools (via ToolHandler trait)                       │
│  │  ├─ Resources (via ResourceHandler trait)               │
│  │  └─ Prompts (via PromptHandler trait)                   │
│  │                                                          │
│  ├─ Middleware Pipeline                                     │
│  │  ├─ Authentication (OAuth 2.0, Bearer)                  │
│  │  ├─ Logging (structured)                                │
│  │  ├─ Batching (debouncing)                               │
│  │  ├─ Retry (exponential backoff)                         │
│  │  └─ Metrics (prometheus integration)                    │
│  │                                                          │
│  └─ Transport Layer (pmcp abstractions)                    │
│     ├─ StdioTransport                                       │
│     ├─ HttpTransport (SSE)                                  │
│     └─ WebSocketTransport                                   │
│                                                             │
│  Tool Handlers (26 total):                                 │
│  ├─ DatabaseToolHandler                                     │
│  ├─ NarrativeToolHandler                                    │
│  ├─ LlmToolHandler                                          │
│  ├─ DiscordToolHandler                                      │
│  └─ UtilityToolHandler                                      │
│                                                             │
│  Dependencies: pmcp (replaces mcp-server + mcp-spec)        │
│                                                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  botticelli_mcp_client                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  pmcp::Client                                               │
│  │                                                          │
│  ├─ ClientCapabilities                                      │
│  ├─ Transport (stdio/HTTP/WebSocket)                        │
│  │                                                          │
│  └─ Integration with existing:                             │
│     ├─ LlmAdapter trait (unchanged)                         │
│     ├─ ApprovalManager (enhanced with pmcp hooks)           │
│     ├─ RetryConfig (leverage pmcp middleware)               │
│     └─ McpClientMetrics (integrate with pmcp)               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Key Differences

| Component | Current | PMCP | Migration Effort |
|-----------|---------|------|------------------|
| Server abstraction | Custom `Router` trait | `pmcp::Server` builder | Medium |
| Tool registration | Custom `ToolRegistry` | `ToolHandler` trait + `Server::builder().tool()` | Medium |
| Resources | Custom `ResourceRegistry` | `ResourceHandler` trait | Medium |
| Transport | Manual stdio/HTTP | `StdioTransport`, `HttpTransport`, `WebSocketTransport` | Low |
| Authentication | None | Built-in OAuth 2.0 / Bearer | New feature |
| Middleware | Manual tracing | Extensible middleware pipeline | New feature |
| Batching | None | Built-in batch operations | New feature |
| Type system | `mcp-spec` types | `pmcp` types (compatible) | Low |
| Error handling | Custom `ToolError` | `pmcp::Error` | Medium |

## Dependency Analysis

### Current Dependencies

```toml
[dependencies]
mcp-server = "0.1.0"        # Basic server implementation
mcp-spec = "0.1.0"          # Protocol types
```

### Target Dependencies

```toml
[dependencies]
pmcp = "0.2"                # Replaces both mcp-server + mcp-spec
# Remove: mcp-server, mcp-spec
```

### Transitive Impact

**Crates affected:**
- `botticelli_mcp` (server) - **Major refactor**
- `botticelli_mcp_client` (client) - **Major refactor**
- `botticelli_chat` - Minor (uses client)
- `botticelli_tui` - Minor (uses client)
- `botticelli` - Minor (re-exports)

**External crates preserved:**
- All `botticelli_*` internal crates maintain compatibility
- LLM adapters unchanged
- Database layer unchanged
- Narrative engine unchanged

## Migration Phases

### Phase 1: Server Foundation (Week 1)

**Goal:** Replace server core with pmcp, preserve existing tools

#### Step 1.1: Add pmcp Dependency
**Task:** Update Cargo.toml  
**Effort:** 30 minutes

**Actions:**
```toml
# crates/botticelli_mcp/Cargo.toml
[dependencies]
pmcp = "0.2"
# mcp-server = "0.1.0"  # REMOVE
# mcp-spec = "0.1.0"    # REMOVE
```

**Success Criteria:**
- ✅ `cargo check -p botticelli_mcp` compiles with pmcp
- ✅ No version conflicts with other dependencies

**Rollback:** Revert Cargo.toml change

---

#### Step 1.2: Create Minimal Server
**Task:** Implement basic pmcp server  
**Effort:** 4 hours

**Actions:**
1. Create `src/pmcp_server.rs` (new file, don't touch existing)
2. Implement minimal server with echo tool
3. Verify stdio transport works

**Code Pattern:**
```rust
use pmcp::{Server, ServerCapabilities, ToolHandler};
use async_trait::async_trait;

struct EchoHandler;

#[async_trait]
impl ToolHandler for EchoHandler {
    async fn handle(&self, args: Value, _extra: pmcp::RequestHandlerExtra) 
        -> Result<Value, pmcp::Error> 
    {
        Ok(args) // Echo back
    }
}

pub async fn run_pmcp_server() -> anyhow::Result<()> {
    let server = Server::builder()
        .name("botticelli-pmcp")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(ServerCapabilities::default())
        .tool("echo", EchoHandler)
        .build()?;
    
    server.run_stdio().await?;
    Ok(())
}
```

**Success Criteria:**
- ✅ Server starts without errors
- ✅ Echo tool responds correctly via stdio
- ✅ Graceful shutdown on SIGTERM
- ✅ All instrumentation logs appear

**Testing:**
```bash
# Manual test
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"message":"test"}}}' | \
  cargo run -p botticelli_mcp --bin botticelli-mcp-pmcp

# Expected: JSON response with echoed message
```

**Rollback:** Delete `src/pmcp_server.rs`

---

#### Step 1.3: Migrate Tool Registry Pattern
**Task:** Convert ToolRegistry to pmcp ToolHandler pattern  
**Effort:** 8 hours

**Actions:**
1. Create `src/tools/handlers/` directory
2. For each existing tool (26 total):
   - Wrap in ToolHandler implementation
   - Preserve exact business logic
   - Add proper error conversion
3. Create `ToolHandlerRegistry` struct

**Code Pattern:**
```rust
// src/tools/handlers/echo.rs
use pmcp::{ToolHandler, RequestHandlerExtra, Error as PmcpError};
use async_trait::async_trait;
use crate::tools::EchoTool; // Existing implementation

pub struct EchoToolHandler {
    inner: EchoTool,
}

#[async_trait]
impl ToolHandler for EchoToolHandler {
    #[instrument(skip(self, extra))]
    async fn handle(
        &self, 
        args: Value, 
        extra: RequestHandlerExtra
    ) -> Result<Value, PmcpError> {
        // Convert args to our internal format
        let input = serde_json::from_value(args)
            .map_err(|e| PmcpError::invalid_params(e.to_string()))?;
        
        // Execute existing tool
        let result = self.inner.execute(input).await
            .map_err(|e| PmcpError::internal(e.to_string()))?;
        
        // Convert back to JSON
        serde_json::to_value(result)
            .map_err(|e| PmcpError::internal(e.to_string()))
    }
}
```

**Tools to migrate** (priority order):
1. ✅ `echo` (simplest, test pattern)
2. ✅ `server_info` (metadata)
3. ✅ `query_content` (database)
4. ✅ `validate_narrative` (narrative)
5. ✅ `create_narrative` (narrative)
6. ✅ `save_narrative` (narrative)
7. ✅ `execute_narrative` (narrative)
8. ✅ `modify_narrative` (narrative)
9. ✅ `elicit_metadata` (elicitation)
10. ✅ `elicit_act` (elicitation)
11. ✅ `finalize_narrative` (elicitation)
12. ✅ `start_narrative` (elicitation)
13. ✅ `generate_anthropic` (LLM)
14. ✅ `generate_gemini` (LLM)
15. ✅ `generate_ollama` (LLM)
16. ✅ `generate_groq` (LLM)
17. ✅ `generate_huggingface` (LLM)
18. ✅ `discord_get_channels` (social)
19. ✅ `discord_get_messages` (social)
20. ✅ `discord_get_guild_info` (social)
21. ✅ `discord_post_message` (social)
22. ✅ `export_metrics` (metrics)
23-26. ✅ Remaining utility tools

**Success Criteria:**
- ✅ All 26 tools wrapped as ToolHandlers
- ✅ Zero changes to business logic
- ✅ All input/output schemas preserved
- ✅ Error messages remain descriptive
- ✅ Instrumentation preserved

**Testing:**
```bash
# For each tool
just test-package botticelli_mcp
# Verify existing tests still pass (may need test updates)
```

**Rollback:** Git revert commits for this step

---

#### Step 1.4: Resource Migration
**Task:** Convert ResourceRegistry to pmcp ResourceHandler  
**Effort:** 4 hours

**Actions:**
1. Create `src/resources/handlers/` directory
2. Implement ResourceHandler for:
   - NarrativeResource
   - ContentResource (database feature)
3. Update resource templates

**Code Pattern:**
```rust
use pmcp::{ResourceHandler, Resource, ResourceContents};
use async_trait::async_trait;

pub struct NarrativeResourceHandler {
    inner: NarrativeResource,
}

#[async_trait]
impl ResourceHandler for NarrativeResourceHandler {
    async fn read(&self, uri: &str) -> Result<ResourceContents, pmcp::Error> {
        let narrative = self.inner.fetch_narrative(uri).await
            .map_err(|e| pmcp::Error::resource_not_found(uri))?;
        
        Ok(ResourceContents::Text {
            uri: uri.to_string(),
            mime_type: "application/toml".to_string(),
            text: narrative.to_toml()?,
        })
    }
    
    async fn list(&self) -> Result<Vec<Resource>, pmcp::Error> {
        // Implementation
    }
}
```

**Success Criteria:**
- ✅ All resources accessible via pmcp
- ✅ Resource URIs unchanged
- ✅ List operations work
- ✅ Read operations return correct content

**Testing:**
```bash
# Test resource listing
echo '{"jsonrpc":"2.0","id":1,"method":"resources/list"}' | \
  cargo run -p botticelli_mcp --bin botticelli-mcp-pmcp

# Test resource reading
echo '{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"narrative://example"}}' | \
  cargo run -p botticelli_mcp --bin botticelli-mcp-pmcp
```

**Rollback:** Git revert commits

---

#### Step 1.5: Server Integration
**Task:** Wire up complete server with all tools/resources  
**Effort:** 4 hours

**Actions:**
1. Update `src/pmcp_server.rs` to register all handlers
2. Configure capabilities
3. Add feature gates
4. Update binary entrypoints

**Code Pattern:**
```rust
pub async fn build_server() -> anyhow::Result<Server> {
    let mut builder = Server::builder()
        .name("botticelli")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(
            ServerCapabilities::builder()
                .tools(true)
                .resources(true)
                .build()
        );
    
    // Register all tools
    builder = builder
        .tool("echo", EchoToolHandler::new())
        .tool("server_info", ServerInfoToolHandler::new())
        .tool("query_content", QueryContentToolHandler::new());
    
    // Register resources
    builder = builder
        .resource_handler(NarrativeResourceHandler::new())
        .resource_handler(ContentResourceHandler::new());
    
    Ok(builder.build()?)
}
```

**Success Criteria:**
- ✅ Server lists all 26 tools
- ✅ Server lists all resources
- ✅ Capabilities correctly advertised
- ✅ Feature gates work (database, discord, llm)

**Testing:**
```bash
just check-package botticelli_mcp
just test-package botticelli_mcp

# Integration test
./scripts/test_mcp_server.sh
```

**Rollback:** Git revert commits

---

#### Step 1.6: Parallel Deployment
**Task:** Run old and new servers side-by-side  
**Effort:** 2 hours

**Actions:**
1. Keep old `botticelli-mcp` binary
2. Add new `botticelli-mcp-pmcp` binary
3. Update documentation with both options
4. Add deprecation notice to old binary

**Binary structure:**
```
[[bin]]
name = "botticelli-mcp"
path = "src/bin/botticelli-mcp.rs"
# DEPRECATED: Use botticelli-mcp-pmcp

[[bin]]
name = "botticelli-mcp-pmcp"
path = "src/bin/botticelli-mcp-pmcp.rs"
```

**Success Criteria:**
- ✅ Both binaries compile
- ✅ Both binaries functional
- ✅ Documentation updated
- ✅ Users can test new implementation

**Testing:**
```bash
# Test old binary
cargo run -p botticelli_mcp --bin botticelli-mcp

# Test new binary
cargo run -p botticelli_mcp --bin botticelli-mcp-pmcp

# Both should work identically
```

**Rollback:** Remove new binary, revert docs

---

**Phase 1 Completion Checklist:**
- [ ] pmcp dependency added
- [ ] Minimal server runs
- [ ] All 26 tools migrated to ToolHandler
- [ ] All resources migrated to ResourceHandler
- [ ] Server fully integrated
- [ ] Both binaries coexist
- [ ] Documentation updated
- [ ] All tests passing
- [ ] Performance benchmarks show improvement
- [ ] Zero regressions in functionality

**Estimated Total Effort:** 22.5 hours (3 days)

---

### Phase 2: Client Migration (Week 2)

**Goal:** Replace client with pmcp::Client, preserve LLM integration

#### Step 2.1: Add pmcp Client Dependency
**Task:** Update client Cargo.toml  
**Effort:** 30 minutes

**Actions:**
```toml
# crates/botticelli_mcp_client/Cargo.toml
[dependencies]
pmcp = "0.2"  # Add client capabilities
```

**Success Criteria:**
- ✅ `cargo check -p botticelli_mcp_client` compiles
- ✅ No conflicts with botticelli_mcp

**Rollback:** Revert Cargo.toml

---

#### Step 2.2: Create PMCP Client Wrapper
**Task:** Wrap pmcp::Client for our use case  
**Effort:** 6 hours

**Actions:**
1. Create `src/pmcp_client.rs`
2. Implement client initialization
3. Preserve LlmAdapter integration
4. Add tool discovery

**Code Pattern:**
```rust
use pmcp::{Client, StdioTransport, ClientCapabilities};

pub struct BotticelliMcpClient {
    inner: Client,
    llm_adapter: Box<dyn LlmAdapter>,
    approval_manager: ApprovalManager,
    metrics: McpClientMetrics,
}

impl BotticelliMcpClient {
    pub async fn new(
        llm_adapter: Box<dyn LlmAdapter>,
        approval_policy: ApprovalPolicy,
    ) -> McpClientResult<Self> {
        let transport = StdioTransport::new();
        let mut client = Client::new(transport);
        
        // Initialize connection
        let server_info = client
            .initialize(ClientCapabilities::default())
            .await
            .map_err(|e| McpClientError::new(
                McpClientErrorKind::ConnectionFailed(e.to_string())
            ))?;
        
        // Discover tools
        let tools = client.list_tools(None).await?;
        
        Ok(Self {
            inner: client,
            llm_adapter,
            approval_manager: ApprovalManager::new(approval_policy),
            metrics: McpClientMetrics::new(),
        })
    }
    
    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> McpClientResult<Value> {
        // Request approval
        self.approval_manager.request_approval(tool_name, &arguments).await?;
        
        // Execute via pmcp
        let result = self.inner.call_tool(tool_name, arguments).await
            .map_err(|e| McpClientError::new(
                McpClientErrorKind::ToolExecutionFailed(e.to_string())
            ))?;
        
        // Track metrics
        self.metrics.record_tool_call(tool_name);
        
        Ok(result)
    }
}
```

**Success Criteria:**
- ✅ Client connects to server
- ✅ Tool discovery works
- ✅ Tool execution works
- ✅ LlmAdapter integration preserved
- ✅ ApprovalManager integrated
- ✅ Metrics tracked

**Testing:**
```bash
# Unit tests
just test-package botticelli_mcp_client

# Integration test with server
cargo test --test mcp_integration
```

**Rollback:** Delete `src/pmcp_client.rs`

---

#### Step 2.3: LLM Adapter Integration
**Task:** Update LLM adapters to use pmcp client  
**Effort:** 4 hours

**Actions:**
1. Update `AnthropicAdapter` to use pmcp client
2. Update `GeminiAdapter` to use pmcp client
3. Update `GroqAdapter` to use pmcp client
4. Update `OllamaAdapter` to use pmcp client
5. Preserve all existing adapter logic

**Code Pattern:**
```rust
impl LlmAdapter for AnthropicAdapter {
    async fn generate_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> McpClientResult<GenerationResponse> {
        // Convert to Anthropic format (unchanged)
        let request = self.build_request(messages, tools);
        
        // Execute via Anthropic API (unchanged)
        let response = self.client.generate(request).await?;
        
        // Parse tool calls (unchanged)
        let tool_calls = self.extract_tool_calls(&response)?;
        
        Ok(GenerationResponse {
            content: response.content,
            tool_calls,
            finish_reason: FinishReason::ToolUse,
            token_usage: response.usage,
        })
    }
}
```

**Success Criteria:**
- ✅ All adapters work with pmcp client
- ✅ Tool call format unchanged
- ✅ All existing tests pass
- ✅ No regressions in LLM behavior

**Testing:**
```bash
just test-package botticelli_mcp_client

# Optional: API tests (rate-limited)
# just test-api
```

**Rollback:** Git revert commits

---

#### Step 2.4: Approval & Retry Logic
**Task:** Integrate ApprovalManager and RetryConfig with pmcp  
**Effort:** 4 hours

**Actions:**
1. Hook ApprovalManager into pmcp middleware
2. Leverage pmcp's built-in retry (may simplify our code)
3. Integrate CircuitBreaker with pmcp

**Code Pattern:**
```rust
// Approval middleware
struct ApprovalMiddleware {
    manager: Arc<ApprovalManager>,
}

impl pmcp::Middleware for ApprovalMiddleware {
    async fn handle_request(
        &self,
        req: pmcp::Request,
        next: pmcp::Next,
    ) -> Result<pmcp::Response, pmcp::Error> {
        // Extract tool info
        let tool_name = req.tool_name();
        let args = req.arguments();
        
        // Request approval
        self.manager.request_approval(tool_name, args).await
            .map_err(|e| pmcp::Error::forbidden(e.to_string()))?;
        
        // Continue chain
        next.run(req).await
    }
}
```

**Success Criteria:**
- ✅ ApprovalManager hooks work
- ✅ Retry logic functional
- ✅ CircuitBreaker triggers correctly
- ✅ All approval tests pass

**Testing:**
```bash
cargo test --test approval_test
cargo test --test retry_test
```

**Rollback:** Git revert commits

---

#### Step 2.5: Client Integration Tests
**Task:** End-to-end testing of pmcp client  
**Effort:** 4 hours

**Actions:**
1. Create comprehensive integration tests
2. Test all LLM adapters
3. Test approval workflows
4. Test error handling
5. Test metrics collection

**Test Pattern:**
```rust
#[tokio::test]
async fn test_anthropic_tool_execution() {
    let adapter = AnthropicAdapter::new(/* config */);
    let client = BotticelliMcpClient::new(
        Box::new(adapter),
        ApprovalPolicy::AutoApprove,
    ).await.unwrap();
    
    let result = client.execute_tool(
        "echo",
        json!({"message": "test"}),
    ).await.unwrap();
    
    assert_eq!(result["echo"], "test");
}
```

**Success Criteria:**
- ✅ All adapters pass integration tests
- ✅ Approval flows tested
- ✅ Error paths tested
- ✅ Metrics verified

**Testing:**
```bash
just test-package botticelli_mcp_client
cargo test --test integration -- --test-threads=1
```

**Rollback:** Git revert commits

---

**Phase 2 Completion Checklist:**
- [ ] pmcp client dependency added
- [ ] BotticelliMcpClient implemented
- [ ] All LLM adapters updated
- [ ] Approval/retry integrated
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] No regressions
- [ ] Performance benchmarks

**Estimated Total Effort:** 18.5 hours (2.5 days)

---

### Phase 3: Enhanced Features (Week 3)

**Goal:** Leverage pmcp's advanced capabilities

#### Step 3.1: OAuth 2.0 Authentication
**Task:** Add authentication to HTTP transport  
**Effort:** 6 hours

**Actions:**
1. Configure OAuth 2.0 provider
2. Add bearer token support
3. Update HTTP server
4. Add token refresh logic

**Code Pattern:**
```rust
use pmcp::auth::{OAuth2Config, BearerAuth};

let auth = OAuth2Config::builder()
    .client_id(env::var("OAUTH_CLIENT_ID")?)
    .client_secret(env::var("OAUTH_CLIENT_SECRET")?)
    .token_url("https://auth.example.com/token")
    .build();

let server = Server::builder()
    .name("botticelli")
    .auth(auth)
    .build()?;
```

**Success Criteria:**
- ✅ OAuth 2.0 flow works
- ✅ Bearer tokens validated
- ✅ Unauthorized requests rejected
- ✅ Token refresh automatic

**Testing:**
```bash
# Test with valid token
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/mcp

# Test without token (should fail)
curl http://localhost:8080/mcp
```

**Rollback:** Remove auth configuration

---

#### Step 3.2: Batch Operations
**Task:** Implement notification batching  
**Effort:** 4 hours

**Actions:**
1. Configure batch settings
2. Enable notification debouncing
3. Test batch efficiency

**Code Pattern:**
```rust
use pmcp::batch::{BatchConfig, BatchNotifier};

let batch_config = BatchConfig::builder()
    .max_batch_size(100)
    .debounce_ms(50)
    .build();

let server = Server::builder()
    .name("botticelli")
    .batch_config(batch_config)
    .build()?;
```

**Success Criteria:**
- ✅ Multiple notifications batched
- ✅ Debouncing reduces overhead
- ✅ Performance improved (benchmark)

**Testing:**
```bash
# Benchmark: send 1000 notifications
cargo bench --bench batch_notifications
```

**Rollback:** Remove batch configuration

---

#### Step 3.3: Middleware Pipeline
**Task:** Add custom middleware  
**Effort:** 6 hours

**Actions:**
1. Create logging middleware
2. Create metrics middleware
3. Create rate limiting middleware
4. Wire up pipeline

**Code Pattern:**
```rust
struct LoggingMiddleware;

impl pmcp::Middleware for LoggingMiddleware {
    async fn handle_request(
        &self,
        req: pmcp::Request,
        next: pmcp::Next,
    ) -> Result<pmcp::Response, pmcp::Error> {
        let start = Instant::now();
        info!("Request: {} {}", req.method(), req.id());
        
        let result = next.run(req).await;
        
        let duration = start.elapsed();
        info!("Response: {} ({:?})", req.id(), duration);
        
        result
    }
}

let server = Server::builder()
    .middleware(LoggingMiddleware)
    .middleware(MetricsMiddleware::new())
    .middleware(RateLimitMiddleware::new())
    .build()?;
```

**Success Criteria:**
- ✅ All requests logged
- ✅ Metrics collected
- ✅ Rate limiting works
- ✅ Middleware order respected

**Testing:**
```bash
# Verify logging
RUST_LOG=info cargo run -p botticelli_mcp --bin botticelli-mcp-pmcp

# Verify rate limiting
./scripts/test_rate_limit.sh
```

**Rollback:** Remove middleware

---

#### Step 3.4: WebSocket Transport
**Task:** Add WebSocket support  
**Effort:** 6 hours

**Actions:**
1. Implement WebSocket transport
2. Add connection management
3. Test bidirectional communication
4. Update documentation

**Code Pattern:**
```rust
use pmcp::transport::WebSocketTransport;

let server = Server::builder()
    .name("botticelli")
    .build()?;

server.run_websocket("0.0.0.0:9090").await?;
```

**Success Criteria:**
- ✅ WebSocket connections accepted
- ✅ Bidirectional messaging works
- ✅ Reconnection handled
- ✅ Performance acceptable

**Testing:**
```bash
# Test WebSocket connection
websocat ws://localhost:9090

# Send test message
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

**Rollback:** Remove WebSocket code

---

#### Step 3.5: Health & Metrics Endpoints
**Task:** Expose observability endpoints  
**Effort:** 4 hours

**Actions:**
1. Add `/health` endpoint
2. Add `/metrics` endpoint (Prometheus)
3. Add `/ready` endpoint
4. Integrate with Grafana

**Code Pattern:**
```rust
use pmcp::observability::{HealthCheck, MetricsExporter};

let server = Server::builder()
    .name("botticelli")
    .health_check(HealthCheck::default())
    .metrics_exporter(MetricsExporter::prometheus())
    .build()?;
```

**Success Criteria:**
- ✅ `/health` returns 200
- ✅ `/metrics` exports Prometheus format
- ✅ `/ready` reflects server state
- ✅ Grafana dashboard updated

**Testing:**
```bash
curl http://localhost:8080/health
curl http://localhost:8080/metrics
curl http://localhost:8080/ready
```

**Rollback:** Remove endpoints

---

**Phase 3 Completion Checklist:**
- [ ] OAuth 2.0 authentication working
- [ ] Batch operations enabled
- [ ] Middleware pipeline functional
- [ ] WebSocket transport available
- [ ] Health/metrics endpoints live
- [ ] Documentation updated
- [ ] Performance benchmarks show gains
- [ ] Integration tests passing

**Estimated Total Effort:** 26 hours (3.5 days)

---

### Phase 4: Testing & Documentation (Week 4)

**Goal:** Comprehensive testing, benchmarking, and documentation

#### Step 4.1: Test Migration
**Task:** Update all tests for pmcp  
**Effort:** 8 hours

**Actions:**
1. Update unit tests
2. Update integration tests
3. Add pmcp-specific tests
4. Ensure 100% pass rate

**Coverage targets:**
- Server: 90%+ coverage
- Client: 90%+ coverage
- Tools: 85%+ coverage
- Resources: 85%+ coverage

**Success Criteria:**
- ✅ All tests pass
- ✅ Coverage maintained or improved
- ✅ No flaky tests
- ✅ CI/CD green

**Testing:**
```bash
just test-all
just check-features
cargo tarpaulin --workspace --out Html
```

**Rollback:** N/A (fix tests)

---

#### Step 4.2: Performance Benchmarking
**Task:** Quantify pmcp performance gains  
**Effort:** 6 hours

**Actions:**
1. Create benchmark suite
2. Compare old vs new implementation
3. Measure memory usage
4. Document findings

**Benchmarks:**
- Tool call latency
- Throughput (calls/sec)
- Memory usage
- Batch efficiency
- Zero-copy parsing

**Success Criteria:**
- ✅ Latency reduced by ≥10%
- ✅ Throughput increased by ≥5x
- ✅ Memory usage reduced by ≥30%
- ✅ Documented results

**Testing:**
```bash
cargo bench --bench mcp_performance
cargo bench --bench tool_throughput
cargo bench --bench memory_usage
```

**Rollback:** N/A (documentation)

---

#### Step 4.3: Documentation Updates
**Task:** Update all documentation  
**Effort:** 8 hours

**Actions:**
1. Update `MCP.md`
2. Update `README.md`
3. Update `CLAUDE.md` (if needed)
4. Create `PMCP_MIGRATION_GUIDE.md`
5. Update inline documentation
6. Update examples

**Documents to update:**
- `/MCP.md` - Architecture and usage
- `/README.md` - Quick start
- `/crates/botticelli_mcp/README.md` - Server docs
- `/crates/botticelli_mcp_client/README.md` - Client docs
- `/claude_desktop_config.example.json` - Configuration
- `/examples/*` - Code examples

**Success Criteria:**
- ✅ All docs accurate
- ✅ Migration guide complete
- ✅ Examples working
- ✅ Markdown linting passes

**Testing:**
```bash
markdownlint-cli2 "**/*.md"
# Manually verify examples
```

**Rollback:** Git revert docs

---

#### Step 4.4: Migration Guide for Users
**Task:** Create user-facing migration guide  
**Effort:** 4 hours

**Actions:**
1. Document breaking changes
2. Provide migration examples
3. List new features
4. Add troubleshooting section

**Guide structure:**
```markdown
# PMCP Migration Guide for Users

## Breaking Changes
- Old binary: `botticelli-mcp` → New: `botticelli-mcp-pmcp`
- Configuration: ...
- API changes: ...

## Migration Steps
1. Update Cargo.toml dependencies
2. Update binary path in claude_desktop_config.json
3. Restart Claude Desktop
4. Verify tools work

## New Features
- OAuth 2.0 authentication
- WebSocket transport
- Batch operations
- Enhanced observability

## Troubleshooting
...
```

**Success Criteria:**
- ✅ Guide complete
- ✅ All scenarios covered
- ✅ Examples tested
- ✅ Reviewed by team

**Rollback:** N/A (documentation)

---

#### Step 4.5: Deprecation & Cleanup
**Task:** Remove old implementation  
**Effort:** 4 hours

**Actions:**
1. Add deprecation warnings to old binary
2. Schedule removal date (e.g., 2 releases)
3. Update CHANGELOG.md
4. Create GitHub issue for tracking

**Deprecation notice:**
```rust
// src/bin/botticelli-mcp.rs
#[deprecated(
    since = "0.3.0",
    note = "Use botticelli-mcp-pmcp binary instead. \
            This binary will be removed in 0.5.0."
)]
fn main() {
    eprintln!(
        "WARNING: botticelli-mcp is deprecated. \
         Use botticelli-mcp-pmcp instead."
    );
    // ... existing code
}
```

**Success Criteria:**
- ✅ Deprecation warnings added
- ✅ Removal timeline communicated
- ✅ CHANGELOG updated
- ✅ Users notified

**Testing:**
```bash
# Verify warning appears
cargo run -p botticelli_mcp --bin botticelli-mcp
```

**Rollback:** Remove deprecation notices

---

**Phase 4 Completion Checklist:**
- [ ] All tests migrated and passing
- [ ] Performance benchmarks complete
- [ ] All documentation updated
- [ ] Migration guide published
- [ ] Deprecation plan in place
- [ ] CI/CD passing
- [ ] Code coverage maintained
- [ ] Markdown linting passes

**Estimated Total Effort:** 30 hours (4 days)

---

## Risk Analysis

### High Risk

#### Risk 1: Breaking Changes in Tool Signatures
**Probability:** Medium  
**Impact:** High  
**Mitigation:**
- Preserve old binary during migration
- Extensive integration testing
- User testing period (2 weeks)
- Rollback plan documented

#### Risk 2: Performance Regression
**Probability:** Low  
**Impact:** High  
**Mitigation:**
- Comprehensive benchmarking before/after
- Performance tests in CI/CD
- Monitoring in production
- Rollback on degradation >10%

#### Risk 3: Client-Server Compatibility Issues
**Probability:** Medium  
**Impact:** High  
**Mitigation:**
- PMCP is TypeScript SDK compatible
- Version negotiation in protocol
- Extensive compatibility testing
- Gradual rollout

### Medium Risk

#### Risk 4: Authentication Complexity
**Probability:** Medium  
**Impact:** Medium  
**Mitigation:**
- OAuth 2.0 is optional feature
- Clear documentation
- Example configurations
- Fallback to no-auth mode

#### Risk 5: Learning Curve for Team
**Probability:** Medium  
**Impact:** Medium  
**Mitigation:**
- PMCP guide (27 chapters) available
- Internal training session
- Code review focus
- Pair programming

### Low Risk

#### Risk 6: Dependency Lock-in
**Probability:** Low  
**Impact:** Medium  
**Mitigation:**
- PMCP is open source (MIT/Apache)
- Active maintenance by Pragmatic AI Labs
- Standard MCP protocol underneath
- Can fork if needed

## Success Metrics

### Performance
- ✅ Tool call latency reduced by ≥10%
- ✅ Throughput increased by ≥5x (target: 16x per pmcp claims)
- ✅ Memory usage reduced by ≥30% (target: 50x per pmcp claims)
- ✅ P99 latency <100ms for simple tools

### Reliability
- ✅ Zero critical bugs in production (first month)
- ✅ Uptime ≥99.9% (excluding planned maintenance)
- ✅ Circuit breaker triggers <1% of requests
- ✅ Error rate <0.1%

### Code Quality
- ✅ Lines of code reduced by ≥60% (10.5k → ~4k)
- ✅ Test coverage maintained ≥85%
- ✅ Zero clippy warnings
- ✅ All documentation current

### User Experience
- ✅ Migration guide rated ≥4/5 by users
- ✅ Zero user-reported regressions (first 2 weeks)
- ✅ New features adopted by ≥50% of users (first month)
- ✅ Support tickets reduced by 20%

## Timeline

| Phase | Duration | Start | End | Owner |
|-------|----------|-------|-----|-------|
| Phase 1: Server Foundation | 3 days | Week 1 Mon | Week 1 Wed | TBD |
| Phase 2: Client Migration | 2.5 days | Week 2 Mon | Week 2 Wed | TBD |
| Phase 3: Enhanced Features | 3.5 days | Week 2 Thu | Week 3 Tue | TBD |
| Phase 4: Testing & Docs | 4 days | Week 3 Wed | Week 4 Mon | TBD |
| **Total** | **13 days** | **Week 1** | **Week 4** | |

**Buffer:** 2 days for unexpected issues

## Rollback Strategy

### Pre-Migration
1. Tag current working version: `git tag pre-pmcp-migration`
2. Create backup branch: `git checkout -b backup/pre-pmcp`
3. Document current performance baselines
4. Snapshot all tests passing

### During Migration
1. Keep old binaries functional during transition
2. Feature flag new implementation: `--features=pmcp`
3. Run both implementations in parallel
4. Monitor metrics for both

### Rollback Triggers
- Any critical bug in production
- Performance degradation >10%
- User-reported regressions >5
- Test pass rate <95%

### Rollback Procedure
```bash
# Stop new binary
systemctl stop botticelli-mcp-pmcp

# Revert to old binary
git checkout pre-pmcp-migration

# Rebuild
just build

# Restart old binary
systemctl start botticelli-mcp

# Notify users
./scripts/notify_rollback.sh
```

**Recovery Time Objective (RTO):** <1 hour  
**Recovery Point Objective (RPO):** Zero data loss (stateless server)

## Post-Migration

### Monitoring (First Month)
- Daily review of error logs
- Weekly performance reports
- Bi-weekly user feedback sessions
- Monthly retrospective

### Optimization Opportunities
1. Fine-tune batch settings
2. Optimize middleware pipeline
3. Enable zero-copy parsing for large payloads
4. Implement additional transports (gRPC?)

### Future Enhancements
1. Plugin system using pmcp extensions
2. Advanced authentication (mTLS, JWT)
3. Rate limiting per-user/per-tool
4. Request tracing with OpenTelemetry
5. Distributed tracing across services

## Open Questions

1. **Q:** Do we need backward compatibility with existing MCP clients?  
   **A:** TBD - depends on deployment environment

2. **Q:** Should we support multiple authentication methods simultaneously?  
   **A:** TBD - start with OAuth 2.0, add others if needed

3. **Q:** What's the deprecation timeline for old implementation?  
   **A:** Proposal: 2 releases (0.3.0 deprecate, 0.5.0 remove)

4. **Q:** Do we need pmcp's CLI tooling (`cargo-pmcp`)?  
   **A:** TBD - evaluate after server migration

5. **Q:** Should we contribute back to pmcp project?  
   **A:** Yes - any bug fixes or improvements

## References

- [PMCP Documentation](https://docs.rs/pmcp)
- [PMCP Guide](https://paiml.github.io/rust-mcp-sdk/)
- [PMCP GitHub](https://github.com/paiml/rust-mcp-sdk)
- [Model Context Protocol Spec](https://modelcontextprotocol.io)
- [Current MCP Implementation](./MCP.md)
- [Current MCP Client Design](./MCP_CLIENT_DESIGN.md)

## Appendices

### Appendix A: Tool Migration Checklist

| Tool Name | Priority | Handler Implemented | Tests Passing | Docs Updated |
|-----------|----------|---------------------|---------------|--------------|
| echo | P0 | ☐ | ☐ | ☐ |
| server_info | P0 | ☐ | ☐ | ☐ |
| query_content | P1 | ☐ | ☐ | ☐ |
| validate_narrative | P1 | ☐ | ☐ | ☐ |
| create_narrative | P1 | ☐ | ☐ | ☐ |
| save_narrative | P1 | ☐ | ☐ | ☐ |
| execute_narrative | P1 | ☐ | ☐ | ☐ |
| modify_narrative | P2 | ☐ | ☐ | ☐ |
| elicit_metadata | P2 | ☐ | ☐ | ☐ |
| elicit_act | P2 | ☐ | ☐ | ☐ |
| finalize_narrative | P2 | ☐ | ☐ | ☐ |
| start_narrative | P2 | ☐ | ☐ | ☐ |
| generate_anthropic | P1 | ☐ | ☐ | ☐ |
| generate_gemini | P1 | ☐ | ☐ | ☐ |
| generate_ollama | P2 | ☐ | ☐ | ☐ |
| generate_groq | P2 | ☐ | ☐ | ☐ |
| generate_huggingface | P2 | ☐ | ☐ | ☐ |
| discord_get_channels | P2 | ☐ | ☐ | ☐ |
| discord_get_messages | P2 | ☐ | ☐ | ☐ |
| discord_get_guild_info | P2 | ☐ | ☐ | ☐ |
| discord_post_message | P2 | ☐ | ☐ | ☐ |
| export_metrics | P3 | ☐ | ☐ | ☐ |

**Priority:**
- P0: Critical (basic functionality)
- P1: High (core features)
- P2: Medium (important features)
- P3: Low (nice-to-have)

### Appendix B: Performance Baseline

| Metric | Current (mcp-server) | Target (pmcp) | Improvement |
|--------|----------------------|---------------|-------------|
| Tool call latency (p50) | TBD ms | TBD ms | TBD% |
| Tool call latency (p99) | TBD ms | <100 ms | TBD% |
| Throughput (calls/sec) | TBD | TBD | 5-16x |
| Memory usage (idle) | TBD MB | TBD MB | 30-50x |
| Memory usage (peak) | TBD MB | TBD MB | 30-50x |

*TBD: To be measured during Phase 4.2*

### Appendix C: Dependency Tree

```
botticelli_mcp (server)
└── pmcp = "0.2"           # Replaces mcp-server + mcp-spec
    ├── tokio
    ├── serde
    ├── serde_json
    └── ... (pmcp's dependencies)

botticelli_mcp_client (client)
└── pmcp = "0.2"           # Replaces custom client
    └── ... (same as above)

# Removed dependencies
# mcp-server = "0.1.0"
# mcp-spec = "0.1.0"
```

### Appendix D: Code Reduction Analysis

| Component | Before (LOC) | After (LOC) | Reduction |
|-----------|--------------|-------------|-----------|
| Server router | ~500 | ~100 | 80% |
| Tool registry | ~800 | ~300 | 62% |
| Resource registry | ~400 | ~150 | 62% |
| Transport layer | ~600 | ~50 | 91% |
| Client implementation | ~1000 | ~400 | 60% |
| Error handling | ~300 | ~100 | 67% |
| **Total** | **~10,500** | **~4,000** | **~62%** |

*Note: Business logic (tool implementations) mostly unchanged*

---

**Document Version:** 1.0  
**Last Updated:** 2025-12-15  
**Next Review:** After Phase 1 completion
