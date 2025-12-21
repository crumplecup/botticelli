# Chat Tool Integration Analysis

## Current State

### ✅ What's Working

1. **Tool Execution Infrastructure**
   - `conversation_loop.rs` - Manages multi-turn LLM conversations with tool calling
   - `tool_handler.rs` - Executes tools via MCP client
   - `UnifiedMcpClient` - Connects to tool registries
   - Tool results feed back into LLM for next turn

2. **Registered Tools** (via `register_internal_tools`)
   - **Database Tools** (with `database` feature):
     - `create_table` - Create database tables
     - `query_table` - Query database tables  
     - `inspect_table` - Inspect table schema
     - `table_exists` - Check if table exists
   - **Elicitation Tools**:
     - `create_elicitation_session` - Start elicitation workflow
     - `elicit_metadata` - Elicit narrative metadata
     - `elicit_act` - Elicit narrative acts
     - `finalize_elicitation` - Complete elicitation
     - `create_carousel` - Create input carousel
     - `execute_carousel` - Execute carousel workflow

### ❌ What's Missing

1. **LLM Not Seeing Tools**
   - Tools are registered in `UnifiedMcpClient` internal registry
   - But LLM client needs tool definitions via `ToolCalling::available_tools()`
   - Need to wire MCP registry tools into LLM provider initialization

2. **No Narrative CRUD Tools**
   - Comment in registry.rs line 33-35: "Narrative tools need refactoring"
   - Missing: `create_narrative`, `list_narratives`, `load_narrative`, `validate_narrative`
   - These existed before but removed during database migration

3. **Tool Discovery Not Exposed**
   - LLM can't discover what tools are available
   - Need to pass tool definitions from registry to LLM client

## Root Cause Analysis

The system has **two separate worlds**:

1. **MCP Tool World** - Tools registered in `ToolRegistry`, executable via `UnifiedMcpClient`
2. **LLM Provider World** - LLM clients need `Vec<ToolDefinition>` to advertise tools

**The bridge is missing**: We need to convert registered tools in MCP registry into `ToolDefinition` objects that LLM providers can advertise.

## Solution Architecture

### Phase 1: Bridge MCP Registry → LLM Providers

```rust
// In UnifiedMcpClient
impl ToolCalling for UnifiedMcpClient {
    fn available_tools(&self) -> Vec<ToolDefinition> {
        self.internal_registry()
            .list_tools()
            .into_iter()
            .map(|name| {
                // Get tool from registry
                // Convert to ToolDefinition
            })
            .collect()
    }
}
```

### Phase 2: Pass Tools to LLM Client

```rust
// In botticelli-chat.rs main()
let mcp_client = UnifiedMcpClient::builder().build();
register_internal_tools(mcp_client.internal_registry_mut(), "./narratives", Some(db_pool))?;

// Create LLM client with tools
let tool_defs = mcp_client.available_tools();
let llm_client = services.create_tool_calling_client(model_id)?
    .with_tools(tool_defs)?;
```

### Phase 3: Re-implement Narrative CRUD Tools

Create database-backed versions:
- `CreateNarrativeTool` - Insert into database
- `ListNarrativesTool` - Query narratives table
- `LoadNarrativeTool` - Fetch by ID
- `ValidateNarrativeTool` - Validate structure

Register in `register_internal_tools()` alongside other database tools.

## Testing Strategy

1. **Unit Tests**: Tool registration and conversion
2. **Integration Tests**: LLM sees and calls tools
3. **End-to-End**: Full conversation with tool usage

## Success Criteria

- [ ] LLM client receives all registered tool definitions
- [ ] LLM can call any registered tool
- [ ] Tool results flow back to LLM correctly
- [ ] Narrative CRUD tools implemented and working
- [ ] All elicitation tools visible and callable
- [ ] Database tools functional (with feature flag)

## Next Steps

1. Implement `ToolRegistry::list_tools()` method
2. Add `ToolDefinition` extraction from registered tools
3. Wire tool definitions into LLM client initialization
4. Re-implement narrative CRUD tools with database backend
5. Test end-to-end tool calling flow
