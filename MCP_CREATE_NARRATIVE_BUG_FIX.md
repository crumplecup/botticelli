# MCP Create Narrative Tool Bug Fix

## Problem

When using the chat interface to create a narrative, the system reported:
```
Tool 'create_narrative' not found
```

## Root Cause

The `CreateNarrativeTool` was implemented in `crates/botticelli_mcp/src/tools/create_narrative.rs` and exported from the tools module, but **was never registered** in the `BotticelliRouter` default tool set.

## Fix

Added `CreateNarrativeTool` registration to `/home/erik/repos/botticelli/crates/botticelli_mcp/src/server.rs`:

```rust
// Line ~216
registry.register(Arc::new(crate::tools::CreateNarrativeTool));
```

Added between `ServerInfoTool` and `ValidateNarrativeTool` registrations.

## Testing

### Integration Test Created

Created `/home/erik/repos/botticelli/crates/botticelli_chat/tests/mcp_server_tools_test.rs` to verify:

1. MCP HTTP server exposes `/tools/list` endpoint
2. `create_narrative` tool is in the list
3. Tool has correct structure (description, inputSchema, parameters)

### Manual Testing

1. Rebuild MCP server:
   ```bash
   cargo build --bin botticelli-mcp-http -p botticelli_mcp --features http
   ```

2. Start server:
   ```bash
   cargo run --bin botticelli-mcp-http -p botticelli_mcp --features http
   ```

3. Query available tools:
   ```bash
   curl http://localhost:3000/tools/list | jq '.tools[].name'
   ```

4. Verify `create_narrative` is in the list

5. Test via chat interface:
   ```bash
   just chat-local
   # Type: "create a narrative about..."
   ```

## Files Changed

- `crates/botticelli_mcp/src/server.rs` - Added tool registration
- `crates/botticelli_chat/tests/mcp_server_tools_test.rs` - New integration test

## Related

- Chat system expects MCP server to be running
- Auto-start logic in `crates/botticelli_chat/src/startup.rs`
- MCP client calls in `crates/botticelli_chat/src/executor.rs`

## Lessons Learned

1. **Tool registration is manual** - New tools must be explicitly added to the router builder
2. **Integration tests catch this** - Unit tests wouldn't have detected the missing registration
3. **HTTP server needs `http` feature** - Binary requires `-p botticelli_mcp --features http`
4. **Separate tool implementation from registration** - Easy to forget the second step

## Next Steps

1. Consider auto-registration via inventory crate or similar
2. Add CI check that all tools in `tools/` module are registered
3. Document tool addition process in CONTRIBUTING.md
