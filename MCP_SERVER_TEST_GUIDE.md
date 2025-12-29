# MCP Server Testing Guide

This document describes the MCP server integration tests.

## Overview

Two tests were created to verify MCP server functionality:

### Test A: Server Startup and Health (`test_mcp_server_startup`)

**Purpose:** Verify the MCP server starts, is reachable, and reports correct version info.

**What it tests:**
- Server `/health` endpoint responds with `{"status": "healthy"}`
- Server `/info` endpoint returns version and name
- Version follows semver format (e.g., "0.2.0")
- Server name is non-empty

**How to run:**
```bash
cargo test --package botticelli --test mcp_server_test test_mcp_server_startup -- --ignored --nocapture
```

**Expected output:**
```
Testing MCP server startup and health...
Waiting 5 seconds for MCP server to start...
Checking /health endpoint...
Health response: {
  "status": "healthy"
}
Checking /info endpoint...
Server info: {
  "name": "Botticelli MCP Server",
  "version": "0.2.0",
  "tools": [...],
  "resources": [...]
}

=== MCP Server Verification ===
✓ Health endpoint responding
✓ Version correct: 0.2.0
✓ Server name: Botticelli MCP Server
```

### Test B: Client Connection (`test_mcp_client_connection`)

**Purpose:** Verify the MCP client can connect to the server and list tools.

**What it tests:**
- HTTP transport connects to MCP server at localhost:8080
- `tools/list` MCP protocol call works
- Server returns at least one tool
- Tool definitions have name and description fields

**How to run:**
```bash
cargo test --package botticelli --test mcp_server_test test_mcp_client_connection -- --ignored --nocapture
```

**Expected output:**
```
Testing MCP client connection to server...
Creating HTTP transport to MCP server...
Connecting to MCP server...
✓ Successfully connected to MCP server via HTTP transport

Listing available tools via transport...
Found 12 tools:
  - create_narrative: Creates a new narrative from a TOML specification
  - list_narratives: Lists all available narratives
  - ...

=== MCP Client Verification ===
✓ HTTP transport connected successfully
✓ Tools listed: 12 available
```

## Test Location

- **File:** `crates/botticelli/tests/mcp_server_test.rs`
- **Package:** `botticelli`

## Dependencies

These tests require the following dev-dependencies (already added):
- `tokio` - async runtime for Test B
- `reqwest` - HTTP client for both tests
- `serde_json` - JSON parsing
- `botticelli_mcp_client` - MCP client library

## Implementation Notes

### Test A (Server Startup)
- Uses `just chat rebuild` to start TUI (which auto-starts MCP server)
- Blocks for 5 seconds to allow startup
- Makes synchronous HTTP requests to verify endpoints
- Cleans up child process on completion

### Test B (Client Connection)
- Checks if server is already running (via `/health`)
- If not, starts it with `just mcp rebuild`
- Creates `HttpTransport` to communicate via MCP protocol
- Uses `McpTransport` trait methods to connect and list tools
- Async test using `#[tokio::test]`

### Why HTTP Transport?

The `ExternalServerConfig` is designed for stdio-based MCP servers (spawning child processes like `npx @modelcontextprotocol/server-filesystem`). Our MCP server uses HTTP, so we use `HttpTransport` directly.

## Troubleshooting

### Server not starting
- Ensure port 8080 is available
- Check MCP server binary is built: `cargo build --package botticelli_mcp`
- View logs: `cat botticelli-mcp.log`

### Connection refused
- Server may need more time to start (increase sleep duration)
- Check firewall settings
- Verify server is listening: `lsof -i :8080`

### Tool list empty
- Check that internal tools are registered
- Verify MCP server configuration
- Check server logs for tool registration failures

## Future Enhancements

- [ ] Test tool execution (call a tool and verify response)
- [ ] Test error handling (invalid tool names, malformed requests)
- [ ] Test concurrent connections
- [ ] Test server restart/recovery
- [ ] Integration with chat TUI (verify TUI connects to server)
