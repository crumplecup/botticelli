# MCP Server Test Findings

## Tests Created

Two MCP integration tests were created in `crates/botticelli/tests/mcp_server_test.rs`:

1. **test_mcp_server_startup** - Verifies MCP server starts and responds to /health and /info endpoints
2. **test_mcp_client_connection** - Verifies MCP client can connect via HTTP transport and list tools

## Issues Discovered

### 1. MCP HTTP Binary Was Broken

**File:** `crates/botticelli_mcp/src/bin/botticelli-mcp-http.rs`

**Problem:** Server builder was missing required tools registry:
```rust
// ❌ Before - panics on startup
let router = BotticelliRouter::builder()
    .name("botticelli")
    .version(env!("CARGO_PKG_VERSION"))
    .resources(resources)
    .build(); // Panics: "Tools registry must be provided via with_tools()"
```

**Fix:** Added empty ToolRegistry:
```rust
// ✅ After - starts successfully
let tools = ToolRegistry::new();
let router = BotticelliRouter::builder()
    .name("botticelli")
    .version(env!("CARGO_PKG_VERSION"))
    .tools(tools)
    .resources(resources)
    .build();
```

### 2. Port Configuration Mismatch

**Justfile expects port 3030:**
```bash
# justfile
for i in {1..30}; do
    if curl -sf http://localhost:3030/health > /dev/null 2>&1; then
```

**MCP HTTP binary defaults to port 3000:**
```rust
// botticelli-mcp-http.rs
let port = std::env::var("MCP_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(3000); // ❌ Should be 3030
```

**Fix needed:** Change default to 3030 or set MCP_PORT environment variable in justfile.

### 3. Test Strategy Issues

**Problem:** `just chat rebuild` performs full cargo rebuild including dependencies:
- Takes 30-60 seconds just to compile
- Test timeout of 30 seconds insufficient
- Kills child processes before they finish starting

**Current approach:**
```rust
Command::new("just").arg("chat").arg("rebuild")
    .spawn()?;
thread::sleep(Duration::from_secs(30)); // Not enough!
```

**Better approaches:**
1. Start pre-compiled binary directly
2. Use fixture that keeps server running between tests
3. Mock HTTP responses instead of starting real server

### 4. Empty Tool Registry

Server starts with 0 tools. Need to either:
- Use `pmcp_server::register_all_tools()` to register built-in tools
- Or accept empty registry for basic health check tests

## Recommendations

### Short Term
1. ✅ Fix port mismatch (use 3030 consistently)
2. ✅ Fix MCP HTTP binary panic (tool registry - DONE)
3. Update test to start pre-built binary instead of `just chat rebuild`
4. Register default tools in HTTP binary

### Long Term  
1. Create test fixture that manages server lifecycle
2. Add proper health check endpoint that waits for full init
3. Consider separating "server startup" tests from "client connection" tests
4. Add MCP server port to centralized config

## Test Documentation

Created:
- `MCP_SERVER_TEST_GUIDE.md` - Usage instructions for the tests
- `MCP_TEST_FINDINGS.md` - This document with discovered issues

## Status

- ✅ Tests compile successfully
- ✅ Fixed MCP HTTP binary panic
- ⏳ Tests don't pass yet (port/timing issues)
- ⏳ Need to update test strategy
- ⏳ Need to register default tools in HTTP binary

## Next Steps

1. Change MCP HTTP binary default port to 3030
2. Register tools using `register_all_tools()` or similar
3. Update test to use pre-built binary
4. Increase test timeout or add polling with retries
5. Run tests to verify fixes
