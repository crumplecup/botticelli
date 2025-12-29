# RMCP Tool Migration Plan

## Overview

We have successfully migrated the core infrastructure from pmcp to rmcp. Now we need to migrate the existing 36 tool files from the old `McpTool` trait pattern to the new rmcp `#[tool]` macro pattern.

## Current State

### Old Pattern (tools/*.rs - McpTool trait)
```rust
use async_trait::async_trait;
use crate::tools::McpTool;

pub struct EchoTool;

#[async_trait]
impl McpTool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "..." }
    fn input_schema(&self) -> Value { json!({...}) }
    async fn execute(&self, input: Value) -> McpResult<Value> { ... }
}
```

### New Pattern (rmcp_server.rs - rmcp macros)
```rust
#[tool_router]
impl BotticelliServer {
    #[tool(description = "...")]
    #[instrument(skip(self))]
    pub async fn echo(
        &self,
        Parameters(EchoParams { message }): Parameters<EchoParams>
    ) -> Result<Json<EchoResult>, rmcp::ErrorData> {
        // Implementation
    }
}
```

## Key Differences

| Aspect | Old (McpTool) | New (rmcp) |
|--------|---------------|------------|
| **Type Safety** | `Value` (JSON) | Strongly-typed Rust structs |
| **Schema** | Manual JSON | `JsonSchema` derive |
| **Errors** | `McpError` | `rmcp::ErrorData` |
| **Registration** | Manual trait impl | `#[tool]` macro |
| **Parameters** | JSON extraction | Pattern matching |
| **Return** | `Value` | `Json<T>` wrapper |
| **State** | Separate tool struct | Methods on server |

## Benefits of New Pattern

1. **Compiler Validation** - Types checked at compile time
2. **Better IDE Support** - Auto-completion, refactoring
3. **Less Boilerplate** - Macro generates schemas
4. **Cleaner Errors** - Type mismatches caught early
5. **Self-Documenting** - Types = documentation
6. **Test Friendly** - Direct method calls, no JSON

## Migration Strategy

### Phase 1: Core Tools (Foundation)
Migrate the simplest tools that have no dependencies:

1. ✅ **echo** - Already migrated (Step 5)
2. ✅ **server_info** - Already migrated (Step 7)
3. **database/query_content** - Simple database query
4. **export_metrics** - Stateless metrics export

### Phase 2: Elicitation Primitives
Migrate the basic elicitation tools that dialog resource needs:

5. **elicit_text** - Text input primitive
6. **elicit_bool** - Boolean input primitive
7. **elicit_number** - Number input primitive
8. **elicit_select** - Selection primitive

### Phase 3: Narrative Core
Migrate narrative creation and validation:

9. **create_narrative** - Start new narrative
10. **validate_narrative** - Validate narrative structure
11. **save_narrative** - Persist narrative
12. **get_narrative_state** - Query narrative state
13. **modify_narrative** - Update narrative

### Phase 4: Execution
Migrate execution-related tools:

14. **execute_act** - Execute single act
15. **execute_narrative** - Execute full narrative

### Phase 5: LLM Integration
Migrate generation tools (require botticelli_models):

16. **generate_gemini** - Gemini generation
17. **generate_anthropic** - Claude generation
18. **generate_ollama** - Ollama generation
19. **generate** - Generic generation wrapper

### Phase 6: Complex Features
Migrate advanced features:

20. **Discord tools** (4 tools) - If discord feature enabled
21. **Metrics/Prometheus** - Observability
22. **Bot commands** - Command handling

## Per-Tool Migration Steps

For each tool:

### 1. Create Types Module
```rust
// src/tool_name.rs
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolNameParams {
    pub field1: String,
    // All fields public for rmcp
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ToolNameResult {
    pub output: String,
}

impl ToolNameResult {
    pub fn new(output: String) -> Self {
        Self { output }
    }
}
```

### 2. Add to Server
```rust
// src/rmcp_server.rs
#[tool_router]
impl BotticelliServer {
    #[tool(description = "Tool description")]
    #[instrument(skip(self))]
    pub async fn tool_name(
        &self,
        Parameters(ToolNameParams { field1 }): Parameters<ToolNameParams>
    ) -> Result<Json<ToolNameResult>, rmcp::ErrorData> {
        // Implementation
        Ok(Json(ToolNameResult::new(result)))
    }
}
```

### 3. Export Types
```rust
// src/lib.rs
mod tool_name;
pub use tool_name::{ToolNameParams, ToolNameResult};
```

### 4. Write Tests
```rust
// tests/tool_name_test.rs
use botticelli_mcp::{BotticelliServer, ToolNameParams};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_tool_name() {
    let server = BotticelliServer::builder().build();
    let params = ToolNameParams { field1: "test".to_string() };
    
    let result = server.tool_name(Parameters(params))
        .await
        .expect("Should succeed");
    
    assert_eq!(result.0.output, "expected");
}
```

### 5. Delete Old Implementation
```bash
# After tests pass:
rm src/tools/tool_name.rs
# Update src/tools/mod.rs to remove old exports
```

## Dependencies to Handle

### Dialog Resource
- Used by elicitation tools
- May need to be passed to server via builder
- Pattern: `dialog: Option<Arc<DialogResource>>`

### Database
- Used by query_content tool
- Requires database feature
- Pattern: `database: Option<Arc<Database>>`

### LLM Clients
- Used by generation tools
- Requires respective feature flags
- Pattern: `gemini_client: Option<Arc<GeminiClient>>`

### Narrative Registry
- Used by narrative tools
- Stateful registry needs Arc<Mutex<...>> or similar
- Pattern: `registry: Arc<RwLock<NarrativeRegistry>>`

## Server State Management

The BotticelliServer will grow as we add dependencies:

```rust
pub struct BotticelliServer {
    tool_router: ToolRouter<Self>,
    
    // Optional dependencies (feature-gated)
    dialog: Option<Arc<DialogResource>>,
    
    #[cfg(feature = "database")]
    database: Option<Arc<Database>>,
    
    #[cfg(feature = "gemini")]
    gemini_client: Option<Arc<GeminiClient>>,
    
    // Stateful registries
    narrative_registry: Arc<RwLock<NarrativeRegistry>>,
}
```

Builder methods:
```rust
impl BotticelliServerBuilder {
    pub fn dialog(mut self, dialog: Arc<DialogResource>) -> Self {
        self.dialog = Some(dialog);
        self
    }
    
    #[cfg(feature = "database")]
    pub fn database(mut self, db: Arc<Database>) -> Self {
        self.database = Some(db);
        self
    }
}
```

## Error Handling Strategy

Current tools use various error types:
- `McpError` (old)
- `SamplingError`
- `DatabaseError`
- etc.

All must convert to `rmcp::ErrorData`:

```rust
impl From<SamplingError> for rmcp::ErrorData {
    fn from(err: SamplingError) -> Self {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;
        
        rmcp::ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Owned(err.to_string()),
            None,
        )
    }
}
```

## Testing Strategy

1. **Unit Tests** - Test each tool method directly
2. **Integration Tests** - Test tool combinations
3. **Feature Tests** - Test with/without features
4. **No API Tests** - Avoid rate-limited API calls

## Rollout Plan

### Week 1: Foundation (Steps 9-12)
- Migrate core tools (database, metrics)
- Establish patterns
- Document learnings

### Week 2: Elicitation (Steps 13-16)
- Migrate primitive elicitation tools
- Test dialog resource integration
- Verify elicitation framework works

### Week 3: Narratives (Steps 17-21)
- Migrate narrative tools
- Test stateful registry
- Verify narrative workflows

### Week 4: Execution & LLM (Steps 22-26)
- Migrate execution tools
- Add LLM client integration
- Test generation workflows

### Week 5: Advanced (Steps 27-30)
- Migrate Discord tools (if needed)
- Migrate metrics/observability
- Clean up old tools/ directory

### Week 6: Polish & Cleanup
- Delete old tools/ implementations
- Update documentation
- Binary entry point updates
- Final testing

## Success Criteria

- [ ] All tools migrated to rmcp pattern
- [ ] Zero old McpTool trait implementations
- [ ] All tests passing (aim for 100+ tests)
- [ ] Documentation updated
- [ ] Binary entry points working
- [ ] Feature flags functional
- [ ] Zero clippy warnings
- [ ] Ready for production use

## Notes

- Keep old implementations until tests pass
- Commit after each tool migration
- Update PLANNING_INDEX.md regularly
- Run `just check-all` before each commit
- Maintain backward compatibility where possible

## Current Status

**Completed:**
- ✅ Steps 1-8: Infrastructure and pmcp removal
- ✅ Echo tool migrated
- ✅ server_info tool migrated
- ✅ 6/6 tests passing

**Next:** Step 9 - Migrate query_content (database) tool
