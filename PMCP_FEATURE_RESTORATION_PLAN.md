# PMCP Feature Restoration Plan

## Problem Statement

During refactoring to integrate pmcp, critical features were **deleted** instead of **adapted**:

1. **Approval system** - Human-in-the-loop for tool execution
2. **Retry/resilience logic** - Fault tolerance for tool calls  
3. **Metrics/observability** - Integration with our tracing infrastructure
4. **Agentic loop** - `McpClient.execute()` with iteration limits
5. **Tool execution** - `ToolExecutor` for internal tools

**What went wrong**: Saw pmcp's `Client` and thought "we'll just use theirs" - ignoring that pmcp provides **transport/protocol**, not **our business logic**.

## What pmcp Provides

- `pmcp::Client` - External server connection (stdio/SSE transport)
- `pmcp::Server` - MCP server implementation
- Protocol handling (JSON-RPC, types, schemas)

## What pmcp Does NOT Provide

- **Approval workflows** - Botticelli-specific safety
- **Retry logic** - Our error handling strategy
- **Metrics** - Integration with our Prometheus/Jaeger stack
- **Internal tool execution** - In-process function calls (no transport needed)
- **Agentic orchestration** - Multi-turn LLM+tool loops

## Restoration Strategy

### Phase 1: Restore Core Orchestration (Priority 1)

**Goal**: Bring back `McpClient::execute()` agentic loop with pmcp integration

**Files to restore/adapt**:
- `src/client.rs` - Agentic loop coordinator
- `src/tool_executor.rs` - Internal tool execution

**Key changes**:
```rust
// OLD (deleted):
impl McpClient {
    async fn execute<B: LlmBackend>(
        &self, 
        backend: &B, 
        messages: Vec<Message>
    ) -> McpClientResult<String> {
        // Agentic loop with max_iterations
    }
}

// NEW (restore with pmcp):
impl McpClient {
    async fn execute<B: LlmBackend>(
        &self,
        backend: &B,
        messages: Vec<Message>
    ) -> McpClientResult<String> {
        // Same agentic loop logic
        // But tool execution uses:
        // - Internal tools -> direct function calls
        // - External tools -> pmcp::Client
    }
}
```

**Steps**:
1. Extract `McpClient` from git history (commit `a66e98c^`)
2. Update imports to use pmcp types where appropriate
3. Keep agentic loop logic intact
4. Add routing: internal vs external tool execution
5. Preserve max_iterations, conversation state

**Success criteria**:
- ✅ `McpClient::execute()` compiles
- ✅ Agentic loop runs with iteration limit
- ✅ Can call both internal and external tools
- ✅ All existing tests pass

---

### Phase 2: Restore Approval System (Priority 1)

**Goal**: Human-in-the-loop tool execution safety

**Files to restore**:
- Check git history for approval-related code
- May have been in `tool_executor.rs` or separate module

**Strategy**:
```rust
pub struct ApprovalConfig {
    /// Require approval for all tools
    require_approval: bool,
    /// Tools that always require approval
    always_approve: HashSet<String>,
    /// Tools that never require approval  
    never_approve: HashSet<String>,
}

impl ToolExecutor {
    async fn execute_with_approval(
        &self,
        tool_name: &str,
        arguments: Value,
        approval_config: &ApprovalConfig,
    ) -> McpClientResult<Value> {
        if approval_config.needs_approval(tool_name) {
            // Prompt human via terminal/UI
            let approved = self.request_approval(tool_name, &arguments).await?;
            if !approved {
                return Err(McpClientError::new(
                    McpClientErrorKind::ApprovalDenied(tool_name.to_string())
                ));
            }
        }
        
        self.execute_tool(tool_name, arguments).await
    }
}
```

**Integration points**:
- Terminal UI (botticelli_chat)
- TUI (botticelli_tui) 
- Discord (botticelli_discord)

**Steps**:
1. Search git history for approval code
2. Extract approval logic
3. Create `ApprovalConfig` builder
4. Add approval middleware to tool execution
5. Integrate with existing UIs

**Success criteria**:
- ✅ Can require approval per tool
- ✅ Can approve/deny from terminal
- ✅ Can configure approval rules
- ✅ Metrics track approval rate

---

### Phase 3: Restore Retry/Resilience (Priority 2)

**Goal**: Fault-tolerant tool execution

**Strategy**:
```rust
pub struct RetryConfig {
    /// Maximum retry attempts
    max_retries: usize,
    /// Backoff strategy
    backoff: BackoffStrategy,
    /// Which errors are retryable
    retryable_errors: HashSet<String>,
}

impl ToolExecutor {
    async fn execute_with_retry(
        &self,
        tool_name: &str,
        arguments: Value,
        retry_config: &RetryConfig,
    ) -> McpClientResult<Value> {
        let mut attempts = 0;
        
        loop {
            match self.execute_tool(tool_name, arguments.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) if attempts < retry_config.max_retries 
                         && retry_config.is_retryable(&e) => {
                    attempts += 1;
                    let delay = retry_config.backoff.next_delay(attempts);
                    warn!(
                        tool = tool_name,
                        attempt = attempts,
                        delay_ms = delay.as_millis(),
                        error = ?e,
                        "Tool execution failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

**Steps**:
1. Search git history for retry logic
2. Define `RetryConfig` and `BackoffStrategy`
3. Implement retry wrapper around tool execution
4. Add retry metrics (attempt count, success rate)
5. Configure per-tool retry policies

**Success criteria**:
- ✅ Configurable retry attempts
- ✅ Exponential backoff
- ✅ Per-tool retry policies
- ✅ Metrics track retry attempts/successes

---

### Phase 4: Restore Observability (Priority 1)

**Goal**: Full tracing/metrics for MCP operations

**Current problem**: All the observability code was deleted:
```rust
// DELETED - need to restore:
struct McpClientMetrics {
    tool_executions: Counter,
    tool_duration: Histogram,
    approval_rate: Gauge,
    retry_attempts: Counter,
}
```

**Strategy**:
```rust
use opentelemetry::metrics::{Counter, Histogram, Meter};
use tracing::{instrument, info, debug, warn, error};

pub struct McpMetrics {
    tool_executions_total: Counter<u64>,
    tool_duration_seconds: Histogram<f64>,
    tool_errors_total: Counter<u64>,
    approval_requests_total: Counter<u64>,
    approval_granted_total: Counter<u64>,
    retry_attempts_total: Counter<u64>,
}

impl McpMetrics {
    pub fn new(meter: &Meter) -> Self {
        Self {
            tool_executions_total: meter
                .u64_counter("mcp.tool.executions.total")
                .with_description("Total tool executions")
                .init(),
            tool_duration_seconds: meter
                .f64_histogram("mcp.tool.duration.seconds")
                .with_description("Tool execution duration")
                .init(),
            // ... etc
        }
    }
    
    #[instrument(skip(self))]
    pub fn record_tool_execution(&self, tool_name: &str, duration: Duration, success: bool) {
        self.tool_executions_total.add(
            1,
            &[KeyValue::new("tool", tool_name.to_string()),
              KeyValue::new("success", success)]
        );
        self.tool_duration_seconds.record(
            duration.as_secs_f64(),
            &[KeyValue::new("tool", tool_name.to_string())]
        );
    }
}
```

**Tracing spans**:
```rust
#[instrument(skip(self, backend), fields(iteration, tool_count))]
async fn execute<B: LlmBackend>(&self, backend: &B, messages: Vec<Message>) {
    // All key operations logged
}

#[instrument(skip(self, arguments), fields(tool = %tool_name))]
async fn execute_tool(&self, tool_name: &str, arguments: Value) {
    // Tool execution traced
}
```

**Steps**:
1. Search git history for metrics code
2. Define `McpMetrics` with OpenTelemetry
3. Add metrics to all tool operations
4. Add tracing spans to all async methods
5. Create Grafana dashboard for MCP metrics

**Success criteria**:
- ✅ All tool executions logged with spans
- ✅ Metrics exported to Prometheus
- ✅ Grafana dashboard shows MCP health
- ✅ Error rates tracked per tool
- ✅ Approval/retry metrics visible

---

### Phase 5: Internal Tool Registry (Priority 1)

**Goal**: In-process tool execution without transport overhead

**Current state**: We have `NarrativeTool` trait but no registry or execution

**Strategy**:
```rust
// Already exists - good!
pub trait NarrativeTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error>>;
}

// Need to add:
pub struct InternalToolRegistry {
    tools: HashMap<String, Arc<dyn NarrativeTool>>,
    metrics: Arc<McpMetrics>,
}

impl InternalToolRegistry {
    pub fn new(metrics: Arc<McpMetrics>) -> Self {
        Self {
            tools: HashMap::new(),
            metrics,
        }
    }
    
    pub fn register(&mut self, tool: Arc<dyn NarrativeTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    #[instrument(skip(self, args))]
    pub async fn execute(
        &self,
        tool_name: &str,
        args: Value,
    ) -> McpClientResult<Value> {
        let start = Instant::now();
        
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| McpClientError::new(
                McpClientErrorKind::ToolNotFound(tool_name.to_string())
            ))?;
        
        match tool.execute(args).await {
            Ok(result) => {
                self.metrics.record_tool_execution(
                    tool_name,
                    start.elapsed(),
                    true
                );
                Ok(result)
            }
            Err(e) => {
                self.metrics.record_tool_execution(
                    tool_name,
                    start.elapsed(),
                    false
                );
                Err(McpClientError::new(
                    McpClientErrorKind::ToolExecutionFailed(e.to_string())
                ))
            }
        }
    }
}
```

**Steps**:
1. Create `InternalToolRegistry`
2. Integrate with existing `NarrativeTool` implementations
3. Add registry to `UnifiedMcpClient`
4. Route internal tool calls to registry (no pmcp needed)
5. Add metrics to internal tool execution

**Success criteria**:
- ✅ Can register internal tools
- ✅ Can execute internal tools without transport
- ✅ Internal tools have metrics
- ✅ Internal tools support approval/retry

---

### Phase 6: Unified Client Complete (Priority 2)

**Goal**: Single client that handles both internal and external tools seamlessly

**Architecture**:
```rust
pub struct UnifiedMcpClient {
    // Internal tools (no transport)
    internal_registry: Arc<InternalToolRegistry>,
    
    // External tools (via pmcp)
    external_client: Arc<pmcp::Client>,
    
    // Orchestration
    agentic_client: Arc<McpClient>,
    
    // Cross-cutting concerns
    metrics: Arc<McpMetrics>,
    approval_config: ApprovalConfig,
    retry_config: RetryConfig,
}

impl UnifiedMcpClient {
    #[instrument(skip(self, args))]
    pub async fn call_tool(
        &self,
        tool_name: &str,
        args: Value,
    ) -> McpClientResult<Value> {
        // Check if internal tool
        if self.internal_registry.has_tool(tool_name) {
            // Direct execution
            let result = self.internal_registry.execute(tool_name, args).await?;
            return Ok(result);
        }
        
        // Must be external tool - use pmcp
        let result = self.external_client
            .call_tool(tool_name, args)
            .await
            .map_err(|e| McpClientError::new(
                McpClientErrorKind::ExternalToolError(e.to_string())
            ))?;
        
        Ok(result)
    }
    
    #[instrument(skip(self, backend))]
    pub async fn execute<B: LlmBackend>(
        &self,
        backend: &B,
        messages: Vec<Message>,
    ) -> McpClientResult<String> {
        // Delegate to agentic client
        self.agentic_client.execute(backend, messages).await
    }
}
```

**Steps**:
1. Combine all previous phases into `UnifiedMcpClient`
2. Add tool routing logic (internal vs external)
3. Apply approval/retry/metrics to both paths
4. Update all consumers to use unified client

**Success criteria**:
- ✅ Single API for all tools
- ✅ Transparent routing
- ✅ Approval/retry work for both internal and external
- ✅ Full observability for both paths
- ✅ All tests pass

---

## Implementation Order

1. **Phase 1** (Core Orchestration) - Restore agentic loop
2. **Phase 5** (Internal Registry) - Get internal tools working  
3. **Phase 4** (Observability) - Add metrics/tracing
4. **Phase 2** (Approval) - Add human-in-the-loop
5. **Phase 3** (Retry) - Add fault tolerance
6. **Phase 6** (Unified Client) - Tie it all together

## Git History Reference

Key commits to reference:
- `a66e98c^` - Last version with `McpClient`
- `7d85df2` - Original `ToolExecutor` implementation
- Search for "approval", "retry", "metrics" in pre-refactor commits

## Testing Strategy

Each phase includes:
- Unit tests for new code
- Integration tests with existing tools
- Manual testing with real LLM
- Metrics validation in Grafana

## Success Criteria (Overall)

- ✅ All deleted features restored and adapted
- ✅ Internal tools execute without transport
- ✅ External tools use pmcp correctly
- ✅ Full observability (traces + metrics)
- ✅ Approval system functional
- ✅ Retry logic operational
- ✅ Zero compilation errors/warnings
- ✅ All tests pass
- ✅ Documentation updated
- ✅ Botticelli is truly "self-driving"

## What This Is NOT

This is **NOT** about:
- Adding new features
- Redesigning the architecture
- "Improving" what worked before

This **IS** about:
- Restoring features that were deleted
- Adapting them to work with pmcp
- Preserving battle-tested logic
- Making minimal, surgical changes

---

**Next Step**: Begin Phase 1 - restore `McpClient::execute()` from git history and adapt for pmcp integration.
