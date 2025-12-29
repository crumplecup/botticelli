# RMCP Migration Vision

**Migrating from pmcp to rmcp: Embracing the Official Rust MCP SDK**

## Executive Summary

We propose migrating from `pmcp` to `rmcp` (the official Rust Model Context Protocol SDK) to leverage a mature, well-maintained ecosystem with powerful macro-based tool generation. This migration will:

1. **Replace manual tool trait implementations** with declarative `#[tool]` macros
2. **Eliminate boilerplate adapter code** through automatic schema generation
3. **Enable type-safe tool routing** with compile-time guarantees
4. **Align with ecosystem standards** for better interoperability
5. **Unlock advanced features** like structured JSON output, automatic documentation, and prompt support

## Current State Analysis

### What We Have (pmcp-based)

Our current architecture uses pmcp with manual implementations:

```rust
// Current: Manual trait implementation
#[async_trait]
impl McpTool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes back..." }
    fn input_schema(&self) -> Value { json!({...}) }
    async fn execute(&self, input: Value) -> McpResult<Value> {
        // Manual JSON extraction
        let message = input.get("message").and_then(|v| v.as_str())?;
        // Manual result construction
        Ok(json!({"echo": message}))
    }
}

// Adapter layer required
pub struct McpToolAdapter { tool: Arc<dyn McpTool> }
impl ToolHandler for McpToolAdapter { /* boilerplate */ }

// Registration with adapter wrapping
builder = builder.tool("echo", McpToolAdapter::new(EchoTool));
```

**Pain Points:**
- 30+ manual `impl McpTool` implementations across codebase
- Manual JSON schema construction prone to errors
- Adapter layer adds indirection and cognitive overhead
- No compile-time validation of schemas vs. types
- Verbose parameter extraction with runtime errors
- Duplicate documentation (docstrings + schema descriptions)

### What We Could Have (rmcp-based)

With rmcp, the same tool becomes:

```rust
// Future: Declarative macro-driven implementation
#[derive(Serialize, Deserialize, JsonSchema)]
struct EchoParams {
    /// The message to echo back
    message: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct EchoResult {
    echo: String,
    timestamp: String,
}

#[tool_router]
impl BotticelliServer {
    #[tool(description = "Echoes back the input message")]
    async fn echo(&self, Parameters(params): Parameters<EchoParams>) 
        -> Result<Json<EchoResult>, String> 
    {
        Ok(Json(EchoResult {
            echo: params.message,
            timestamp: Utc::now().to_rfc3339(),
        }))
    }
}
```

**Benefits:**
- Schema generated automatically from types
- Type-safe parameter extraction
- Direct access to structured data (no JSON juggling)
- Documentation flows from type annotations
- Compile-time validation of tool signatures
- No adapter layer needed

## Architecture Transformation

### From Trait-Based to Macro-Based Tools

**Current (pmcp):**
```
┌─────────────────┐
│   McpTool       │  ← Manual trait impl per tool
│   (trait)       │
└────────┬────────┘
         │
    ┌────▼──────────┐
    │  McpToolAdapter│  ← Adapter layer (N adapters)
    └────────┬───────┘
             │
    ┌────────▼────────┐
    │  pmcp::Server   │
    └─────────────────┘
```

**Future (rmcp):**
```
┌─────────────────┐
│ #[tool_router]  │  ← Single macro per impl block
│ impl Handler    │
│   #[tool]       │  ← Declarative per method
│   #[tool]       │
└────────┬────────┘
         │
    ┌────▼──────────┐
    │ rmcp::Server  │  ← Direct integration
    └───────────────┘
```

### Tool Organization Strategy

**Option 1: Single Large Router (Recommended for Phase 1)**
```rust
#[tool_router]
impl BotticelliServer {
    // Core tools
    #[tool] async fn echo(...) { }
    #[tool] async fn server_info(...) { }
    
    // Database tools
    #[cfg(feature = "database")]
    #[tool] async fn query_content(...) { }
    
    // Narrative tools
    #[tool] async fn create_narrative(...) { }
    #[tool] async fn validate_narrative(...) { }
    
    // Elicitation tools
    #[tool] async fn elicit_text(...) { }
    #[tool] async fn elicit_select(...) { }
}
```

**Benefits:**
- Simple, centralized tool management
- Easy to see all tools at once
- Minimal refactoring required
- Feature flags work naturally

**Option 2: Modular Routers (Future Evolution)**
```rust
// Separate impl blocks per domain
mod database_tools {
    #[tool_router(router = database_router, vis = "pub")]
    impl DatabaseTools {
        #[tool] async fn query_content(...) { }
        #[tool] async fn list_tables(...) { }
    }
}

mod narrative_tools {
    #[tool_router(router = narrative_router, vis = "pub")]
    impl NarrativeTools {
        #[tool] async fn create_narrative(...) { }
        #[tool] async fn validate_narrative(...) { }
    }
}

// Combine routers
impl BotticelliServer {
    fn new() -> Self {
        Self {
            tool_router: database_tools::database_router() 
                       + narrative_tools::narrative_router(),
        }
    }
}
```

**Benefits:**
- Better code organization by domain
- Independent development of tool domains
- Easier to test tool subsets
- Natural alignment with crate boundaries

## Migration Strategy

### Phase 1: Foundation (Week 1)

**Goal:** Establish rmcp infrastructure alongside pmcp

1. **Add rmcp dependency**
   ```toml
   [dependencies]
   rmcp = { version = "0.12.0", features = ["server"] }
   rmcp-macros = "0.12.0"
   ```

2. **Create parallel server structure**
   ```
   crates/botticelli_mcp/src/
   ├── lib.rs
   ├── pmcp_server.rs         # Keep existing
   ├── rmcp_server.rs         # New: rmcp implementation
   ├── tools/                 # Keep existing tools
   └── rmcp_tools/            # New: macro-based tools
       ├── mod.rs
       ├── core.rs            # echo, server_info
       └── types.rs           # Shared parameter types
   ```

3. **Implement 3-5 pilot tools** using rmcp macros
   - `echo` - Simple no-state tool
   - `server_info` - Access to self state
   - `query_content` - Database integration
   - `create_narrative` - Complex parameters
   - `elicit_text` - Stateful with dialog resource

4. **Create rmcp binary** alongside pmcp one
   ```rust
   // src/bin/botticelli-mcp-rmcp.rs
   ```

5. **Document patterns** in RMCP_PATTERNS.md

**Deliverables:**
- Working rmcp server with 5 tools
- Side-by-side comparison with pmcp equivalents
- Pattern documentation for team

### Phase 2: Core Migration (Week 2-3)

**Goal:** Migrate all core and database tools

1. **Type definitions**
   - Create `rmcp_tools/types.rs` with all parameter/result types
   - Use `#[derive(Serialize, Deserialize, JsonSchema)]`
   - Extract from existing JSON schemas

2. **Tool migration order**
   - Core tools (5 tools)
   - Database tools (1 tool) 
   - Narrative tools (5 tools)
   - Generator tools (2-5 depending on features)

3. **Testing strategy**
   - Keep integration tests pointing at pmcp server
   - Add parallel rmcp integration tests
   - Compare outputs between implementations
   - Validate schema compatibility

4. **Feature flags**
   - Ensure `#[cfg(feature = "...")]` works with macros
   - Test feature combinations
   - Document feature interactions

**Deliverables:**
- ~15 tools migrated to rmcp
- All tests passing for both servers
- Feature flag validation complete

### Phase 3: Elicitation Tools (Week 3-4)

**Goal:** Migrate stateful elicitation system

**Challenge:** Elicitation tools require shared mutable state (DialogResource)

**Solution 1: Arc-based State (Recommended)**
```rust
#[derive(Clone)]
pub struct BotticelliServer {
    dialog: Arc<DialogResource>,
    narrative_registry: Arc<PartialNarrativeRegistry>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BotticelliServer {
    #[tool]
    async fn elicit_text(
        &self, 
        Parameters(params): Parameters<ElicitTextParams>
    ) -> Result<Json<ElicitTextResult>, String> {
        // Access dialog via Arc
        self.dialog.prompt_text(&params.prompt).await
            .map(|text| Json(ElicitTextResult { value: text }))
            .map_err(|e| e.to_string())
    }
}
```

**Solution 2: Message-Passing**
```rust
// Hybrid: Tools send messages to dialog actor
#[tool]
async fn elicit_text(...) -> Result<...> {
    self.dialog_tx.send(ElicitTextRequest { ... }).await?;
    let result = self.dialog_rx.recv().await?;
    Ok(Json(result))
}
```

**Tasks:**
1. Design state management approach
2. Migrate 4 primitive elicitation tools
3. Migrate session-based tools (create/finalize/modify)
4. Integrate with carousel state machine
5. Test complex multi-step elicitation flows

**Deliverables:**
- All elicitation tools migrated
- State management pattern documented
- Complex flow tests passing

### Phase 4: Advanced Features (Week 4-5)

**Goal:** Leverage rmcp unique capabilities

1. **Prompt support**
   ```rust
   #[prompt_router]
   impl BotticelliServer {
       #[prompt(name = "narrative_guidance")]
       async fn narrative_guidance(
           &self, 
           Parameters(args): Parameters<NarrativeGuidanceArgs>
       ) -> Result<Vec<PromptMessage>, String> {
           // Generate structured prompts for LLMs
       }
   }
   ```

2. **Resource support**
   - Expose narratives as MCP resources
   - Provide database content as resources
   - Enable resource templates

3. **Structured outputs**
   - Add JSON schemas to all tool outputs
   - Enable LLMs to understand return types
   - Better error messages with structured errors

4. **Tool composition**
   - Use multiple `#[tool_router]` blocks
   - Combine domain-specific routers
   - Modular tool organization

**Deliverables:**
- Prompt system integrated
- Resources exposed via MCP
- Structured output documentation
- Modular router architecture

### Phase 5: Cleanup & Deprecation (Week 5-6)

**Goal:** Remove pmcp completely

1. **Deprecation notices**
   - Mark `pmcp_server.rs` as deprecated
   - Add migration guide for any external users
   - Update all documentation

2. **Binary consolidation**
   - Remove `botticelli-mcp-pmcp` binary
   - Rename `botticelli-mcp-rmcp` to `botticelli-mcp`
   - Update deployment configs

3. **Code removal**
   - Delete `pmcp_server.rs`
   - Delete `pmcp_adapters.rs`
   - Delete `pmcp_middleware.rs`
   - Remove pmcp dependency

4. **Test cleanup**
   - Remove pmcp-specific tests
   - Consolidate integration tests
   - Update test documentation

5. **Documentation updates**
   - Rewrite MCP.md
   - Update README.md
   - Create RMCP_GUIDE.md
   - Add migration tutorial

**Deliverables:**
- pmcp code removed
- Single rmcp-based binary
- Complete documentation refresh
- Zero breaking changes for end users

## Key Opportunities

### 1. Type-Safe Tool Ecosystem

**Before:**
```rust
// Runtime JSON manipulation
let prompt = input.get("prompt")
    .and_then(|v| v.as_str())
    .ok_or_else(|| McpError::invalid_input("Missing 'prompt'"))?;
    
let options = input.get("options")
    .and_then(|v| v.as_array())
    .ok_or_else(|| McpError::invalid_input("Missing 'options'"))?
    .iter()
    .filter_map(|v| v.as_str())
    .map(String::from)
    .collect();
```

**After:**
```rust
// Compile-time validated types
#[derive(Deserialize, JsonSchema)]
struct SelectParams {
    prompt: String,
    options: Vec<String>,
    #[serde(default)]
    allow_multiple: bool,
}

#[tool]
async fn elicit_select(
    &self, 
    Parameters(params): Parameters<SelectParams>
) -> Result<Json<SelectResult>, String> {
    // Direct field access, no runtime parsing
    self.dialog.prompt_select(
        &params.prompt, 
        &params.options,
        params.allow_multiple
    ).await
}
```

**Benefits:**
- Compile-time validation of parameter types
- IDE autocomplete for tool parameters
- Refactoring safety (rename detection)
- Self-documenting code through types
- Reduced runtime error surface

### 2. Automatic Schema Generation

**Before:**
```rust
fn input_schema(&self) -> Value {
    json!({
        "type": "object",
        "properties": {
            "prompt": {
                "type": "string",
                "description": "The prompt to display"  // Can drift from impl
            },
            "options": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Available choices"
            }
        },
        "required": ["prompt", "options"]  // Easy to forget fields
    })
}
```

**After:**
```rust
#[derive(Deserialize, JsonSchema)]
struct SelectParams {
    /// The prompt to display to the user
    #[schemars(description = "Question or instruction")]
    prompt: String,
    
    /// Available choices for the user to select from
    options: Vec<String>,
    
    /// Allow selecting multiple options
    #[serde(default)]
    allow_multiple: bool,
}

// Schema generated automatically, always in sync!
```

**Benefits:**
- Schema and implementation always match
- Documentation in one place (doc comments)
- Optional fields with `#[serde(default)]`
- Validation rules via schemars attributes
- No manual JSON schema maintenance

### 3. Eliminate Adapter Layer

**Impact:** ~200 lines of boilerplate removed

```rust
// DELETE: crates/botticelli_mcp/src/pmcp_adapters.rs
pub struct McpToolAdapter { /* 50 lines */ }
impl ToolHandler for McpToolAdapter { /* 100 lines */ }

// DELETE: crates/botticelli_mcp/src/pmcp_middleware.rs  
// Middleware wrapping logic (~50 lines)

// DELETE: Registration boilerplate
builder = builder.tool("name", McpToolAdapter::new(Tool));  // × 30 tools
```

**Direct integration:**
```rust
#[tool_router]
impl BotticelliServer {
    #[tool] async fn tool1(...) { }
    #[tool] async fn tool2(...) { }
    // Router generated automatically
}
```

### 4. Enhanced Error Handling

**Before:**
```rust
async fn execute(&self, input: Value) -> McpResult<Value> {
    let id = input.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_input("Missing id"))?;
    
    let narrative = load_narrative(id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string()))?;
    
    Ok(json!({"narrative": narrative}))
}
```

**After:**
```rust
#[derive(Serialize, JsonSchema)]
enum NarrativeError {
    #[serde(rename = "not_found")]
    NotFound { id: String },
    
    #[serde(rename = "invalid_format")]
    InvalidFormat { reason: String },
    
    #[serde(rename = "io_error")]
    IoError { message: String },
}

#[tool]
async fn get_narrative(
    &self,
    Parameters(params): Parameters<GetNarrativeParams>
) -> Result<Json<Narrative>, Json<NarrativeError>> {
    load_narrative(&params.id)
        .await
        .map(Json)
        .map_err(|e| Json(match e {
            LoadError::NotFound(id) => NarrativeError::NotFound { id },
            LoadError::Parse(msg) => NarrativeError::InvalidFormat { reason: msg },
            LoadError::Io(err) => NarrativeError::IoError { message: err.to_string() },
        }))
}
```

**Benefits:**
- Structured error responses
- LLMs can parse and handle errors
- Schema validation for errors too
- Better debugging with typed errors

### 5. New Feature: Prompts

Prompts are reusable LLM prompt templates that servers can expose:

```rust
#[derive(Deserialize, JsonSchema)]
struct NarrativeReviewArgs {
    narrative_toml: String,
    focus_area: Option<String>,
}

#[prompt_router]
impl BotticelliServer {
    #[prompt(
        name = "review_narrative",
        description = "Generate a detailed review prompt for a narrative"
    )]
    async fn review_narrative_prompt(
        &self,
        Parameters(args): Parameters<NarrativeReviewArgs>
    ) -> Result<Vec<PromptMessage>, String> {
        let focus = args.focus_area.unwrap_or("overall quality".to_string());
        
        Ok(vec![
            PromptMessage {
                role: Role::User,
                content: Content::Text(format!(
                    "Review this narrative TOML, focusing on {}:\n\n{}",
                    focus, args.narrative_toml
                )),
            }
        ])
    }
    
    #[prompt(name = "act_generation")]
    async fn act_generation_prompt(...) -> Result<...> {
        // Generate prompts for creating new narrative acts
    }
}
```

**Use Cases:**
- Standardize prompt templates across clients
- Version prompts alongside tools
- Enable clients to discover available prompts
- Reduce prompt duplication in client code

### 6. Resource Integration

Expose data as MCP resources (files, database records, etc.):

```rust
impl ServerHandler for BotticelliServer {
    async fn list_resources(&self, ...) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource {
                uri: "narrative://metadata".into(),
                name: "Narrative Metadata".into(),
                description: Some("List of all narrative names and descriptions".into()),
                mime_type: Some("application/json".into()),
            },
            Resource {
                uri: "db://content".into(),
                name: "Database Content".into(),
                description: Some("Queryable content table".into()),
                mime_type: Some("application/json".into()),
            },
        ]))
    }
    
    async fn read_resource(&self, request: ReadResourceRequest, ...) 
        -> Result<ReadResourceResult, ErrorData> 
    {
        match request.uri.as_str() {
            "narrative://metadata" => {
                let narratives = self.list_all_narratives().await?;
                Ok(ReadResourceResult { 
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&narratives)?
                    )]
                })
            }
            // ...
        }
    }
}
```

**Benefits:**
- LLMs can browse available data
- Lazy loading of large datasets
- Standard URI-based access
- Better than embedding data in tool responses

## Risk Mitigation

### Risk 1: Breaking Changes During Migration

**Mitigation:**
- Parallel implementation (pmcp + rmcp coexist)
- Feature flag to switch between implementations
- Extensive integration testing
- Gradual rollout with fallback plan

### Risk 2: rmcp API Instability

**Current:** rmcp is at v0.12.0 (pre-1.0)

**Mitigation:**
- Pin to specific version initially
- Monitor rmcp changelog closely
- Contribute fixes upstream if needed
- Wrap rmcp types in our own abstractions for critical paths
- Official SDK means good backward compatibility expectations

### Risk 3: Elicitation State Management

**Challenge:** rmcp tools are methods on `&self`, but elicitation needs mutable state

**Mitigation:**
- Use `Arc<DialogResource>` with interior mutability
- DialogResource already uses channels internally
- Test state access patterns early (Phase 1)
- Consider actor pattern if needed
- Prototype in Phase 1 with pilot tools

### Risk 4: Learning Curve

**Mitigation:**
- Start with simple tools (echo, server_info)
- Document patterns extensively
- Create migration cookbook with examples
- Pair programming for first few tools
- Weekly knowledge sharing sessions

### Risk 5: Performance Impact

**Concern:** Additional abstraction layers

**Mitigation:**
- Benchmark current vs. new implementation
- rmcp uses zero-cost abstractions (macros expand at compile-time)
- Less runtime JSON parsing likely improves performance
- Profile with realistic workloads
- Optimization pass if needed in Phase 5

## Success Metrics

### Code Quality
- [ ] 50%+ reduction in boilerplate (target: ~500 lines removed)
- [ ] Zero `#[allow(dead_code)]` in tool implementations
- [ ] 100% schema-type consistency (compile-time guaranteed)
- [ ] All tools use `#[tool]` macro (no manual trait impls)

### Development Velocity
- [ ] New tool creation: <30 minutes (vs. ~1 hour currently)
- [ ] Schema updates: automatic (vs. manual JSON editing)
- [ ] Tool refactoring: IDE-assisted (vs. manual text search)

### Reliability
- [ ] Zero schema-implementation mismatches
- [ ] 100% type-safe parameter extraction
- [ ] Compile-time tool validation
- [ ] All integration tests passing

### Features
- [ ] Structured JSON output on all tools
- [ ] At least 3 prompt templates exposed
- [ ] Resource support for narratives and database
- [ ] Modular tool routers by domain

### Documentation
- [ ] Complete RMCP_GUIDE.md
- [ ] Migration tutorial with examples
- [ ] Pattern cookbook for common cases
- [ ] Updated MCP.md reflecting rmcp

## Timeline

| Week | Phase | Deliverables | Risk Level |
|------|-------|--------------|------------|
| 1 | Foundation | 5 pilot tools, parallel infra | Medium |
| 2-3 | Core Migration | 15 tools migrated, tests passing | Low |
| 3-4 | Elicitation | Stateful tools, complex flows | High |
| 4-5 | Advanced Features | Prompts, resources, composition | Medium |
| 5-6 | Cleanup | pmcp removed, docs updated | Low |

**Total Duration:** 5-6 weeks
**Parallel Work:** Can continue feature development on pmcp during migration
**Rollback Point:** End of each phase (pmcp remains functional)

## Conclusion

Migrating from pmcp to rmcp is a strategic investment that will:

1. **Reduce maintenance burden** through declarative tooling
2. **Improve type safety** with compile-time validation
3. **Accelerate development** with automatic schema generation
4. **Unlock new features** like prompts and resources
5. **Align with ecosystem** using official MCP SDK

The macro-based approach eliminates the adapter layer and 30+ manual trait implementations, replacing them with clean, declarative tool definitions. Type-safe parameter extraction and automatic schema generation eliminate entire classes of runtime errors.

With a careful phased approach, we can migrate incrementally while maintaining full backward compatibility, reducing risk and enabling continuous testing throughout the process.

**Recommendation:** Proceed with Phase 1 to validate the approach with pilot tools, then commit to full migration based on concrete results.

---

## Appendix: Code Comparison

### Example 1: Simple Tool (Echo)

**Current (pmcp, 49 lines):**
```rust
pub struct EchoTool;

#[async_trait]
impl McpTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes back the input message. Useful for testing the MCP connection."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo back"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        debug!(input = ?input, "Echo tool called");

        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing 'message' field".to_string()))?;

        Ok(json!({
            "echo": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

// Plus adapter registration
builder = builder.tool("echo", McpToolAdapter::new(EchoTool));
```

**Future (rmcp, 25 lines):**
```rust
#[derive(Deserialize, JsonSchema)]
struct EchoParams {
    /// The message to echo back
    message: String,
}

#[derive(Serialize, JsonSchema)]
struct EchoResult {
    echo: String,
    timestamp: String,
}

#[tool_router]
impl BotticelliServer {
    /// Echoes back the input message. Useful for testing the MCP connection.
    #[tool]
    async fn echo(&self, Parameters(params): Parameters<EchoParams>) 
        -> Result<Json<EchoResult>, String> 
    {
        Ok(Json(EchoResult {
            echo: params.message,
            timestamp: Utc::now().to_rfc3339(),
        }))
    }
}
```

**Savings:** -24 lines (49%), zero JSON manipulation, type-safe

### Example 2: Complex Tool (Query Content)

**Current (pmcp, 85 lines):**
```rust
pub struct QueryContentTool {
    db_ops: Arc<dyn DatabaseRegistryOperations>,
}

#[async_trait]
impl McpTool for QueryContentTool {
    fn name(&self) -> &str {
        "query_content"
    }

    fn description(&self) -> &str {
        "Query the content table with filters"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "table_name": {"type": "string"},
                "limit": {"type": "integer", "default": 10},
                "filters": {
                    "type": "object",
                    "properties": {
                        "content_type": {"type": "string"},
                        "after_date": {"type": "string"}
                    }
                }
            },
            "required": ["table_name"]
        })
    }

    async fn execute(&self, input: Value) -> McpResult<Value> {
        let table_name = input.get("table_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_input("Missing table_name"))?;
        
        let limit = input.get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);
        
        let filters = input.get("filters")
            .and_then(|v| v.as_object())
            .cloned();
        
        let content_type = filters.as_ref()
            .and_then(|f| f.get("content_type"))
            .and_then(|v| v.as_str());
        
        let after_date = filters.as_ref()
            .and_then(|f| f.get("after_date"))
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        
        let results = self.db_ops
            .query_content(table_name, limit, content_type, after_date)
            .await
            .map_err(|e| McpError::internal_error(e.to_string()))?;
        
        Ok(json!({
            "results": results,
            "count": results.len()
        }))
    }
}

builder = builder.tool("query_content", McpToolAdapter::new(
    QueryContentTool::new(db_ops)
));
```

**Future (rmcp, 45 lines):**
```rust
#[derive(Deserialize, JsonSchema)]
struct QueryContentParams {
    /// Name of the table to query
    table_name: String,
    
    /// Maximum number of results
    #[serde(default = "default_limit")]
    limit: i64,
    
    /// Optional filters
    #[serde(default)]
    filters: QueryFilters,
}

#[derive(Deserialize, JsonSchema, Default)]
struct QueryFilters {
    content_type: Option<String>,
    
    #[serde(with = "optional_datetime")]
    after_date: Option<DateTime<Utc>>,
}

#[derive(Serialize, JsonSchema)]
struct QueryContentResult {
    results: Vec<ContentRow>,
    count: usize,
}

fn default_limit() -> i64 { 10 }

#[tool_router]
impl BotticelliServer {
    /// Query the content table with optional filters
    #[tool]
    async fn query_content(
        &self,
        Parameters(params): Parameters<QueryContentParams>
    ) -> Result<Json<QueryContentResult>, String> {
        let results = self.db_ops
            .query_content(
                &params.table_name,
                params.limit,
                params.filters.content_type.as_deref(),
                params.filters.after_date
            )
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(Json(QueryContentResult {
            count: results.len(),
            results,
        }))
    }
}
```

**Savings:** -40 lines (47%), compile-time filter validation, better defaults

## Next Steps

1. **Review this vision document** with team
2. **Approve or modify** migration strategy
3. **Begin Phase 1** with pilot tool implementation
4. **Schedule weekly sync** to review progress
5. **Update PLANNING_INDEX.md** with this document

---

*Document created: 2024-12-29*
*Status: DRAFT - Awaiting Review*
*Owner: Development Team*
