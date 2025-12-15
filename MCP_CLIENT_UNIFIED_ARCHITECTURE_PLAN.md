# MCP Client Unified Architecture Plan

**Status:** Phase 1 - Type System Unification (Step 1.1 Complete)  
**Started:** 2025-12-15  
**Goal:** Eliminate type duplication, leverage pmcp types, complete TODO implementations

---

## Executive Summary

- **Problem:** Duplicate types between our code and pmcp library
- **Solution:** Use pmcp types directly, eliminate conversions
- **Benefit:** Type safety, validation, less code, better maintenance
- **Risk:** Breaking changes, but pre-1.0 and deprecated aliases help
- **Timeline:** Phase 1 this session (careful, methodical)

---

## Current State

### What We Have

```
botticelli_mcp/           # Server ✅ COMPLETE (uses pmcp::Server)
├── pmcp_server.rs
├── pmcp_adapters.rs
├── pmcp_middleware.rs
└── tools/

botticelli_mcp_client/    # Client ⚠️ NEEDS WORK
├── external_client.rs    # ✅ Uses pmcp::Client
├── tool_executor.rs      # ⚠️ Custom types + TODOs
├── schema/               # ⚠️ Duplicate provider schemas
├── client.rs             # ⚠️ Needs audit
├── llm_adapter.rs        # ⚠️ Needs audit
└── context.rs            # ⚠️ Needs audit
```

### Type Duplication Problem

```rust
// OUR TYPE (tool_executor.rs)
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,  // ⚠️ Unvalidated JSON
}

// PMCP TYPE (battle-tested library)
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: JsonSchema,  // ✅ Validated!
}
```

**Impact:**
- `external_client.rs:191-199` converts pmcp → ours
- `external_client.rs:276-281` converts Content → raw JSON
- Schema validation lost
- Type safety reduced
- More code to maintain

---

## Phase 1: Type System Unification

### Step 1.1: Audit ✅ COMPLETE

**Type Mapping:**

| Our Type | pmcp Type | Strategy |
|----------|-----------|----------|
| `ToolDefinition` | `pmcp::types::Tool` | Replace |
| `ToolSchema` | `pmcp::types::Tool` | Deprecate |
| Tool results (Value) | `pmcp::types::Content` | Structured |
| Schemas (Value) | `pmcp::types::JsonSchema` | Validated |

**Conversion Points:**
1. `external_client.rs:191-199` - Tool conversion
2. `external_client.rs:276-281` - Content → Value
3. `schema/*` modules - Provider conversions
4. `tool_executor.rs` - Returns Value not Content

---

### Step 1.2: Replace ToolDefinition ⏳ NEXT

**Goal:** Use `pmcp::types::Tool` everywhere

**Changes:**

1. **tool_executor.rs**
   ```rust
   // Remove ToolDefinition struct
   // Change: HashMap<String, ToolDefinition>
   //     To: HashMap<String, Tool>
   use pmcp::types::Tool;
   ```

2. **external_client.rs**
   ```rust
   // Remove conversion (lines 191-199)
   // Before:
   let tools: Vec<ToolDefinition> = tools_result.tools
       .into_iter()
       .map(|t| ToolDefinition { ... })
       .collect();
   
   // After:
   let tools = tools_result.tools;  // Direct!
   ```

3. **lib.rs**
   ```rust
   // Export pmcp type
   pub use pmcp::types::Tool;
   
   // Deprecated alias
   #[deprecated(since = "0.3.0", note = "Use pmcp::types::Tool")]
   pub type ToolDefinition = pmcp::types::Tool;
   ```

4. **schema/mod.rs + providers**
   ```rust
   // Update trait
   pub trait ToolSchemaConverter {
       type Output;
       fn convert(tool: &pmcp::types::Tool) -> Self::Output;
   }
   
   // Update all: anthropic.rs, gemini.rs, openai.rs, etc.
   ```

**Test:**
```bash
just check-package botticelli_mcp_client
just test-package botticelli_mcp_client
```

**Acceptance:**
- ✅ No ToolDefinition struct
- ✅ No pmcp::Tool conversions
- ✅ All tests pass
- ✅ Deprecation alias works

---

### Step 1.3: Structured Content

**Goal:** Use `Vec<Content>` not `Value` for tool results

**Changes:**

1. **external_client.rs**
   ```rust
   // Remove lines 276-281 (serialization to Value)
   // Return Content directly
   pub async fn call_tool(...) -> McpClientResult<Vec<Content>> {
       // ...
       Ok(result.content)  // Direct!
   }
   ```

2. **tool_executor.rs**
   ```rust
   use pmcp::types::{Content, TextContent};
   
   pub async fn execute(...) -> McpClientResult<Vec<Content>> {
       // Return structured content
       Ok(vec![Content::Text(TextContent {
           text: format!("Tool {} executed", tool_name),
           annotations: None,
       })])
   }
   ```

3. **content_helpers.rs** (new file)
   ```rust
   use pmcp::types::*;
   
   pub fn extract_text(content: &[Content]) -> String {
       content.iter()
           .filter_map(|c| match c {
               Content::Text(t) => Some(t.text.clone()),
               _ => None,
           })
           .join("\n")
   }
   
   pub fn extract_resources(content: &[Content]) -> Vec<ResourceContent> { ... }
   pub fn extract_images(content: &[Content]) -> Vec<ImageContent> { ... }
   pub fn has_errors(content: &[Content]) -> bool { ... }
   ```

4. **lib.rs**
   ```rust
   pub use content_helpers::*;
   pub use pmcp::types::{Content, TextContent, ImageContent, ResourceContent};
   ```

**Test:**
```bash
just test-package botticelli_mcp_client
```

**Acceptance:**
- ✅ All tool results are Vec<Content>
- ✅ Helper functions work
- ✅ Tests updated
- ✅ Type safety improved

---

### Step 1.4: JsonSchema Validation

**Goal:** Use validated `JsonSchema` not raw `Value`

**Research Needed:**
```bash
cargo doc --package pmcp --no-deps --open
# Check JsonSchema API
```

**Changes:**

1. **Construction**
   ```rust
   // Before: let schema = json!({ ... });
   // After:  let schema = JsonSchema::try_from(json!({ ... }))?;
   ```

2. **Validation**
   ```rust
   // Schema validation happens at construction
   // Invalid schemas caught early
   ```

3. **Errors**
   ```rust
   // error.rs
   #[display("Invalid JSON schema: {}", _0)]
   InvalidSchema(String),
   ```

**Test:**
```bash
just test-package botticelli_mcp_client
# Include invalid schema tests
```

**Acceptance:**
- ✅ JsonSchema used everywhere
- ✅ Invalid schemas caught early
- ✅ Good error messages

---

## Phase 2: Complete TODOs

(After Phase 1 complete)

Key areas:
- Implement tool_executor actual execution
- Complete client.rs orchestration
- Review llm_adapter.rs (keep/remove?)
- Review context.rs (keep/use ConversationSession?)

See: `MCP_CLIENT_TODO_COMPLETION_PLAN.md` (to be created)

---

## Phase 3: Integration Tests

(After Phase 1 & 2 complete)

Test with real MCP servers:
- @modelcontextprotocol/server-filesystem
- Full end-to-end validation
- Error handling
- Performance

Feature-gated: `#[cfg_attr(not(feature = "integration"), ignore)]`

---

## Checklist

### Phase 1: Type System ⏳ IN PROGRESS
- [x] Step 1.1: Type audit (this document)
- [ ] Step 1.2: Replace ToolDefinition with pmcp::Tool
  - [ ] Update tool_executor.rs
  - [ ] Update external_client.rs
  - [ ] Update lib.rs with deprecation
  - [ ] Update schema/* modules
  - [ ] Tests pass
- [ ] Step 1.3: Structured Content
  - [ ] Update external_client.rs
  - [ ] Update tool_executor.rs
  - [ ] Create content_helpers.rs
  - [ ] Update lib.rs
  - [ ] Tests pass
- [ ] Step 1.4: JsonSchema validation
  - [ ] Research pmcp API
  - [ ] Update construction
  - [ ] Add validation
  - [ ] Tests pass
- [ ] **Commit:** "refactor(mcp_client): unify types with pmcp"

### Phase 2: TODO Completion
- [ ] See separate plan document
- [ ] **Commit:** "feat(mcp_client): complete TODO implementations"

### Phase 3: Integration Testing
- [ ] Filesystem server test
- [ ] Documentation
- [ ] **Commit:** "test(mcp_client): add integration tests"

---

## Risk Assessment

**Low Risk** ✅
- Compile-time type safety
- Deprecation aliases
- Test coverage
- pmcp is mature/battle-tested

**Medium Risk** ⚠️
- Breaking API changes (mitigated by deprecation)
- pmcp version changes (pin: `pmcp = "=1.8.6"`)

**High Risk** ��
- External server compat (mitigate: integration tests)
- Schema strictness (mitigate: good errors)

---

## Success Metrics

**Phase 1:**
- Zero type conversions in external_client.rs
- All schema converters use pmcp::Tool
- Content types everywhere
- 100% test pass
- ~100-150 LOC reduction

**Phase 2:**
- All TODOs resolved
- Full execution pipeline
- Clear patterns

**Phase 3:**
- Real servers work
- Integration tests pass
- Production-ready

---

## Ready to Proceed

**Next:** Step 1.2 - Replace ToolDefinition

This is careful, methodical work. Each step:
1. Make minimal changes
2. Run tests
3. Verify
4. Move to next

No rushing. Getting it right matters more than speed.

**Questions before starting?**
