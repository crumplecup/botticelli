# Chat Tool Connection Fix

## Problem Statement
The chat interface only shows 6 tools despite having 30+ tools implemented and registered in `botticelli_mcp::server`. The issue: **botticelli_chat creates an MCP client but never connects it to the MCP server.**

## Current State
- ✅ MCP server (`botticelli_mcp::server`) registers ~30+ tools in `server.rs:225-323`
- ✅ Tools are properly implemented
- ❌ Chat creates `UnifiedMcpClient` but doesn't connect to server
- ❌ Only internal tools (6) are registered, server tools never loaded

## Root Cause
In `botticelli_chat/src/bin/botticelli-chat.rs:122`:
```rust
let mut mcp_client = UnifiedMcpClient::builder().build(); // ❌ No connection!
```

The code to connect (lines 145-159) is commented out with TODO.

## Solution Architecture

### Option 1: Stdio-Based MCP Connection (Recommended)
Use MCP's standard stdio protocol to connect chat → server:

```rust
// In botticelli-chat.rs
let mcp_config = ExternalServerConfig::builder()
    .name("botticelli-mcp".to_string())
    .command("cargo".to_string())
    .args(vec![
        "run".to_string(),
        "--bin".to_string(),
        "botticelli-mcp".to_string(),
    ])
    .build();

mcp_client.connect_external_server(mcp_config).await?;
```

### Option 2: Direct In-Process Registration
Register tools directly without separate server process:

```rust
// In botticelli-chat.rs
use botticelli_mcp::BotticelliRouterBuilder;

let router = BotticelliRouterBuilder::default()
    .name("botticelli-chat".to_string())
    .version(env!("CARGO_PKG_VERSION").to_string())
    .build()?;

// Extract tool registry from router and add to MCP client
let tools = router.list_tools();
for tool in tools {
    mcp_client.register_tool(tool);
}
```

## Implementation Plan

### Phase 1: Configuration (1 task)
**Task 1.1: Add MCP Server Config to TOML**
- [ ] Add `[mcp.internal_server]` section to `chat.toml`
- [ ] Fields: `enabled`, `command`, `args`
- [ ] Update `ChatAppConfig` to parse this

### Phase 2: Connection (2 tasks)  
**Task 2.1: Implement Server Connection**
- [ ] Uncomment and fix lines 145-159 in `botticelli-chat.rs`
- [ ] Use config values for command/args
- [ ] Add error handling for connection failure

**Task 2.2: Add Server Startup**
- [ ] Option A: Launch `botticelli-mcp` as child process
- [ ] Option B: Use in-process registration (no separate process)
- [ ] Verify all tools appear in `list_all_tools()`

### Phase 3: Testing (1 task)
**Task 3.1: Verify Tool Availability**
- [ ] Run `just chat`
- [ ] Check logs for "MCP tools loaded tool_count=30+"
- [ ] Test tool call: "create a narrative session"
- [ ] Verify tool executes successfully

## Success Criteria
1. ✅ Chat logs show 30+ tools loaded
2. ✅ LLM can see all registered tools
3. ✅ Tool calls execute successfully
4. ✅ No duplicate registrations
5. ✅ Works with `database` feature enabled/disabled

## Current Tool Count by Category
Based on `server.rs:225-323`:
- Core: 3 (echo, server_info, export_metrics)
- Narrative (basic): 5 (create, modify, save, validate, generate)
- Narrative (execution): 2 (execute_act, execute_narrative)
- Elicitation: 7 (session, metadata, acts, carousel, state, validate, apply_fixes)
- Scene: 4 (create, list, update, delete)
- Actor: 5 (create, get, update, delete, list)
- Database: 1 (query_content - with feature)
- Discord: 6 (with feature)

**Total: ~33 tools** (depending on features)

Currently showing: **6 tools** (internal only)

## Notes
- The MCP server architecture is sound - tools are properly registered
- The chat client is properly structured - just missing connection step
- This is purely a wiring issue, not an architectural problem
