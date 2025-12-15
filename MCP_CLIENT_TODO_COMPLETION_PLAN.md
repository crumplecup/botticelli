# MCP Client TODO Completion Plan

**Status**: Phase 3 Ready  
**Branch**: `pmcp-cleanup`  
**Progress**: 2/4 phases complete

## Completed ✅

### Phase 1: Remove Dead Code ✅
- Removed unused `transport.rs` module (196 lines)
- Commit: 4cfa5c6

### Phase 2: Simplify Tool Execution ✅  
- Removed incomplete internal tool executor
- UnifiedMcpClient now handles external servers only
- All tests passing
- Commit: dec97a3

## Next Steps

### Phase 3: Migrate from McpClient → UnifiedMcpClient

**Current Issue**: `McpClient.extract_tool_calls()` returns `None` (TODO at client.rs:103)

**Used in**:
- `crates/botticelli_chat/src/services.rs`
- `crates/botticelli/src/cli/mcp.rs`

**Plan**:
1. Replace `McpClient` with `UnifiedMcpClient` in both files
2. Delete `crates/botticelli_mcp_client/src/client.rs`
3. Remove `McpClient` export from lib.rs
4. Verify all tests pass

### Phase 4: Documentation
1. Update module-level docs
2. Add usage example
3. Document architecture decisions

---

## Ready to Proceed with Phase 3

Migrating McpClient usages to UnifiedMcpClient.
