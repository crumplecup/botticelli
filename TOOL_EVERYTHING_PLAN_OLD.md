# Tool Everything: Universal MCP Exposure

## Philosophy

**Tool everything** means: If it's `pub`, it gets `#[tool]`.

- Not "should this be a tool?" → "Why wouldn't this be a tool?"
- Internal plumbing? **Tool it.**
- Getters/constructors? **Tool it.**
- CRUD operations? **Tool it.**
- Utilities? **Tool it.**

LLMs are power users who can orchestrate atomic operations into workflows.

## Architecture

### Current State
```
botticelli_mcp (32 tools via #[tool_router])
  ↓
Calls library functions
```

### Target State
```
botticelli_core         (#[tool] on all pub fn) → 22 tools
botticelli_storage      (#[tool] on all pub fn) → 2 tools
botticelli_database     (#[tool] on all pub fn) → 55 tools
botticelli_models       (#[tool] on all pub fn) → 38 tools
botticelli_rate_limit   (#[tool] on all pub fn) → ~5 tools
botticelli_security     (#[tool] on all pub fn) → ~3 tools
botticelli_cache        (#[tool] on all pub fn) → ~2 tools
botticelli_social       (#[tool] on all pub fn) → ~8 tools
         ↓
botticelli_mcp (#[tool_router] aggregates everything) → 135+ tools
         ↓
LLMs compose atomic operations into workflows
```

## Implementation Strategy

### Phase 1: Add rmcp Dependency (All Crates)

Add to each crate's `Cargo.toml`:
```toml
[dependencies]
rmcp = { workspace = true }
schemars = { workspace = true }  # For JsonSchema derives
```

**Crates to update**:
- ✅ botticelli_mcp (already has it)
- [ ] botticelli_core
- [ ] botticelli_storage
- [ ] botticelli_database
- [ ] botticelli_models
- [ ] botticelli_rate_limit
- [ ] botticelli_security
- [ ] botticelli_cache
- [ ] botticelli_social

### Phase 2: Add #[tool] to All Public Functions

**Pattern**:
```rust
use rmcp::tool;

/// Existing documentation stays the same
#[tool]
#[instrument]  // Keep existing instrumentation
pub fn function_name(params: Type) -> Result<ReturnType, Error> {
    // Implementation unchanged
}
```

**For async functions**:
```rust
#[tool]
#[instrument]
pub async fn async_function(params: Type) -> Result<ReturnType, Error> {
    // Implementation unchanged
}
```

**For methods**:
```rust
impl MyType {
    #[tool]
    #[instrument]
    pub fn method(&self, params: Type) -> Result<ReturnType, Error> {
        // Implementation unchanged
    }
}
```

### Phase 3: Add JsonSchema to Parameter/Return Types

All types used in tool signatures need `JsonSchema`:

```rust
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MyParams {
    field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MyResult {
    output: String,
}
```

**Existing derives stay** - just add `JsonSchema` to the list.

### Phase 4: Aggregate in botticelli_mcp

The `#[tool_router]` macro in botticelli_mcp automatically discovers and registers all tools from dependencies.

No manual registration needed!

## Crate-by-Crate Breakdown

### botticelli_core (22 functions → 22 tools)

**All public functions become tools**:
- Input/Output construction
- Message building  
- Role utilities
- Content serialization
- All getters/constructors

**Example**:
```rust
#[tool]
pub fn text_input(content: impl Into<String>) -> Input {
    Input::Text(content.into())
}

#[tool]
pub fn build_message(role: Role, content: Vec<Input>) -> Message {
    Message::builder().role(role).content(content).build()
}
```

### botticelli_database (55 functions → 55 tools)

**Everything becomes a tool**:
- Connection management: `establish_connection()`, `create_pool()`
- CRUD: `list_content()`, `insert_content()`, `update_content_metadata()`, `delete_content()`
- Queries: `query_content()`, `pull_and_delete()`, `promote_content()`
- Schema: `list_tables()`, `describe_table()`, `infer_schema()`
- Conversions: `rows_to_narrative_execution()`, `status_to_string()`
- Even utilities and internal helpers

**Example workflow LLMs can build**:
```
establish_connection(url)
  → list_tables()
  → describe_table("posts")
  → query_content("posts", "WHERE status = 'active'", 10)
  → get_content_by_id("posts", 123)
  → update_content_metadata(...)
  → promote_content(...)
```

### botticelli_models (38 functions → 38 tools)

**Everything becomes a tool**:
- Model selection: `ModelSelector::select()`
- Model navigation: `move_up()`, `move_down()`, `current()`
- Bounds checking: `ModelBounds::allows()`
- Metrics: `ModelMetrics::record_request()`
- Getters: `as_str()`, `provider()`, `supports_tools()`
- Classification: `classify_error()`

**Example workflow**:
```
list_models()
  → check_model_bounds(model_id)
  → select_model(strategy)
  → supports_tools(model_id)
  → record_request(model_id, tokens)
```

### botticelli_storage (2 functions → 2 tools)

**All storage operations**:
- Media storage/retrieval
- File system operations

### botticelli_rate_limit (5 functions → 5 tools)

**All rate limit operations**:
- Config loading: `load_config()`
- Tier info: `get_tier_info()` (already exposed)
- Rate checking
- Usage tracking

### botticelli_security (3 functions → 3 tools)

**All security operations**:
- Context creation
- Permission checks
- User validation

### botticelli_cache (2 functions → 2 tools)

**All caching operations**:
- Cache get/set
- Invalidation

### botticelli_social (8 functions → 8 tools)

**All social media operations**:
- Post creation
- Media upload
- Status retrieval

## Expected Result

**Before**: 32 MCP-specific tools  
**After**: 135+ tools spanning entire workspace

**LLM Capabilities**:
- Database workflows: connect → query → transform → update
- Model workflows: select → validate → execute → track metrics
- Content workflows: create → validate → store → publish
- Complex multi-step operations composed from atomic tools

## Implementation Order

### Sprint 1: Core Infrastructure (Week 1)
1. Add rmcp dependency to all 8 crates
2. Add `use rmcp::tool;` imports
3. Verify compilation (no tools added yet)

### Sprint 2: Database Tools (Week 1-2)
1. Add JsonSchema to all database types
2. Add #[tool] to all 55 database functions
3. Test tool discovery in botticelli_mcp
4. Verify 55 database tools registered

### Sprint 3: Models Tools (Week 2)
1. Add JsonSchema to all model types
2. Add #[tool] to all 38 model functions
3. Verify 38 model tools registered

### Sprint 4: Core Tools (Week 2-3)
1. Add JsonSchema to core types (Input, Output, Message, Role)
2. Add #[tool] to all 22 core functions
3. Verify 22 core tools registered

### Sprint 5: Remaining Crates (Week 3)
1. Storage (2 tools)
2. Rate Limit (5 tools)
3. Security (3 tools)
4. Cache (2 tools)
5. Social (8 tools)

### Sprint 6: Integration & Documentation (Week 3-4)
1. Comprehensive testing of tool composition
2. Update MCP documentation with full tool catalog
3. Create example workflows demonstrating tool chains
4. Performance testing with 135+ tools

## Technical Requirements

### Every Tool Must Have

1. ✅ `#[tool]` attribute
2. ✅ `#[instrument]` for observability
3. ✅ JsonSchema on all parameter/return types
4. ✅ Proper documentation (existing docs stay)
5. ✅ Elicit derives where applicable

### Type Requirements

```rust
// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ToolParams {
    field: String,
}

// Return types  
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ToolResult {
    output: String,
}
```

### Error Handling

Tools return `Result<T, ErrorType>`:
- Database tools → `DatabaseResult<T>`
- Model tools → `ModelsResult<T>`
- Core tools → `CoreResult<T>`

MCP layer (botticelli_mcp) handles error conversion to `rmcp::ErrorData`.

## Benefits

### For LLMs
- **Atomic operations**: Build complex workflows from simple tools
- **Discovery**: All library functionality is discoverable via MCP
- **Composition**: Chain tools like Unix pipes
- **Flexibility**: Mix high-level and low-level operations

### For Developers
- **No wrapper boilerplate**: `#[tool]` is one line per function
- **Type safety**: JsonSchema catches mismatches at tool boundaries
- **Observability**: All tools instrumented by default
- **Consistency**: Same pattern everywhere

### For botticelli
- **Maximum exposure**: Entire library accessible via MCP
- **Ecosystem growth**: Third-party tools can compose with ours
- **Future-proof**: New functions automatically become tools
- **Competitive advantage**: Most comprehensive MCP tool library

## Success Metrics

- **Tool count**: 135+ tools registered
- **Coverage**: 100% of public functions are tools
- **Compilation**: Zero errors, zero warnings
- **Tests**: All existing tests pass
- **Performance**: Tool discovery < 100ms
- **Documentation**: Every tool has clear description

## Philosophy Summary

**"Tool Everything" means**:
- Default to YES for making something a tool
- Trust LLMs to use tools intelligently
- Expose atomic operations for maximum composability
- Let usage patterns emerge rather than predicting them
- The only bad tool is an undiscovered function

**Result**: The most comprehensive, composable, LLM-friendly library in the Rust ecosystem.

🚀 **Let's tool everything.** 🚀
