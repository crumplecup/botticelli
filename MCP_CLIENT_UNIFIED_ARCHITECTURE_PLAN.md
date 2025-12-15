# MCP Client Unified Architecture Plan (Option 3)

## Executive Summary

**Goal:** Eliminate architectural duplication by refactoring `botticelli_mcp_client` to use existing proven type systems and implementations.

**Why Now:** Early development is the right time to fix architectural issues. The mcp_client crate duplicates ~1400 LOC that exists elsewhere, creating maintenance burden and type fragmentation.

**Breaking Changes:** Yes, but minimal external impact (only 2 files import from this crate).

**Time Estimate:** 4-5 days

**Result:** Single unified architecture, no type translation, maintainable long-term.

---

## Problem Statement

### Current Duplication

The `botticelli_mcp_client` crate (1,407 LOC) has:

1. **Duplicate Types:**
   - `llm_adapter::Message` vs `botticelli_core::Message`
   - `llm_adapter::ToolCall` vs `botticelli_core::ToolCall`
   - `llm_adapter::MessageRole` vs `botticelli_core::Role`
   - `tool_executor::ToolDefinition` vs `botticelli_mcp::ToolDefinition`

2. **Duplicate Traits:**
   - `llm_adapter::LlmAdapter` vs `botticelli_core::LlmProvider`
   - Purpose: Same (generate with tools)
   - Interface: Nearly identical
   - Implementations: All stubbed in mcp_client

3. **Duplicate Functionality:**
   - `tool_executor::ToolExecutor` vs `botticelli_mcp::ToolRegistry`
   - `context::ContextManager` vs `botticelli_mcp::ConversationSession`
   - `client::McpClient` vs `botticelli_mcp::SamplingCoordinator`

4. **Duplicate Schemas:**
   - `schema/*` modules duplicate provider-specific schemas
   - Already exist in `botticelli_models` crates

### Impact

**Maintenance:**
- API changes require updates in 2 places
- Bug fixes need duplicate implementations
- Tests need duplicate coverage

**Cognitive Load:**
- Two ways to do everything
- Type conversion at boundaries
- Unclear which to use when

**Binary Size:**
- Duplicate code compiled in
- Larger deployment artifacts

**Type Safety:**
- Conversions between similar types
- Runtime errors possible
- Loss of static guarantees

---

## Current Usage Analysis

### Who Uses `botticelli_mcp_client`?

```bash
$ grep -r "use botticelli_mcp_client" crates --include="*.rs"
crates/botticelli/src/cli/mcp.rs:use botticelli_mcp_client::McpClient;
crates/botticelli_mcp_client/tests/external_client_test.rs:use botticelli_mcp_client::{...};
```

**Only 2 locations:**
1. `cli/mcp.rs` - CLI command (easily updatable)
2. `tests/external_client_test.rs` - Internal test (easily updatable)

**Conclusion: Refactor is safe - minimal external impact.**

---

## Target Architecture

### After Refactor

The `botticelli_mcp_client` crate becomes:

**Purpose:** External MCP server connectivity (only what's unique)

**Contents:**
1. ✅ **external_client.rs** - Connect to external MCP servers (KEEP)
2. ✅ **approval.rs** - User approval for external tools (KEEP)
3. ✅ **retry.rs** - Retry logic with backoff (KEEP)
4. ✅ **metrics.rs** - External client metrics (KEEP)
5. ✅ **error.rs** - Client-specific errors (KEEP)
6. ❌ **client.rs** - DELETE (use `SamplingCoordinator` instead)
7. ❌ **llm_adapter.rs** - DELETE (use `LlmProvider` instead)
8. ❌ **tool_executor.rs** - DELETE (use `ToolRegistry` instead)
9. ❌ **context.rs** - DELETE (use `ConversationSession` instead)
10. ❌ **schema/** - DELETE (use provider implementations)

**New Size:** ~400 LOC (down from 1,407)

**Renamed To:** `botticelli_mcp_external` (clearer purpose)

### Unified Type System

```
┌─────────────────────────────────────────────────────────┐
│                  botticelli_core                        │
│  - Message, Role, Input, Output                         │
│  - GenerateRequest, GenerateResponse                    │
│  - LlmProvider trait                                    │
│  - ToolCall (shared tool call type)                     │
└─────────────────────────────────────────────────────────┘
                           ▲
                           │ (uses)
            ┌──────────────┼──────────────┐
            │              │              │
     ┌──────▼──────┐  ┌───▼───────┐  ┌──▼──────────────┐
     │ botticelli  │  │ botticelli│  │ botticelli_mcp  │
     │   _models   │  │   _chat   │  │                 │
     │             │  │           │  │ - Conversation  │
     │ Anthropic   │  │ Sampling  │  │ - ToolRegistry  │
     │ Gemini      │  │ TUI       │  │ - Coordinator   │
     │ Groq        │  │           │  │ - LlmSampler    │
     │ Ollama      │  │           │  │                 │
     └─────────────┘  └───────────┘  └─────────────────┘
                                               ▲
                                               │ (uses)
                                    ┌──────────▼──────────┐
                                    │ botticelli_mcp      │
                                    │    _external        │
                                    │                     │
                                    │ - ExternalClient    │
                                    │ - Approval          │
                                    │ - Retry/Metrics     │
                                    └─────────────────────┘
```

**Key Principle:** One type system, one trait hierarchy, no duplication.

---

## Implementation Plan

### Phase 1: Create Migration Branch (1 hour)

**Goal:** Safe workspace for breaking changes.

**Steps:**
1. Create `feature/mcp-client-unified-arch` branch
2. Document all current usages
3. Set up tracking document
4. Baseline all tests

**Success Criteria:**
- ✅ Branch created
- ✅ All tests pass on branch start
- ✅ Baseline metrics recorded

### Phase 2: Extract External Client (4 hours)

**Goal:** Separate unique functionality before deletion.

**Step 2.1: Create `botticelli_mcp_external` crate**

```bash
cargo new --lib crates/botticelli_mcp_external
```

**Step 2.2: Move external-specific code**

Files to move:
- `external_client.rs` → Keep all (ChildProcessTransport, etc.)
- `approval.rs` → Keep all
- `retry.rs` → Keep all
- `metrics.rs` → Keep all
- `error.rs` → Keep only external-specific errors

**Step 2.3: Update dependencies**

```toml
# crates/botticelli_mcp_external/Cargo.toml
[dependencies]
botticelli_core = { path = "../botticelli_core" }
botticelli_mcp = { path = "../botticelli_mcp" }
pmcp = "0.1"
tokio = { version = "1", features = ["process", "io-util"] }
serde_json = "1"
tracing = "0.1"
```

**Step 2.4: Update imports**

```rust
// external_client.rs - use core types
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_mcp::{ToolRegistry, ToolDefinition};
use pmcp::types::{ServerInfo, Tool};
```

**Step 2.5: Test external client in isolation**

```bash
just test-package botticelli_mcp_external
```

**Success Criteria:**
- ✅ New crate compiles
- ✅ External client tests pass
- ✅ All unique functionality preserved
- ✅ Dependencies only on core types

**Files Changed:**
- `crates/botticelli_mcp_external/` (NEW)
- `Cargo.toml` (workspace member)

### Phase 3: Migrate `client.rs` → `SamplingCoordinator` (4 hours)

**Goal:** Use existing conversation orchestration.

**Current `client.rs` responsibilities:**
1. Initialize with LLM + tools
2. Execute user prompt
3. Loop: Generate → Parse tools → Execute → Continue
4. Return final response

**Existing equivalent: `botticelli_mcp::SamplingCoordinator`**
- ✅ Has `sampler: Arc<dyn LlmSampler>`
- ✅ Has `tool_registry: Arc<ToolRegistry>`
- ✅ Has conversation loop
- ✅ Handles tool calling
- ❌ Needs external tool support

**Step 3.1: Extend `SamplingCoordinator`**

```rust
// crates/botticelli_mcp/src/tools/sampling.rs

impl SamplingCoordinator {
    /// Add external tools from an external MCP server.
    pub fn add_external_tools(
        &mut self,
        client: Arc<ExternalMcpClient>,
    ) -> BotticelliResult<()> {
        let external_tools = client.list_tools().await?;
        
        for tool in external_tools {
            self.tool_registry.register_external(tool, client.clone())?;
        }
        
        Ok(())
    }
    
    /// Execute a self-driving prompt with internal + external tools.
    pub async fn execute_with_external(
        &self,
        prompt: impl Into<String>,
        external_clients: Vec<Arc<ExternalMcpClient>>,
    ) -> BotticelliResult<String> {
        let mut session = ConversationSession::new(
            "You are a helpful assistant with access to tools."
        );
        
        session.add_turn(ConversationTurn::UserMessage {
            content: prompt.into(),
            attachments: None,
        });
        
        // Add external tools
        for client in external_clients {
            self.tool_registry.register_external_client(client)?;
        }
        
        // Get all tools (internal + external)
        let tools = self.tool_registry.tool_definitions();
        
        // Sample with tools
        self.sampler.sample(&mut session, &tools).await?;
        
        // Extract final response
        let response = extract_final_response(&session)?;
        Ok(response)
    }
}
```

**Step 3.2: Update `ToolRegistry` for external tools**

```rust
// crates/botticelli_mcp/src/tools/registry.rs

pub struct ToolRegistry {
    internal_tools: HashMap<String, Box<dyn McpTool>>,
    external_clients: HashMap<String, Arc<ExternalMcpClient>>,
    external_tool_mapping: HashMap<String, String>, // tool_name -> client_id
}

impl ToolRegistry {
    pub fn register_external_client(
        &mut self,
        client: Arc<ExternalMcpClient>,
    ) -> BotticelliResult<()> {
        let client_id = client.id();
        let tools = client.list_tools().await?;
        
        for tool in tools {
            self.external_tool_mapping.insert(
                tool.name.clone(),
                client_id.clone(),
            );
        }
        
        self.external_clients.insert(client_id, client);
        Ok(())
    }
    
    pub async fn execute_tool(
        &self,
        name: &str,
        input: ToolInput,
    ) -> BotticelliResult<ToolOutput> {
        // Try internal first
        if let Some(tool) = self.internal_tools.get(name) {
            return tool.execute(input).await;
        }
        
        // Try external
        if let Some(client_id) = self.external_tool_mapping.get(name) {
            if let Some(client) = self.external_clients.get(client_id) {
                return client.call_tool(name, input).await;
            }
        }
        
        Err(ToolError::NotFound(name.to_string()))
    }
}
```

**Step 3.3: Update CLI to use new architecture**

```rust
// crates/botticelli/src/cli/mcp.rs

use botticelli_mcp::{SamplingCoordinator, ToolRegistry};
use botticelli_mcp_external::ExternalMcpClient;
use botticelli_chat::ChatLlmSampler;

pub async fn run_self_driving(
    prompt: String,
    external_server_configs: Vec<ExternalServerConfig>,
) -> BotticelliResult<()> {
    // Create provider and sampler
    let provider = create_provider()?;
    let sampler = Arc::new(ChatLlmSampler::new(
        provider,
        Arc::new(ToolRegistry::new()),
    ));
    
    // Create coordinator
    let tool_registry = Arc::new(ToolRegistry::new());
    let coordinator = SamplingCoordinator::new(sampler, tool_registry);
    
    // Connect external servers
    let mut external_clients = Vec::new();
    for config in external_server_configs {
        let client = ExternalMcpClient::connect(config).await?;
        external_clients.push(Arc::new(client));
    }
    
    // Execute with all tools
    let response = coordinator
        .execute_with_external(prompt, external_clients)
        .await?;
    
    println!("Response: {}", response);
    Ok(())
}
```

**Step 3.4: Delete old `client.rs`**

```bash
git rm crates/botticelli_mcp_client/src/client.rs
```

**Success Criteria:**
- ✅ `SamplingCoordinator` supports external tools
- ✅ `ToolRegistry` routes to internal/external
- ✅ CLI uses new architecture
- ✅ Self-driving works end-to-end
- ✅ Old `client.rs` deleted

**Files Changed:**
- `crates/botticelli_mcp/src/tools/sampling.rs` (~50 lines)
- `crates/botticelli_mcp/src/tools/registry.rs` (~100 lines)
- `crates/botticelli/src/cli/mcp.rs` (~50 lines refactor)
- `crates/botticelli_mcp_client/src/client.rs` (DELETE)

### Phase 4: Delete `llm_adapter.rs` + `schema/` (2 hours)

**Goal:** Use existing `LlmProvider` implementations.

**Step 4.1: Update `LlmSampler` trait**

```rust
// crates/botticelli_mcp/src/tools/sampling.rs

#[async_trait]
pub trait LlmSampler: Send + Sync {
    /// Low-level: Generate a single response with optional tools.
    async fn generate_with_tools(
        &self,
        session: &ConversationSession,
        tools: &[ToolDefinition],
    ) -> SamplingResult<GenerateResponse>;
    
    /// High-level: Sample until completion (handles tool calling loop).
    async fn sample(
        &self,
        session: &mut ConversationSession,
        tools: &[ToolDefinition],
    ) -> SamplingResult<GenerateResponse>;
}
```

**Step 4.2: Verify `ChatLlmSampler` implementation**

```rust
// crates/botticelli_chat/src/sampling.rs

#[async_trait]
impl LlmSampler for ChatLlmSampler {
    async fn generate_with_tools(
        &self,
        session: &ConversationSession,
        tools: &[ToolDefinition],
    ) -> SamplingResult<GenerateResponse> {
        let request = self.build_request(session, tools)?;
        
        // Use existing LlmProvider
        let response = self.provider.generate(request).await?;
        
        Ok(response)
    }
    
    async fn sample(
        &self,
        session: &mut ConversationSession,
        tools: &[ToolDefinition],
    ) -> SamplingResult<GenerateResponse> {
        // Full tool calling loop
        loop {
            let response = self.generate_with_tools(session, tools).await?;
            
            // Extract tool calls
            let tool_calls = extract_tool_calls(&response);
            
            if tool_calls.is_empty() {
                // No more tools, done
                return Ok(response);
            }
            
            // Execute tools
            let results = self.tool_registry.execute_tools(tool_calls).await?;
            
            // Add results to session
            session.add_turn(ConversationTurn::ToolResults { results });
            
            // Continue loop
        }
    }
}
```

**Step 4.3: Delete duplicate code**

```bash
git rm -r crates/botticelli_mcp_client/src/llm_adapter.rs
git rm -r crates/botticelli_mcp_client/src/schema/
```

**Step 4.4: Verify all providers work**

```bash
just test-package botticelli_chat
just test-package botticelli_mcp
```

**Success Criteria:**
- ✅ `LlmSampler` trait is sufficient
- ✅ `ChatLlmSampler` fully implements trait
- ✅ All providers work through existing infrastructure
- ✅ Old `llm_adapter.rs` deleted
- ✅ Old `schema/` deleted

**Files Changed:**
- `crates/botticelli_mcp/src/tools/sampling.rs` (trait cleanup)
- `crates/botticelli_chat/src/sampling.rs` (verify)
- `crates/botticelli_mcp_client/src/llm_adapter.rs` (DELETE)
- `crates/botticelli_mcp_client/src/schema/` (DELETE)

### Phase 5: Delete `tool_executor.rs` (1 hour)

**Goal:** Use existing `ToolRegistry`.

**Step 5.1: Verify `ToolRegistry` has needed features**

```rust
// crates/botticelli_mcp/src/tools/registry.rs

impl ToolRegistry {
    /// Get all tool definitions for LLM context.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        // Internal tools
        let mut defs: Vec<_> = self.internal_tools
            .values()
            .map(|tool| tool.definition())
            .collect();
        
        // External tools
        for (tool_name, client_id) in &self.external_tool_mapping {
            if let Some(client) = self.external_clients.get(client_id) {
                if let Some(def) = client.get_tool_definition(tool_name) {
                    defs.push(def);
                }
            }
        }
        
        defs
    }
    
    /// Execute a tool by name.
    pub async fn execute_tool(
        &self,
        name: &str,
        input: ToolInput,
    ) -> BotticelliResult<ToolOutput> {
        // Already implemented in Phase 3
    }
}
```

**Step 5.2: Delete old tool executor**

```bash
git rm crates/botticelli_mcp_client/src/tool_executor.rs
```

**Success Criteria:**
- ✅ `ToolRegistry` provides all needed functionality
- ✅ Old `tool_executor.rs` deleted

**Files Changed:**
- `crates/botticelli_mcp_client/src/tool_executor.rs` (DELETE)

### Phase 6: Delete `context.rs` (1 hour)

**Goal:** Use existing `ConversationSession`.

**Step 6.1: Verify `ConversationSession` has needed features**

```rust
// crates/botticelli_mcp/src/conversation.rs

impl ConversationSession {
    /// Add history limit with summarization.
    pub fn set_max_history(&mut self, max: usize) {
        self.max_turns = max;
    }
    
    /// Summarize old turns (placeholder for now).
    pub async fn summarize_history(
        &mut self,
        sampler: &dyn LlmSampler,
    ) -> BotticelliResult<()> {
        if self.turns.len() <= self.max_turns {
            return Ok(());
        }
        
        // Extract old turns
        let old_turns = self.turns.drain(0..self.turns.len()/2).collect::<Vec<_>>();
        
        // Create summarization request
        let summary_prompt = format!(
            "Summarize this conversation history:\n{:#?}",
            old_turns
        );
        
        let mut temp_session = ConversationSession::new(
            "You are a helpful assistant that summarizes conversations."
        );
        temp_session.add_turn(ConversationTurn::UserMessage {
            content: summary_prompt,
            attachments: None,
        });
        
        let response = sampler.generate_with_tools(&temp_session, &[]).await?;
        let summary = extract_text(&response);
        
        // Insert summary at beginning
        self.turns.insert(0, ConversationTurn::AssistantMessage {
            content: format!("Previous conversation summary: {}", summary),
        });
        
        Ok(())
    }
}
```

**Step 6.2: Delete old context manager**

```bash
git rm crates/botticelli_mcp_client/src/context.rs
```

**Success Criteria:**
- ✅ `ConversationSession` provides all needed functionality
- ✅ History management works
- ✅ Old `context.rs` deleted

**Files Changed:**
- `crates/botticelli_mcp/src/conversation.rs` (~30 lines)
- `crates/botticelli_mcp_client/src/context.rs` (DELETE)

### Phase 7: Rename & Clean (2 hours)

**Goal:** Rename crate to reflect new purpose.

**Step 7.1: Rename crate**

```bash
# Rename directory
git mv crates/botticelli_mcp_client crates/botticelli_mcp_external

# Update Cargo.toml
sed -i 's/botticelli_mcp_client/botticelli_mcp_external/g' crates/botticelli_mcp_external/Cargo.toml

# Update workspace
sed -i 's/botticelli_mcp_client/botticelli_mcp_external/g' Cargo.toml
```

**Step 7.2: Update lib.rs exports**

```rust
// crates/botticelli_mcp_external/src/lib.rs

//! External MCP server connectivity.
//!
//! This crate provides client functionality for connecting to and
//! communicating with external MCP servers (e.g., filesystem, git).

mod approval;
mod external_client;
mod error;
mod metrics;
mod retry;

pub use approval::{ApprovalMode, ApprovalRequest};
pub use error::{ExternalClientError, ExternalClientErrorKind, ExternalClientResult};
pub use external_client::{
    ChildProcessTransport, ExternalMcpClient, ExternalServerConfig, Transport,
};
pub use metrics::ExternalClientMetrics;
pub use retry::{RetryConfig, RetryStrategy};
```

**Step 7.3: Update all imports**

```bash
# Find all usages
rg "botticelli_mcp_client" --files-with-matches

# Update imports
sed -i 's/botticelli_mcp_client/botticelli_mcp_external/g' crates/botticelli/src/cli/mcp.rs
```

**Step 7.4: Update documentation**

- Update README references
- Update PLANNING_INDEX.md
- Update cargo metadata

**Success Criteria:**
- ✅ Crate renamed
- ✅ All imports updated
- ✅ Documentation updated
- ✅ Cargo metadata correct

**Files Changed:**
- `crates/botticelli_mcp_external/` (RENAMED)
- `Cargo.toml` (workspace)
- `crates/botticelli/src/cli/mcp.rs` (imports)
- Documentation files

### Phase 8: Testing & Validation (4 hours)

**Goal:** Ensure unified architecture works correctly.

**Step 8.1: Unit tests**

```bash
just test-package botticelli_mcp_external
just test-package botticelli_mcp
just test-package botticelli_chat
```

**Step 8.2: Integration tests**

Create comprehensive integration test:

```rust
// crates/botticelli_mcp/tests/self_driving_integration_test.rs

#[tokio::test]
async fn test_self_driving_with_external_tools() {
    // Create provider
    let provider = create_test_provider();
    
    // Create sampler
    let tool_registry = Arc::new(ToolRegistry::new());
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry.clone()));
    
    // Create coordinator
    let coordinator = SamplingCoordinator::new(sampler, tool_registry);
    
    // Connect external filesystem server
    let fs_config = ExternalServerConfig {
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "./test_data".to_string(),
        ],
        env: HashMap::new(),
    };
    
    let fs_client = ExternalMcpClient::connect(fs_config).await.unwrap();
    
    // Execute self-driving prompt
    let response = coordinator
        .execute_with_external(
            "List all files in the current directory",
            vec![Arc::new(fs_client)],
        )
        .await
        .unwrap();
    
    // Verify response mentions files
    assert!(response.contains("file") || response.contains("directory"));
}

#[tokio::test]
async fn test_internal_and_external_tools_together() {
    // Create coordinator with internal tools
    let tool_registry = Arc::new(ToolRegistry::new());
    tool_registry.register(Box::new(ServerInfoTool::new()))?;
    
    let provider = create_test_provider();
    let sampler = Arc::new(ChatLlmSampler::new(provider, tool_registry.clone()));
    let coordinator = SamplingCoordinator::new(sampler, tool_registry);
    
    // Add external tools
    let git_config = ExternalServerConfig {
        command: "mcp-server-git".to_string(),
        args: vec!["--repo".to_string(), ".".to_string()],
        env: HashMap::new(),
    };
    
    let git_client = ExternalMcpClient::connect(git_config).await.unwrap();
    
    // Execute prompt that uses both internal and external
    let response = coordinator
        .execute_with_external(
            "What server version are you running, and what's the latest git commit?",
            vec![Arc::new(git_client)],
        )
        .await
        .unwrap();
    
    // Should mention both internal tool (server version) and external (git)
    assert!(response.len() > 0);
}
```

**Step 8.3: Manual end-to-end testing**

```bash
# Test self-driving CLI with filesystem server
cargo run --bin botticelli -- mcp self-drive \
    --prompt "List files in ./src" \
    --external-server "npx -y @modelcontextprotocol/server-filesystem ./src"

# Test with multiple external servers
cargo run --bin botticelli -- mcp self-drive \
    --prompt "Show me files and recent commits" \
    --external-server "npx -y @modelcontextprotocol/server-filesystem ." \
    --external-server "mcp-server-git --repo ."
```

**Step 8.4: Performance benchmarks**

```rust
// Compare old vs new architecture
#[bench]
fn bench_tool_execution_old(b: &mut Bencher) {
    // Old: Type translation overhead
}

#[bench]
fn bench_tool_execution_new(b: &mut Bencher) {
    // New: Direct types, no translation
}
```

**Step 8.5: Documentation validation**

- Verify all examples work
- Check API docs are clear
- Validate usage guides

**Success Criteria:**
- ✅ All unit tests pass
- ✅ Integration tests pass
- ✅ Manual testing succeeds
- ✅ Performance is same or better
- ✅ Documentation is accurate

**Files Changed:**
- `crates/botticelli_mcp/tests/self_driving_integration_test.rs` (NEW)
- Various test files (updates)

### Phase 9: Update Documentation (2 hours)

**Goal:** Document the new unified architecture.

**Step 9.1: Update planning documents**

- Mark this plan as COMPLETE
- Update PLANNING_INDEX.md
- Update PLANNING_TRACKER.md

**Step 9.2: Update technical documentation**

Create `MCP_UNIFIED_ARCHITECTURE.md`:

```markdown
# Botticelli MCP Unified Architecture

## Overview

Botticelli uses a unified type system across all MCP functionality:
- Single `Message` type (botticelli_core)
- Single `LlmProvider` trait (botticelli_core)
- Single `ConversationSession` (botticelli_mcp)
- Single `ToolRegistry` (botticelli_mcp)

## Architecture Diagram

[Include the diagram from this document]

## Self-Driving Flow

1. User provides prompt
2. SamplingCoordinator creates ConversationSession
3. LlmSampler generates with tools
4. ToolRegistry executes (internal or external)
5. Repeat until completion
6. Return final response

## Adding External Tools

[Usage examples]

## Benefits

- No type duplication
- Single source of truth
- Maintainable long-term
- Type-safe throughout
```

**Step 9.3: Update API documentation**

- Add examples to pub items
- Document integration points
- Show usage patterns

**Step 9.4: Update README**

- Reflect new architecture
- Update feature list
- Show self-driving examples

**Success Criteria:**
- ✅ Planning docs updated
- ✅ Technical docs created
- ✅ API docs complete
- ✅ README accurate

**Files Changed:**
- `MCP_UNIFIED_ARCHITECTURE.md` (NEW)
- `PLANNING_INDEX.md` (update)
- `PLANNING_TRACKER.md` (update)
- `README.md` (update)
- API doc comments (various files)

### Phase 10: Cleanup & PR (2 hours)

**Goal:** Prepare for merge to dev.

**Step 10.1: Run all checks**

```bash
just check-all
just test-all
just audit
just check-features
```

**Step 10.2: Review diff**

```bash
git diff dev...feature/mcp-client-unified-arch --stat
git diff dev...feature/mcp-client-unified-arch
```

**Step 10.3: Write comprehensive PR**

```markdown
# MCP Client Unified Architecture Refactor

## Summary

Eliminates architectural duplication by migrating `botticelli_mcp_client`
to use existing proven type systems.

## Changes

**Deleted (~1000 LOC):**
- client.rs - use SamplingCoordinator
- llm_adapter.rs - use LlmProvider
- tool_executor.rs - use ToolRegistry
- context.rs - use ConversationSession
- schema/* - use provider implementations

**Kept (~400 LOC):**
- external_client.rs - unique external MCP functionality
- approval.rs - user approval for external tools
- retry.rs - retry logic
- metrics.rs - external client metrics

**Renamed:**
- botticelli_mcp_client → botticelli_mcp_external

**Result:**
- Single unified type system
- No type translation overhead
- Easier maintenance
- Clear responsibility boundaries

## Testing

- ✅ All unit tests pass
- ✅ Integration tests pass
- ✅ Manual end-to-end validated
- ✅ Documentation updated

## Breaking Changes

Minimal external impact (only 2 files imported from this crate).
Migration guide provided in documentation.

## Benefits

1. **Maintainability:** Changes happen once
2. **Type Safety:** No runtime translation
3. **Code Size:** -70% LOC in mcp_external
4. **Clarity:** Clear responsibilities
5. **Performance:** No translation overhead
```

**Step 10.4: Commit strategy**

One commit per phase for easy review:

```bash
git add -A && git commit -m "refactor(mcp): Phase 1 - Create botticelli_mcp_external crate"
git add -A && git commit -m "refactor(mcp): Phase 2 - Migrate to SamplingCoordinator"
# ... etc
```

**Step 10.5: Create PR**

```bash
gh pr create \
    --title "Refactor: MCP Client Unified Architecture" \
    --body-file PR_DESCRIPTION.md \
    --base dev \
    --label "refactor,breaking-change"
```

**Success Criteria:**
- ✅ All checks pass
- ✅ Diff reviewed
- ✅ PR created
- ✅ Ready for review

---

## Before/After Comparison

### Before: Duplicated Architecture

```
botticelli_mcp_client (1,407 LOC)
├── client.rs              ❌ Duplicates SamplingCoordinator
├── llm_adapter.rs         ❌ Duplicates LlmProvider
├── tool_executor.rs       ❌ Duplicates ToolRegistry
├── context.rs             ❌ Duplicates ConversationSession
├── schema/                ❌ Duplicates provider schemas
│   ├── anthropic.rs
│   ├── gemini.rs
│   ├── groq.rs
│   ├── ollama.rs
│   ├── openai.rs
│   └── huggingface.rs
├── external_client.rs     ✅ Unique functionality
├── approval.rs            ✅ Unique functionality
├── retry.rs               ✅ Unique functionality
├── metrics.rs             ✅ Unique functionality
└── error.rs               ✅ Unique functionality
```

**Problems:**
- 6 files duplicating existing functionality (~1000 LOC)
- Type translation at boundaries
- Two ways to do everything
- Maintenance burden

### After: Unified Architecture

```
botticelli_mcp_external (~400 LOC)
├── external_client.rs     ✅ Connects to external MCP servers
├── approval.rs            ✅ User approval for external tools
├── retry.rs               ✅ Retry with backoff
├── metrics.rs             ✅ External client metrics
└── error.rs               ✅ External-specific errors

Uses from other crates:
├── botticelli_core::LlmProvider      (trait)
├── botticelli_core::Message          (types)
├── botticelli_mcp::SamplingCoordinator
├── botticelli_mcp::ToolRegistry
├── botticelli_mcp::ConversationSession
└── botticelli_chat::ChatLlmSampler
```

**Benefits:**
- Single type system
- No duplication
- Clear responsibilities
- Easy maintenance

---

## Risks & Mitigation

### Risk 1: Breaking Existing Code

**Likelihood:** Low (only 2 import locations)

**Impact:** Medium (CLI needs updates)

**Mitigation:**
- Update imports in same PR
- Provide migration guide
- Test thoroughly before merge

### Risk 2: Missing Functionality

**Likelihood:** Low (existing code is more complete)

**Impact:** High (could break features)

**Mitigation:**
- Comprehensive testing
- Side-by-side comparison
- Feature checklist validation

### Risk 3: Performance Regression

**Likelihood:** Very Low (removing translation overhead)

**Impact:** Medium

**Mitigation:**
- Benchmark old vs new
- Performance tests in CI
- Monitor metrics

### Risk 4: Incomplete Migration

**Likelihood:** Low (good planning)

**Impact:** High (half-migrated is worse than not migrated)

**Mitigation:**
- Detailed phase checklist
- Success criteria per phase
- Don't merge until 100% complete

---

## Success Metrics

### Code Metrics

**Before:**
- `botticelli_mcp_client`: 1,407 LOC
- Duplicate types: 4 types × 2 locations = 8
- Duplicate traits: 2 traits × 2 locations = 4
- Duplicate implementations: 6 providers × 2 = 12

**After:**
- `botticelli_mcp_external`: ~400 LOC
- Duplicate types: 0
- Duplicate traits: 0
- Duplicate implementations: 0

**Improvement:**
- -70% code in external crate
- -100% type duplication
- -100% trait duplication
- -100% implementation duplication

### Maintenance Metrics

**Before:**
- API change: Update in 2 places
- Bug fix: Update in 2 places
- New provider: Implement in 2 places

**After:**
- API change: Update in 1 place
- Bug fix: Update in 1 place
- New provider: Implement in 1 place

**Improvement:**
- 50% fewer changes for any update

### Type Safety Metrics

**Before:**
- Type translations: ~15 per request
- Runtime translation errors: Possible
- Type mismatches: Caught at translation

**After:**
- Type translations: 0
- Runtime translation errors: Impossible
- Type mismatches: Caught at compile time

**Improvement:**
- 100% fewer translations
- 100% fewer runtime errors
- Better compile-time safety

---

## Timeline

| Phase | Duration | Cumulative |
|-------|----------|------------|
| 1. Migration branch | 1h | 1h |
| 2. Extract external | 4h | 5h |
| 3. Migrate client | 4h | 9h |
| 4. Delete llm_adapter | 2h | 11h |
| 5. Delete tool_executor | 1h | 12h |
| 6. Delete context | 1h | 13h |
| 7. Rename & clean | 2h | 15h |
| 8. Testing | 4h | 19h |
| 9. Documentation | 2h | 21h |
| 10. Cleanup & PR | 2h | 23h |

**Total: 23 hours (3 days with breaks)**

**Buffer: +1 day for unexpected issues**

**Final Estimate: 4-5 days**

---

## Acceptance Criteria

### Phase Completion

- ✅ All 10 phases complete
- ✅ All success criteria met
- ✅ All tests passing
- ✅ All docs updated

### Code Quality

- ✅ Zero TODOs in production code
- ✅ Zero type duplication
- ✅ Zero clippy warnings
- ✅ Zero failed tests

### Functionality

- ✅ Self-driving works end-to-end
- ✅ Internal tools work
- ✅ External tools work
- ✅ Mixed internal+external works

### Documentation

- ✅ Architecture documented
- ✅ API examples provided
- ✅ Migration guide complete
- ✅ README updated

### Review

- ✅ PR created
- ✅ All checks passing
- ✅ Ready for merge

---

## Next Steps

**Ready to proceed with Phase 1: Create Migration Branch?**

This comprehensive plan will:
1. Eliminate ~1000 LOC of duplication
2. Unify the type system
3. Improve maintainability
4. Preserve all functionality
5. Take 4-5 days to complete properly

The investment in doing this right now will save significant maintenance
burden over the lifetime of the project.
