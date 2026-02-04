# Parallel Implementation Analysis: Enabling Agent Autonomy Through Abstracted Elicitation

## Executive Summary

**The Real Goal**: Enable agents to seamlessly elicit arbitrary data through sampling, while humans elicit through interactive prompts - using the same types and tools.

**The Problem**: 
- ~489 lines of handrolled elicitation code in `botticelli_mcp` is human-only
- No abstraction layer for agent-based elicitation (sampling/reasoning)
- Elicitation derives can't reach their full potential (work for humans, not agents)

**The Solution**: Refactor to Provider trait architecture
- Abstracts over human (interactive) vs agent (sampling) protocols
- Each context implements ElicitationProvider trait
- Server tools become generic over Provider
- All Elicit-derived types work in BOTH contexts automatically

**The Vision**: **Agents can elicit as much data as they want through sampling** - seamlessly constructing complex types through their reasoning capabilities

**Impact**: 
- Remove ~122 lines of redundant code
- Gain automatic tool generation for 159+ types
- Enable agent autonomy through abstracted elicitation
- One architecture supports humans AND agents

---

## The Architecture Vision

### Provider Trait Pattern

```rust
/// Types that provide elicitation services
pub trait Provider {
    type ProviderType: ElicitationProvider;
    fn provider(&self) -> &Self::ProviderType;
}

/// Primitive elicitation operations
pub trait ElicitationProvider {
    async fn get_text(&self, prompt: &str) -> Result<String, Error>;
    async fn get_bool(&self, prompt: &str, default: bool) -> Result<bool, Error>;
    async fn get_number(&self, prompt: &str, min: i64, max: i64) -> Result<i64, Error>;
    async fn get_select(&self, prompt: &str, options: &[&str]) -> Result<String, Error>;
}

// Human implementation - interactive TUI prompts
struct HumanElicitation {
    dialog: Arc<DialogResource>,
}

impl ElicitationProvider for HumanElicitation {
    async fn get_text(&self, prompt: &str) -> Result<String, Error> {
        self.dialog.ask_text(prompt).await  // Interactive prompt in TUI
    }
    // ... other methods delegate to dialog
}

// Agent implementation - sampling/reasoning via LLM
struct AgentElicitation {
    client: Arc<dyn BotticelliDriver>,
}

impl ElicitationProvider for AgentElicitation {
    async fn get_text(&self, prompt: &str) -> Result<String, Error> {
        // Agent reasons about prompt through sampling
        let request = self.build_sampling_request(prompt);
        let response = self.client.generate(&request).await?;
        Ok(response.content)
    }
    // ... other methods use LLM reasoning
}

// Session types specify their provider
impl Provider for HumanSession {
    type ProviderType = HumanElicitation;
    fn provider(&self) -> &Self::ProviderType { &self.elicitation }
}

impl Provider for AgentSession {
    type ProviderType = AgentElicitation;
    fn provider(&self) -> &Self::ProviderType { &self.elicitation }
}

// Generic server tools work for BOTH contexts
impl BotticelliServer {
    pub async fn elicit_text<P: Provider>(
        &self,
        session: &P,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        let prompt = parse_prompt(params)?;
        let text = session.provider().get_text(&prompt).await?;
        Ok(json!({ "value": text }))
    }
}
```

### The Payoff

**Every Elicit-derived type automatically works in both contexts:**

```rust
#[derive(elicitation::Elicit)]
pub struct NarrativeMetadata {
    name: String,
    description: String,
    default_model: Option<String>,
}

// Auto-generated tool:
pub async fn elicit_narrative_metadata<P: Provider>(
    provider: &P
) -> Result<NarrativeMetadata, Error> {
    let name = provider.get_text("Enter narrative name:").await?;
    let description = provider.get_text("Enter description:").await?;
    let model = provider.get_text("Default model (optional):").await?;
    Ok(NarrativeMetadata { name, description, model })
}

// Works seamlessly:
// - Human session: Prompts user for each field interactively
// - Agent session: Agent samples/reasons about each field
```

**Agents can now autonomously construct ANY complex type through sampling!**

---

## The Redundancy

### 1. Primitive Elicitation Tools (4 types × 2 files each = ~122 lines)

**Handrolled in `botticelli_mcp`:**

```rust
// crates/botticelli_mcp/src/elicit_text.rs (22 lines)
pub struct ElicitTextParams { prompt: String }
pub struct ElicitTextResult { value: String }

// crates/botticelli_mcp/src/elicit_bool.rs (25 lines)  
pub struct ElicitBoolParams { prompt: String, default: bool }
pub struct ElicitBoolResult { value: bool }

// crates/botticelli_mcp/src/elicit_number.rs (27 lines)
pub struct ElicitNumberParams { prompt: String, min: i64, max: i64 }
pub struct ElicitNumberResult { value: i64 }

// crates/botticelli_mcp/src/elicit_select.rs (48 lines)
pub struct ElicitSelectParams { prompt: String, options: Vec<String> }
pub struct ElicitSelectResult { value: String }
```

**Built-in to `elicitation` (307 lines in `src/mcp/`):**

```rust
// elicitation/src/mcp/tools.rs - Automatically provided!
pub mod tool_names {
    pub fn elicit_text() -> &'static str { "elicit_text" }
    pub fn elicit_bool() -> &'static str { "elicit_bool" }
    pub fn elicit_integer() -> &'static str { "elicit_integer" }
    pub fn elicit_select() -> &'static str { "elicit_select" }
}

pub fn text_params(prompt: &str) -> serde_json::Value { /* ... */ }
pub fn bool_params(prompt: &str) -> serde_json::Value { /* ... */ }
pub fn number_params(prompt: &str, min: i64, max: i64) -> serde_json::Value { /* ... */ }
pub fn select_params(prompt: &str, options: &[&str]) -> serde_json::Value { /* ... */ }
```

**Verdict**: ❌ **Delete handrolled types** - elicitation provides these out-of-the-box

---

### 2. Tool Implementation Functions (7 functions = ~200 lines)

**Handrolled in `rmcp_server/tools/elicitation.rs` (367 lines total):**

```rust
impl BotticelliServer {
    // Lines 22-48: elicit_text() - delegates to dialog.ask_text()
    pub async fn elicit_text(&self, params: ElicitTextParams) 
        -> Result<ElicitTextResult, rmcp::ErrorData> { /* ... */ }

    // Lines 52-79: elicit_bool() - delegates to dialog.ask_confirmation()
    pub async fn elicit_bool(&self, params: ElicitBoolParams) 
        -> Result<ElicitBoolResult, rmcp::ErrorData> { /* ... */ }

    // Lines 83-120: elicit_number() - delegates to dialog.ask_number()
    pub async fn elicit_number(&self, params: ElicitNumberParams)
        -> Result<ElicitNumberResult, rmcp::ErrorData> { /* ... */ }

    // Lines 124-172: elicit_select() - delegates to dialog.ask_choice()
    pub async fn elicit_select(&self, params: ElicitSelectParams)
        -> Result<ElicitSelectResult, rmcp::ErrorData> { /* ... */ }
        
    // Lines 176-220: elicit_metadata() - narrative-specific (KEEP?)
    // Lines 224-284: elicit_act() - narrative-specific (KEEP?)
    // Lines 288-367: elicit_carousel() - narrative-specific (KEEP?)
}
```

**What elicitation provides:**

```rust
// Types with #[derive(Elicit)] automatically get:
impl Elicitation for MyType {
    async fn elicit(client: &ElicitClient<'_>) -> ElicitResult<Self> {
        // Automatically generated - calls elicit_text, elicit_bool, etc.
    }
}

// Plus auto-generated MCP tool function:
pub async fn elicit_my_type(
    client: &Peer<RoleClient>,
) -> Result<MyType, ElicitError> {
    MyType::elicit(&ElicitClient::new(client)).await
}
```

**Verdict**: 
- ❌ **Delete** `elicit_text`, `elicit_bool`, `elicit_number`, `elicit_select` (redundant)
- ✅ **KEEP** `elicit_metadata`, `elicit_act`, `elicit_carousel` (domain-specific, not primitives)

---

### 3. Helper Code (tools/elicitation/* = ~167 lines)

**Files in `src/tools/elicitation/`:**

```
helpers.rs   - 222 lines - DescriptionAnalysis, name validation, act detection
registry.rs  - State management for narratives  
state.rs     - GetNarrativeState input/output types
validation.rs - ValidateNarrative input/output types
mod.rs       - 20 lines - Module exports
```

**Analysis**:
- `helpers.rs`: Domain logic (narrative-specific), **KEEP**
- `registry.rs`, `state.rs`, `validation.rs`: Narrative state management, **KEEP**
- These are NOT primitive elicitation - they're workflow helpers

**Verdict**: ✅ **KEEP ALL** - Not redundant with elicitation crate

---

### 4. DialogResource Integration (1 wrapper = ~30 lines)

**File**: `src/dialog_resource.rs` (connects to UI layer)

```rust
pub struct DialogResource {
    dialog: Arc<Mutex<Box<dyn ElicitationDialog + Send>>>,
}

impl DialogResource {
    pub async fn ask_text(&self, prompt: &str) -> McpResult<String> { /* ... */ }
    pub async fn ask_confirmation(&self, prompt: &str, default: bool) -> McpResult<bool> { /* ... */ }
    pub async fn ask_number(&self, prompt: &str, min: i64, max: i64) -> McpResult<i64> { /* ... */ }
    pub async fn ask_choice(&self, prompt: &str, options: &[&str]) -> McpResult<usize> { /* ... */ }
}
```

**Analysis**:
- This is a **bridge** between MCP server and UI (TUI/CLI)
- ElicitationDialog trait is still needed for UI abstraction
- DialogResource wraps the dialog in Arc<Mutex<>> for sharing

**Verdict**: ✅ **KEEP** - Bridge to UI layer, not redundant

---

## The Architecture Gap

### Current (Handrolled)

```
┌─────────────────────────────────────────────────────────────────┐
│ MCP Client (Claude)                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ call_tool("elicit_text", params)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ BotticelliServer                                                │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ elicit_text() - Manual impl                                 │ │
│ │   └─► dialog.ask_text() ─► UI layer                         │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ elicit_bool() - Manual impl                                 │ │
│ │   └─► dialog.ask_confirmation() ─► UI layer                 │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ... (2 more primitive tools)                                    │
└─────────────────────────────────────────────────────────────────┘
```

**Problem**: Every primitive manually delegates to dialog methods

---

### Future (Elicitation-Native)

```
┌─────────────────────────────────────────────────────────────────┐
│ MCP Client (Claude)                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ call_tool("elicit_narrative_metadata")
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Auto-Generated Tools (from derives)                             │
│   elicit_narrative_metadata() ──┐                               │
│   elicit_toml_act() ────────────┼──► ElicitClient ──┐           │
│   elicit_carousel_config() ─────┘                   │           │
└──────────────────────────────────────────────────────┼───────────┘
                                                       │
                                                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ ElicitClient (from elicitation crate)                           │
│   Calls primitive tools:                                        │
│   - elicit_text                                                 │
│   - elicit_bool      ──────────► Peer::call_tool()              │
│   - elicit_integer                      │                       │
│   - elicit_select                       │                       │
└─────────────────────────────────────────┼───────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ MCP Server (must implement primitives)                          │
│   elicit_text ───────┐                                          │
│   elicit_bool ────────┼───► DialogResource ──► UI               │
│   elicit_integer ─────┤                                          │
│   elicit_select ──────┘                                          │
└─────────────────────────────────────────────────────────────────┘
```

**Key Insight**: We MUST keep the primitive tool implementations (or provide them somehow), because elicitation's auto-generated tools CALL the primitives via MCP.

---

## The Refactoring Path

### Phase 1: Introduce Provider Traits (New Code)

**Add new traits** (~50 lines):

```rust
// crates/botticelli_mcp/src/elicitation_provider.rs (new file)

/// Types that provide elicitation services
pub trait Provider {
    type ProviderType: ElicitationProvider;
    fn provider(&self) -> &Self::ProviderType;
}

/// Primitive elicitation operations
#[async_trait::async_trait]
pub trait ElicitationProvider: Send + Sync {
    async fn get_text(&self, prompt: &str) -> McpResult<String>;
    async fn get_bool(&self, prompt: &str, default: bool) -> McpResult<bool>;
    async fn get_number(&self, prompt: &str, min: i64, max: i64) -> McpResult<i64>;
    async fn get_select(&self, prompt: &str, options: &[&str]) -> McpResult<String>;
}

/// Human elicitation via DialogResource
pub struct HumanElicitation {
    dialog: Arc<DialogResource>,
}

#[async_trait::async_trait]
impl ElicitationProvider for HumanElicitation {
    async fn get_text(&self, prompt: &str) -> McpResult<String> {
        self.dialog.ask_text(prompt).await
    }
    
    async fn get_bool(&self, prompt: &str, default: bool) -> McpResult<bool> {
        self.dialog.ask_confirmation(prompt, default).await
    }
    
    async fn get_number(&self, prompt: &str, min: i64, max: i64) -> McpResult<i64> {
        self.dialog.ask_number(prompt, min, max).await
    }
    
    async fn get_select(&self, prompt: &str, options: &[&str]) -> McpResult<String> {
        let index = self.dialog.ask_choice(prompt, options).await?;
        Ok(options[index].to_string())
    }
}

/// Agent elicitation via sampling/reasoning
pub struct AgentElicitation {
    client: Arc<dyn BotticelliDriver>,
}

#[async_trait::async_trait]
impl ElicitationProvider for AgentElicitation {
    async fn get_text(&self, prompt: &str) -> McpResult<String> {
        // Build sampling request with prompt as context
        let request = GenerateRequest::builder()
            .messages(vec![Message {
                role: Role::User,
                content: vec![Input::Text(format!(
                    "Context: {}\n\nProvide a concise response.",
                    prompt
                ))],
            }])
            .max_tokens(Some(200))
            .build()?;
        
        let response = self.client.generate(&request).await?;
        Ok(response.content)
    }
    
    async fn get_bool(&self, prompt: &str, default: bool) -> McpResult<bool> {
        // Agent reasons about yes/no question
        let request = GenerateRequest::builder()
            .messages(vec![Message {
                role: Role::User,
                content: vec![Input::Text(format!(
                    "{}\n\nRespond with only 'yes' or 'no'.",
                    prompt
                ))],
            }])
            .max_tokens(Some(10))
            .build()?;
        
        let response = self.client.generate(&request).await?;
        parse_bool_from_response(&response.content, default)
    }
    
    // ... similar for get_number, get_select
}
```

### Phase 2: Add Session Context Types (~30 lines)

```rust
// crates/botticelli_mcp/src/session.rs (new file or extend existing)

/// Session with human user
pub struct HumanSession {
    elicitation: HumanElicitation,
    // ... other session state
}

impl Provider for HumanSession {
    type ProviderType = HumanElicitation;
    fn provider(&self) -> &Self::ProviderType { &self.elicitation }
}

/// Session with agent
pub struct AgentSession {
    elicitation: AgentElicitation,
    // ... other session state
}

impl Provider for AgentSession {
    type ProviderType = AgentElicitation;
    fn provider(&self) -> &Self::ProviderType { &self.elicitation }
}
```

### Phase 3: Refactor Primitive Tools (~100 lines changed)

**Update** `rmcp_server/tools/elicitation.rs`:

```rust
impl BotticelliServer {
    // Make tools generic over Provider
    #[tool]
    pub async fn elicit_text<P: Provider>(
        &self,
        session: &P,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        let prompt = elicitation::mcp::extract_value(params)?;
        let prompt = elicitation::mcp::parse_string(prompt)?;
        
        let text = session.provider().get_text(&prompt).await
            .map_err(|e| to_mcp_error(e, "Elicitation failed"))?;
        
        Ok(json!({ "value": text }))
    }
    
    #[tool]
    pub async fn elicit_bool<P: Provider>(
        &self,
        session: &P,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        let params = elicitation::mcp::extract_value(params)?;
        let prompt = params.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| /* error */)?;
        let default = params.get("default")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let value = session.provider().get_bool(prompt, default).await
            .map_err(|e| to_mcp_error(e, "Elicitation failed"))?;
        
        Ok(json!({ "value": value }))
    }
    
    // Similar for elicit_number, elicit_select
}
```

### Phase 4: DELETE Redundant Type Files (~122 lines)

1. **Type definitions** (4 files):
   ```
   ❌ src/elicit_text.rs      (22 lines) - ElicitTextParams/Result
   ❌ src/elicit_bool.rs       (25 lines) - ElicitBoolParams/Result
   ❌ src/elicit_number.rs     (27 lines) - ElicitNumberParams/Result
   ❌ src/elicit_select.rs     (48 lines) - ElicitSelectParams/Result
   ```

2. **Re-exports from lib.rs**:
   ```rust
   ❌ pub use elicit_bool::{ElicitBoolParams, ElicitBoolResult};
   ❌ pub use elicit_number::{ElicitNumberParams, ElicitNumberResult};
   ❌ pub use elicit_select::{ElicitSelectParams, ElicitSelectResult};
   ❌ pub use elicit_text::{ElicitTextParams, ElicitTextResult};
   ```

### Phase 5: KEEP Core Infrastructure (~367 lines)

1. **Primitive tool implementations** (required for elicitation):
   ```rust
   ✅ BotticelliServer::elicit_text()     - MCP tool impl
   ✅ BotticelliServer::elicit_bool()     - MCP tool impl
   ✅ BotticelliServer::elicit_number()   - MCP tool impl (rename to elicit_integer?)
   ✅ BotticelliServer::elicit_select()   - MCP tool impl
   ```

2. **Domain-specific tools**:
   ```rust
   ✅ BotticelliServer::elicit_metadata()  - Narrative-specific
   ✅ BotticelliServer::elicit_act()       - Narrative-specific
   ✅ BotticelliServer::elicit_carousel()  - Narrative-specific
   ```

3. **DialogResource bridge**:
   ```rust
   ✅ src/dialog_resource.rs - UI integration layer
   ```

4. **Helper modules**:
   ```rust
   ✅ src/tools/elicitation/ - Narrative workflow helpers
   ```

### Phase 6: Enable Agent Sessions

**Add agent session creation** (~50 lines):

```rust
impl BotticelliServer {
    pub fn create_human_session(&self, dialog: Arc<DialogResource>) -> HumanSession {
        HumanSession {
            elicitation: HumanElicitation { dialog },
            // ... other state
        }
    }
    
    pub fn create_agent_session(&self, client: Arc<dyn BotticelliDriver>) -> AgentSession {
        AgentSession {
            elicitation: AgentElicitation { client },
            // ... other state
        }
    }
}
```

### Phase 7: Test Both Protocols

**Human protocol test**:
```bash
# Interactive TUI session
cargo run --bin botticelli-mcp -- create-narrative
# Should prompt for each field interactively
```

**Agent protocol test**:
```rust
#[tokio::test]
async fn test_agent_elicitation() {
    let server = BotticelliServer::new()?;
    let agent_session = server.create_agent_session(llm_client);
    
    // Agent elicits through sampling
    let metadata = elicit_narrative_metadata(&agent_session).await?;
    
    assert!(!metadata.name.is_empty());
    assert!(!metadata.description.is_empty());
}
```

---

## Summary of Changes

1. **Remove custom param/result types, use elicitation's**:
   ```rust
   // OLD (delete)
   use crate::{ElicitTextParams, ElicitTextResult};
   pub async fn elicit_text(
       &self,
       Parameters(params): Parameters<ElicitTextParams>,
   ) -> Result<Json<ElicitTextResult>, rmcp::ErrorData> { /* ... */ }
   
   // NEW (use elicitation's MCP integration)
   pub async fn elicit_text(
       &self,
       params: serde_json::Value, // Generic params from MCP protocol
   ) -> Result<serde_json::Value, rmcp::ErrorData> {
       // Parse using elicitation::mcp::parse_string, etc.
       let prompt = elicitation::mcp::parse_string(
           params.get("prompt").ok_or(/* error */)?
       )?;
       
       let text = self.dialog()?.ask_text(&prompt).await?;
       
       Ok(serde_json::json!({ "value": text }))
   }
   ```

2. **Update tool registry to remove delegations**:
   ```rust
   // OLD (in tools/registry.rs)
   "elicit_text" => {
       let result = self.server.elicit_text(
           Parameters(serde_json::from_value(input)?)
       ).await?;
       // ...
   }
   
   // NEW (direct registration)
   // Primitives stay in BotticelliServer as #[tool] methods
   // Domain tools become standalone functions with derives:
   
   #[derive(elicitation::Elicit)]
   pub struct NarrativeMetadata {
       name: String,
       description: String,
       // ... auto-generated elicit_narrative_metadata() tool!
   }
   ```

---

## Benefits of Refactoring

### 1. Agent Autonomy

**Before**: Only humans can elicit data  
**After**: **Agents can elicit arbitrary complexity through sampling**

Example - agent autonomously creates narrative:
```rust
// Agent session automatically samples/reasons for each field
let metadata = elicit_narrative_metadata(&agent_session).await?;
let act1 = elicit_toml_act(&agent_session).await?;
let act2 = elicit_toml_act(&agent_session).await?;
let carousel = elicit_carousel_config(&agent_session).await?;

// Agent constructed entire narrative through reasoning!
```

### 2. Protocol Abstraction

**Before**: Hardcoded to DialogResource (humans only)  
**After**: Generic over Provider trait (humans + agents + future protocols)

### 3. Type System Guarantees

**Before**: Runtime branching, easy to miss cases  
**After**: Associated types enforce correct provider at compile time

### 4. Automatic Tool Generation

**Before**: Manual tool for each type
```rust
// Had to manually create ElicitMetadataParams/Result
pub struct ElicitMetadataParams { /* 10 fields */ }
pub struct ElicitMetadataResult { /* status */ }

// Had to manually implement
pub async fn elicit_metadata(...) { /* 40 lines */ }
```

**After**: Just derive
```rust
#[derive(elicitation::Elicit)]
pub struct NarrativeMetadata {
    name: String,
    description: String,
    default_model: Option<String>,
    // ... 7 more fields
}
// Auto-generates: elicit_narrative_metadata() tool!
```

### 5. Seamless Context Switching

**Before**: Different APIs for different contexts  
**After**: Same Elicit derives work everywhere

```rust
// One derive, multiple contexts
#[derive(elicitation::Elicit)]
pub struct WorkflowConfig { /* ... */ }

// Works in human TUI session
let config = WorkflowConfig::elicit(&human_session).await?;

// Works in agent reasoning session  
let config = WorkflowConfig::elicit(&agent_session).await?;

// Same code, different protocols!
```

### 6. Type Safety

**Before**: String-based param passing  
```rust
let params = ElicitTextParams { prompt: "Enter name".to_string() };
// Easy to typo field names, miss required fields
```

**After**: Strongly-typed elicitation
```rust
let metadata = NarrativeMetadata::elicit(&client).await?;
// Compile-time checked, impossible to miss fields
```

### 7. Consistency

**Before**: Two elicitation systems with different APIs  
- Handrolled: `ElicitTextParams` → `ElicitTextResult`
- Elicitation: `String::elicit()` → `String`

**After**: One system, one API  
- Everything: `Type::elicit()` → `Type`

### 8. Feature Parity

**Before**: Handrolled code missing features:
- ❌ No validation
- ❌ No retry logic
- ❌ No defaults
- ❌ No help text
- ❌ No style customization

**After**: Get all elicitation features free:
- ✅ Built-in validation
- ✅ Retry on invalid input
- ✅ Default values
- ✅ Prompt customization
- ✅ Multiple styles (Default, Compact, Verbose, Wizard)

---

## Risks and Mitigations

### Risk 1: Breaking Changes

**Risk**: Removing types breaks existing code  
**Mitigation**: 
- Search codebase for usage first
- Update callers to use elicitation types
- Verify with `cargo check`

### Risk 2: Primitive Tools Still Needed

**Risk**: Elicitation's auto-generated tools CALL our primitives  
**Mitigation**: 
- KEEP primitive implementations (elicit_text, etc.)
- Only remove the TYPES (ElicitTextParams/Result)
- Primitives now accept/return generic JSON values

### Risk 3: UI Integration

**Risk**: DialogResource might not work with new approach  
**Mitigation**: 
- DialogResource is unchanged (UI bridge stays)
- Only MCP layer changes (params/results)
- UI interaction method signatures unchanged

### Risk 4: Tool Registration

**Risk**: Tool router might not find primitives  
**Mitigation**: 
- Keep #[tool] attributes on primitive methods
- They're still on BotticelliServer
- Tool router discovers them normally

---

## Revised Implementation Timeline

### Phase 1: Design Provider Traits (1 hour)
```rust
// Design trait signatures
// Consider error types
// Plan session context types
```

### Phase 2: Implement HumanElicitation (1 hour)
```rust
// Wrap existing DialogResource
// Implement ElicitationProvider trait
// Test with existing TUI flows
```

### Phase 3: Implement AgentElicitation (2 hours)
```rust
// Implement sampling-based elicitation
// Parse agent responses (text, bool, number, select)
// Add reasoning prompts for each primitive
// Test with mock LLM client
```

### Phase 4: Refactor Server Tools (1.5 hours)
```rust
// Make tools generic over Provider
// Remove custom param/result types
// Update tool signatures
// Test with human sessions
```

### Phase 5: Add Agent Session Support (1 hour)
```rust
// Add session creation methods
// Wire up agent clients
// Add session context detection
```

### Phase 6: Delete Redundant Code (15 min)
```bash
git rm src/elicit_{text,bool,number,select}.rs
# Update lib.rs exports
```

### Phase 7: End-to-End Testing (2 hours)
```bash
# Test human protocol
# Test agent protocol  
# Test complex type elicitation in both contexts
# Verify all Elicit-derived types work
```

**Total: ~9 hours** (up from 3-4 due to agent protocol implementation)

### Original Phase 1: Audit (30 min)
```bash
# Find all usage of handrolled types
rg "ElicitTextParams|ElicitBoolParams|ElicitNumberParams|ElicitSelectParams" crates/

# Check imports
rg "use crate::.*elicit_(text|bool|number|select)" crates/

# Verify no external usage
rg "botticelli_mcp::.*(ElicitText|ElicitBool)" --type rust
```

### Original Phase 2: Refactor Tool Implementations (1 hour) - NOW SUPERSEDED BY NEW PHASES
```rust
// Update rmcp_server/tools/elicitation.rs
// Change from custom types to generic JSON handling

impl BotticelliServer {
    #[tool]
    pub async fn elicit_text(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        // Use elicitation::mcp helpers
        let prompt = elicitation::mcp::extract_value(params)?;
        let prompt = elicitation::mcp::parse_string(prompt)?;
        
        let text = self.dialog()?.ask_text(&prompt).await
            .map_err(|e| /* convert to ErrorData */)?;
        
        Ok(serde_json::json!({ "value": text }))
    }
    
    // Same for elicit_bool, elicit_integer, elicit_select
}
```

### Phase 3: Delete Redundant Files (5 min)
```bash
git rm crates/botticelli_mcp/src/elicit_text.rs
git rm crates/botticelli_mcp/src/elicit_bool.rs
git rm crates/botticelli_mcp/src/elicit_number.rs
git rm crates/botticelli_mcp/src/elicit_select.rs
```

### Phase 4: Update lib.rs Exports (5 min)
```rust
// Remove from src/lib.rs:
mod elicit_bool;
mod elicit_number;
mod elicit_select;
mod elicit_text;

pub use elicit_bool::{ElicitBoolParams, ElicitBoolResult};
pub use elicit_number::{ElicitNumberParams, ElicitNumberResult};
pub use elicit_select::{ElicitSelectParams, ElicitSelectResult};
pub use elicit_text::{ElicitTextParams, ElicitTextResult};
```

### Phase 5: Test (30 min)
```bash
cargo check -p botticelli_mcp
cargo test -p botticelli_mcp

# Test narrative creation workflow
cargo run --bin botticelli-mcp -- create-narrative
```

### Phase 6: Add Derives to Domain Types (1 hour)
```rust
// Update src/create_narrative.rs, src/modify_narrative.rs, etc.

#[derive(Debug, Clone, elicitation::Elicit)]
pub struct NarrativeMetadata {
    #[prompt("Enter narrative name:")]
    name: String,
    
    #[prompt("Enter description:")]
    description: String,
    
    #[prompt("Default model (optional):")]
    default_model: Option<String>,
}

// Auto-generates: elicit_narrative_metadata() tool!
```

---

## Expected Outcome

### Before (Current State)
- 489 lines of handrolled code
- Elicitation only works for humans (DialogResource)
- Agents cannot elicit - no protocol for sampling-based elicitation
- Each context needs different implementations
- Manual tools for everything

### After (Target State)
- ~550 lines total (+~180 new trait code, -122 redundant types)
- **Agents can seamlessly elicit arbitrary data through sampling**
- **Humans and agents use same types/tools through Provider abstraction**
- Type system enforces correct protocols (associated types)
- Auto-generated tools work in BOTH contexts
- All 159 Elicit-derived types automatically support both protocols

### Code Diff
```diff
- src/elicit_text.rs       (22 lines)
- src/elicit_bool.rs       (25 lines)
- src/elicit_number.rs     (27 lines)
- src/elicit_select.rs     (48 lines)
+ Use elicitation::mcp helpers in tool impls

  src/rmcp_server/tools/elicitation.rs:
- Parameters<ElicitTextParams>
+ serde_json::Value (generic params)

- ElicitTextResult::new(text)
+ serde_json::json!({ "value": text })
```

---

## Recommendation

**Proceed with Provider-based refactoring:**

### Why This Matters

This isn't just code cleanup - **it's enabling a new capability**: agent autonomy through elicitation.

**The Vision**: Agents that can construct arbitrarily complex types through sampling, using the same elicitation infrastructure humans use for interactive prompts.

### Priorities

1. **High Priority**: Implement Provider traits + HumanElicitation (enables trait-based architecture)
2. **Critical**: Implement AgentElicitation (unlocks agent autonomy)  
3. **Medium**: Refactor existing tools to be generic
4. **Low**: Delete redundant types (cleanup)

### Timeline

**Conservative**: 9 hours (includes agent protocol implementation + testing)  
**Optimistic**: 6 hours (if agent sampling is straightforward)

### Risks

**Medium Risk**: Agent protocol design
- Parsing agent responses may need iteration
- Sampling prompts need tuning for good results
- May need retry logic for invalid agent responses

**Low Risk**: Everything else
- Human protocol just wraps existing DialogResource
- Trait abstraction is straightforward
- Type system enforces correctness

### Decision

**Strongly Recommended** - This refactoring:
- ✅ Enables the core vision (agent autonomy)
- ✅ Maintains all existing functionality (humans work as before)
- ✅ Provides clean abstraction for future protocols
- ✅ Leverages Rust's type system for safety
- ✅ Makes all 159 Elicit types work in both contexts automatically

**Next Step**: Start with Phase 1 (trait design) to validate the architecture before full implementation.
