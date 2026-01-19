# MCP Elicitation Refactor Analysis

## Executive Summary

The `botticelli_mcp` crate has **TWO distinct elicitation systems**:

1. **Old/Legacy**: Custom hand-rolled elicitation using dialog prompts (~1100 lines)
2. **New/Modern**: Using the `elicitation` crate with derives (not yet fully integrated)

**Status**: We're in a **hybrid state** - the elicitation crate is added but not actually used. The old system still handles all elicitation.

**KEY ARCHITECTURAL INSIGHT**: The `#[tool_router]` macro automatically exposes all `pub async fn` methods on `BotticelliServer` as MCP tools. This means:
- Every method = automatic tool registration
- Tool composition = LLMs can chain tools like scripts
- Universal tooling = Everything should be a tool

---

## rmcp Tool Architecture

### Current Pattern

```rust
// src/rmcp_server/tools/mod.rs
#[tool_router]
impl BotticelliServer {}

// src/rmcp_server/tools/narrative.rs
impl BotticelliServer {
    pub async fn create_narrative(
        &self,
        Parameters(params): Parameters<CreateNarrativeParams>,
    ) -> Result<Json<CreateNarrativeResult>, rmcp::ErrorData> {
        // Implementation
    }
}
```

**What `#[tool_router]` does**:
- Scans all `pub async fn` on `BotticelliServer`
- Automatically registers them as MCP tools
- Generates tool schemas from Params types (via JsonSchema)
- No manual tool registration needed!

### Current Tool Count: **27 tools**

Organized in modules:
- `core.rs` - 4 tools (echo, server_info, query_content, export_metrics)
- `narrative.rs` - 8 tools (create, modify, save, validate, etc.)
- `scene.rs` - 4 tools (create, list, update, delete)
- `elicitation.rs` - 5 tools (elicit_bool, elicit_text, elicit_number, elicit_select, elicit_carousel)
- `discord.rs` - 3 tools (post_message, get_messages, get_guild_info, get_channels)
- `execution/*.rs` - 3 tools (generate, execute_act, execute_narrative)

### Tool Composition Vision

**Goal**: Every meaningful function becomes a tool that LLMs can:
1. **Call directly** - Single tool invocation
2. **Chain together** - Multi-step workflows
3. **Compose scripts** - Complex automation

**Example Workflow (LLM scripting)**:
```
1. create_narrative(description) → narrative_id
2. validate_narrative(narrative_id) → validation_result
3. apply_validation_fixes(narrative_id) → fixed
4. save_narrative(narrative_id, path) → saved
5. execute_narrative(narrative_id) → results
```

LLMs can now write "scripts" by chaining tools!

---

## Detailed Breakdown

### 🔴 OLD/LEGACY (Must Refactor or Remove)

#### 1. **Basic Elicit Tools** (src/elicit_*.rs - 4 files)
**Purpose**: Simple MCP tools for eliciting primitives via dialog prompts

Files:
- `src/elicit_bool.rs` (26 lines)
- `src/elicit_text.rs` (23 lines)
- `src/elicit_number.rs` (37 lines)
- `src/elicit_select.rs` (42 lines)

**Assessment**: 
- ✅ **KEEP TYPES** - Params/Result structs are MCP tool DTOs
- 🔄 **REFACTOR IMPLS** - Current impl uses dialog.ask_*(), should use `elicitation` crate
- These are thin wrappers that should delegate to `elicitation::Elicit` trait

**Refactor Strategy**:
```rust
// OLD (current):
let text = dialog.ask_text(&prompt).await?;

// NEW (with elicitation crate):
let text = String::elicit(&elicit_client).await?;
```

---

#### 2. **Session-Based Elicitation** (src/session_tools.rs - 756 lines!)
**Purpose**: Multi-step narrative creation workflow

**Tools Defined**:
- `create_narrative_session` - Initialize session from description
- `elicit_metadata` - Gather name/description
- `elicit_act` - Build act config step-by-step
- `elicit_carousel` - Configure carousel budgets
- `finalize_narrative` - Save completed narrative
- `get_narrative_state` - Query current state
- `validate_narrative_session` - Check completeness
- `apply_validation_fixes` - Auto-fix issues

**Assessment**:
- ✅ **KEEP TOOL STRUCTURE** - The multi-step workflow makes sense
- 🔄 **REFACTOR IMPLS** - Currently hand-coded, should use `elicitation` crate
- 📦 **TYPES CAN GET ELICIT** - All Params/Result structs are candidates

**Key Insight**: 
- **PartialNarrative** / **PartialAct** (src/partial.rs) could use `#[derive(Elicit)]`
- Session state management is valuable - keep the state machine, refactor the elicitation

---

#### 3. **Tools/Elicitation Module** (src/tools/elicitation/ - ~400 lines)
**Purpose**: Helpers and registry for old elicitation system

Files:
- `helpers.rs` - String analysis, prompt generation
- `registry.rs` - Generic trait-based registry pattern
- `state.rs` - Narrative state tracking
- `validation.rs` - Validation and auto-fixes

**Assessment**:
- ❌ **REMOVE** - These are infrastructure for the old system
- The **registry pattern** is interesting but now obsolete
- **Validation logic** should move to `botticelli_narrative` crate
- **State tracking** can be simplified with derive macros

---

#### 4. **RMCP Server Elicitation** (src/rmcp_server/tools/elicitation.rs - 600 lines)
**Purpose**: rmcp server implementations of elicit tools

**Assessment**:
- 🔄 **REFACTOR** - Keep the tool registration, refactor internals
- Uses `DialogResource` to interact with user
- Should integrate with `elicitation` crate's client interface

---

### 🟢 FINE AS-IS (Keep)

#### 1. **DialogResource** (src/dialog_resource.rs)
**Purpose**: rmcp resource for interactive prompts

**Assessment**:
- ✅ **KEEP** - This is the rmcp integration layer
- Provides `ask_text()`, `ask_confirmation()`, etc.
- Can be **adapted** to work with `elicitation` crate's traits

---

#### 2. **ConversationSession** (src/conversation.rs)
**Purpose**: Track conversation state for elicitation

**Assessment**:
- ✅ **KEEP** - Session management is still needed
- Could be simplified, but the pattern is sound
- `SessionState` enum tracks current state

---

#### 3. **Tool Types** (Many Params/Result structs)
**Purpose**: MCP tool input/output DTOs

**Assessment**:
- ✅ **KEEP ALL** - These are API contracts
- 📦 **ADD ELICIT** - Many can derive Elicit for construction
- They're the bridge between MCP protocol and our logic

---

### 🟡 CAN ADD ELICIT (Enhancement)

#### Types That Should Derive Elicit

**Basic Tool Types**:
- ✅ `ElicitBoolParams` - Simple prompt + default
- ✅ `ElicitTextParams` - Prompt + optional default/placeholder
- ✅ `ElicitNumberParams` - Prompt + min/max range
- ✅ `ElicitSelectParams` - Prompt + options list

**Session Tool Types**:
- ✅ `CreateNarrativeSessionParams` - Just a description string
- ✅ `ElicitMetadataParams` - Name + description (optional fields)
- ✅ `ElicitActParams` - Act name + session ID
- ✅ `ElicitCarouselParams` - Session ID + level enum

**Execution Tool Types**:
- ✅ `GenerateParams` - LLM generation config
- ✅ `ExecuteActParams` - Act execution config
- ✅ `ExecuteNarrativeParams` - Narrative execution config

**Narrative Tool Types**:
- ✅ `CreateNarrativeParams` - Narrative TOML creation
- ✅ `ModifyNarrativeParams` - Narrative modification
- ✅ `SaveNarrativeParams` - Save path + content
- ✅ `ValidateNarrativeParams` - Validation config

**Scene Tool Types**:
- ✅ `CreateSceneParams` - Scene creation
- ✅ `UpdateSceneParams` - Scene updates
- ✅ `DeleteSceneParams` - Scene deletion

**Partial Types**:
- ✅ `PartialAct` - In-progress act definition
- ✅ `PartialNarrative` - In-progress narrative definition

**Discord Tool Types**:
- ✅ `DiscordPostMessageParams` - Message sending
- ✅ `DiscordGetMessagesParams` - Message retrieval
- ✅ `DiscordGetGuildInfoParams` - Guild info query
- ✅ `DiscordGetChannelsParams` - Channel listing

**Enums**:
- ✅ `MetricsFormat` - Text/JSON/Prometheus
- ✅ `StateFormat` - Summary/Detailed/Raw
- ✅ `CarouselLevel` - Narrative/Act/Step
- ✅ `ValidationSeverity` - Error/Warning/Info

**Result Types** (maybe - LLMs can understand results):
- 🤔 All *Result types - Query results, execution summaries
- 🤔 Validation/metrics types - Read-only but useful for LLM understanding

---

## Universal Tooling Strategy

### Phase 0: Elicit Derives + Universal Tooling (Current)

**Goal 1**: Make all config types Elicit-able
- ✅ Add `#[derive(Elicit)]` to all Params/Result structs
- ✅ Add Elicit to enums (MetricsFormat, StateFormat, etc.)
- ✅ LLMs can construct all MCP tool parameters

**Goal 2**: Tool-out the workspace
- 🎯 Every public method → MCP tool (via `#[tool_router]`)
- 🎯 Expose library functions as tools
- 🎯 Enable tool composition and scripting

**Why This Matters**:
1. **LLM Construction** - LLMs can build Params with `Elicit` derives
2. **LLM Orchestration** - LLMs can chain tools into workflows
3. **Human Scripts** - Developers can compose tools programmatically
4. **Testing** - Tools become testable units

### Candidates for Tooling

**botticelli_narrative** (expose as tools):
- `validate_narrative_toml(toml: &str) -> ValidationResult`
- `parse_narrative_file(path: &Path) -> Narrative`
- `execute_act(narrative, act_name) -> ActResult`
- `load_state(narrative, scope) -> State`

**botticelli_database** (expose as tools):
- `query_content(table, filters) -> Vec<Row>`
- `insert_content(table, data) -> InsertResult`
- `list_tables() -> Vec<String>`
- `infer_schema(json) -> InferredSchema`

**botticelli_models** (expose as tools):
- `select_model(criteria) -> ModelId`
- `list_models(family) -> Vec<Model>`
- `get_model_metadata(id) -> Metadata`

**Strategy**:
1. Start with high-value functions (validation, querying, execution)
2. Add `pub async fn` wrappers in `BotticelliServer`
3. Automatically registered as tools by `#[tool_router]`
4. LLMs can now call library functions as tools!

---

## Refactor Plan

### Phase 0: Elicit + Universal Tooling (This PR)
**Goal**: Make LLMs able to construct all config/param types

**Tasks**:
1. ✅ Add `elicitation.workspace = true` to Cargo.toml
2. Add `elicitation::Elicit` to all Params structs
3. Add Elicit to enums (MetricsFormat, StateFormat, etc.)
4. Add Elicit to PartialAct, PartialNarrative
5. Verify compilation, fix unused imports

**Benefit**: LLMs can construct all MCP tool parameters programmatically

---

### Phase 2: Refactor Basic Elicit Tools
**Goal**: Replace hand-coded dialog with elicitation crate

**Strategy**:
```rust
// OLD impl (src/rmcp_server/tools/elicitation.rs):
pub async fn elicit_text(&self, params: ElicitTextParams) -> Result<ElicitTextResult> {
    let dialog = self.dialog().as_ref().ok_or(...)?;
    let text = dialog.ask_text(params.prompt()).await?;
    Ok(ElicitTextResult::new(text))
}

// NEW impl:
pub async fn elicit_text(&self, params: ElicitTextParams) -> Result<ElicitTextResult> {
    let client = self.elicit_client(); // ElicitClient wraps dialog
    let text = String::elicit(&client).await?;
    Ok(ElicitTextResult::new(text))
}
```

**Tasks**:
1. Create `ElicitClient` wrapper around `DialogResource`
2. Implement `Prompt` trait for DialogResource
3. Replace manual prompting with trait method calls
4. Keep tool structure, refactor internals only

---

### Phase 3: Refactor Session-Based Elicitation
**Goal**: Use elicitation derives for narrative construction

**Strategy**:
```rust
// OLD impl:
pub async fn elicit_metadata(&self, params: ElicitMetadataParams) -> Result {
    let dialog = self.dialog().as_ref().ok_or(...)?;
    let name = dialog.ask_text("Enter narrative name:").await?;
    let desc = dialog.ask_text("Enter description:").await?;
    // ... manual field gathering
}

// NEW impl:
pub async fn elicit_metadata(&self, params: ElicitMetadataParams) -> Result {
    let client = self.elicit_client();
    let metadata = NarrativeMetadata::elicit(&client).await?;
    // Derives handle prompting automatically!
}
```

**Tasks**:
1. Add `#[derive(Elicit)]` to PartialNarrative, PartialAct
2. Use generated `elicit()` methods instead of manual prompting
3. Keep session state management
4. Simplify tool implementations dramatically

---

### Phase 4: Clean Up Legacy Code
**Goal**: Remove obsolete elicitation infrastructure

**Remove**:
- ❌ `src/tools/elicitation/helpers.rs` - String analysis (replaced by elicitation crate)
- ❌ `src/tools/elicitation/registry.rs` - Generic registry (not needed)
- ❌ Manual validation code - Move to botticelli_narrative

**Keep**:
- ✅ `src/session_tools.rs` - Session management still valuable
- ✅ `src/conversation.rs` - ConversationSession tracking
- ✅ `src/dialog_resource.rs` - rmcp resource interface

---

## Key Architectural Decisions

### 1. Two-Layer Elicitation

**MCP Layer** (Keep):
- MCP tools expose elicitation capabilities
- JSON-based parameter passing
- Tool result types

**Elicitation Layer** (Refactor):
- Use `elicitation` crate for actual prompting
- Derive macros for automatic field collection
- Trait-based abstraction

### 2. DialogResource as Prompt Implementation

```rust
impl elicitation::Prompt for DialogResource {
    async fn prompt(&self, message: &str) -> Result<String> {
        self.ask_text(message).await
    }
    
    async fn select(&self, message: &str, options: &[String]) -> Result<usize> {
        // Map to MCP select tool
    }
}
```

### 3. Session State Management

Keep the session-based workflow:
- `create_narrative_session` - Initialize
- `elicit_metadata` - Gather metadata
- `elicit_act` - Add acts
- `finalize_narrative` - Complete

But simplify internals using elicitation derives.

---

## Code Volume Reduction

**Current (Legacy)**:
- `src/elicit_*.rs`: 128 lines
- `src/session_tools.rs`: 756 lines
- `src/tools/elicitation/`: ~400 lines
- `src/rmcp_server/tools/elicitation.rs`: 600 lines
- **Total**: ~1,900 lines of elicitation code

**After Refactor (Estimate)**:
- Tool impls: ~400 lines (refactored, simplified)
- Session management: ~300 lines (kept but simplified)
- Dialog integration: ~100 lines (ElicitClient wrapper)
- **Total**: ~800 lines (58% reduction!)
- **Removed**: ~1,100 lines of manual prompting/validation

---

## Risk Assessment

### Low Risk
- ✅ Adding Elicit derives (Phase 1)
- ✅ Types are already well-structured
- ✅ No breaking changes to MCP API

### Medium Risk
- 🔄 Refactoring basic elicit tools (Phase 2)
- Need to test dialog → elicitation bridge
- MCP tool behavior must stay identical

### High Risk
- ⚠️ Refactoring session-based elicitation (Phase 3)
- Complex state machine logic
- Multi-step workflows
- Requires careful testing

### Must Test
- Session state transitions
- Error handling in elicitation
- MCP tool compatibility
- Dialog resource integration

---

## Recommendations

### Immediate Actions (This PR)
1. ✅ Add Elicit to all Params/config types
2. ✅ Verify compilation
3. ✅ Document new capabilities

### Next PR (Refactor Phase 2)
1. Create ElicitClient wrapper
2. Refactor basic elicit tools
3. Test MCP tool compatibility
4. Keep legacy code until verified

### Future PR (Refactor Phase 3)
1. Use Elicit derives for session tools
2. Simplify session management
3. Remove legacy elicitation code
4. Update documentation

### Never Do
- ❌ Break MCP tool API
- ❌ Remove session management
- ❌ Delete DialogResource
- ❌ Rush the refactor

---

## Summary

**Current State**:
- Hybrid: elicitation crate added but unused
- ~1,900 lines of manual elicitation code
- Complex hand-coded prompting logic

**Target State**:
- Full elicitation crate integration
- ~800 lines of clean integration code
- Derive macros for automatic elicitation
- 58% code reduction

**Strategy**:
- Phase 1: Add derives (safe, immediate value)
- Phase 2: Refactor basic tools (medium risk)
- Phase 3: Refactor session tools (high value, careful testing)
- Phase 4: Clean up legacy (once verified)

**Timeline**:
- Phase 1: This PR
- Phase 2: Next sprint
- Phase 3: Future sprint
- Phase 4: After full verification
