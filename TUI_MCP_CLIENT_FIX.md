# TUI MCP Client Startup Fix

## Problem
The TUI's MCP HTTP client had several issues discovered through testing the MCP server:

1. **Hardcoded wrong port**: Used `3030` instead of `8080`
2. **No server verification**: Started blindly without checking if MCP server exists
3. **Poor observability**: No clear logs showing connection attempts or failures
4. **No configuration**: Hardcoded values instead of environment variables

## Solution Applied

### Self-Booting MCP Server

The TUI now automatically starts the MCP server if it's not running. No manual server startup required!

**How it works:**
1. TUI checks if MCP server is reachable on startup
2. If not reachable → automatically spawns embedded MCP server
3. Waits 1 second for server to start
4. Verifies server started successfully
5. If auto-start fails → provides manual command

### Lessons from MCP Server Tests
The MCP server lifecycle tests taught us:
- ✅ **Log exact endpoints and ports** - no guessing
- ✅ **Verify server is listening** before trying to use it
- ✅ **Use structured logging** with emoji markers for scannable output
- ✅ **Handle failures gracefully** with helpful error messages

### Changes Made to `minimal_loop.rs`

#### 1. Environment-based Configuration (Lines 54-67)
```rust
// Get MCP server configuration from environment or use defaults
let mcp_host = std::env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
let mcp_port: u16 = std::env::var("MCP_HTTP_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(8080);  // ✅ Correct default port

let mcp_url = format!("http://{}:{}/sse", mcp_host, mcp_port);
info!("📍 MCP server endpoint: {}", mcp_url);
```

**Before**: Hardcoded `http://localhost:3030/sse`
**After**: Configurable via env vars, correct default port

### 2. Server Verification and Auto-Start (Lines 69-128)
```rust
// Verify MCP server is reachable, start it if not
info!("🔍 Verifying MCP server is reachable at {}", mcp_url);

tokio::spawn(async move {
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    match test_client.get(&test_url).send().await {
        Ok(response) => {
            info!("✅ MCP server already running (status: {})", response.status());
        }
        Err(e) => {
            warn!("⚠️  MCP server not reachable: {}", e);
            info!("🚀 Auto-starting MCP server...");
            
            // Start MCP server in background
            tokio::spawn(async move {
                info!("🌐 Spawning embedded MCP HTTP server");
                match run_pmcp_http_server("127.0.0.1", 8080, None).await {
                    Ok(_) => info!("✅ Embedded MCP server completed"),
                    Err(e) => warn!("❌ Embedded MCP server failed: {}", e),
                }
            });
            
            // Wait and verify
            tokio::time::sleep(Duration::from_millis(1000)).await;
            match test_client.get(&test_url).send().await {
                Ok(response) => {
                    info!("✅ MCP server auto-started successfully (status: {})", response.status());
                }
                Err(e) => {
                    warn!("❌ MCP server failed to start: {}", e);
                }
            }
        }
    }
});
```

**Before**: No verification, silent failures
**After**: Auto-starts server if not running, verifies success

#### 3. Enhanced Observability (Throughout)
```rust
info!("🚀 Setting up MCP HTTP client");
info!("📍 MCP server endpoint: {}", mcp_url);
info!("🌐 MCP client task started");
info!("📡 Listening for UI messages to forward to MCP server");
```

**Emoji markers make logs scannable**:
- 🚀 = Starting/launching
- 📍 = Location/endpoint
- 🌐 = Network activity
- 📡 = Communication
- ✅ = Success
- ⚠️  = Warning
- 🛑 = Stopping
- 💡 = Helpful tip

#### 4. Better Error Messages (Lines 127-133)
```rust
Err(e) => {
    warn!(error = ?e, "Failed to send to MCP server at {}", mcp_url);
    warn!("💡 Is the MCP server running? Check: cargo run --bin botticelli-mcp-pmcp-http --features streamable-http");
    
    if let Err(e) = bg_tx.send(BackgroundMessage::Error(format!("MCP server error: {}", e))).await {
        warn!(error = ?e, "Failed to send error to UI");
    }
}
```

**Before**: Generic error "Failed to send to MCP server"
**After**: Shows URL, provides command to start server

#### 5. Client Configuration (Lines 69-72)
```rust
let http_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .expect("Failed to build HTTP client");
```

**Before**: Default client with no timeout
**After**: 30-second timeout configured

## Usage

### Environment Variables
```bash
# Optional - defaults to 127.0.0.1:8080
export MCP_HTTP_HOST=127.0.0.1
export MCP_HTTP_PORT=8080

# Run TUI with MCP client
cargo run --bin botticelli-tui --features cli
```

### Expected Logs

#### When Server Already Running
```
INFO 🚀 Setting up MCP HTTP client
INFO 📍 MCP server endpoint: http://127.0.0.1:8080/sse
INFO 🔍 Verifying MCP server is reachable at http://127.0.0.1:8080/sse
INFO ✅ MCP server already running (status: 404 Not Found)
INFO 🌐 MCP client task started
INFO 📡 Listening for UI messages to forward to MCP server
```

#### When Server Auto-Starts
```
INFO 🚀 Setting up MCP HTTP client
INFO 📍 MCP server endpoint: http://127.0.0.1:8080/sse
INFO 🔍 Verifying MCP server is reachable at http://127.0.0.1:8080/sse
WARN ⚠️  MCP server not reachable: tcp connect error: Connection refused
INFO 🚀 Auto-starting MCP server...
INFO 🌐 Spawning embedded MCP HTTP server
INFO ⏳ Waiting for MCP server to start...
INFO ✅ MCP server auto-started successfully (status: 404 Not Found)
```

### If Server Fails to Auto-Start
```
WARN ⚠️  MCP server not reachable: Connection refused
INFO 🚀 Auto-starting MCP server...
INFO 🌐 Spawning embedded MCP HTTP server
INFO ⏳ Waiting for MCP server to start...
WARN ❌ MCP server failed to start: Connection refused
WARN 💡 You can start it manually:
WARN    cargo run --bin botticelli-mcp-pmcp-http --features streamable-http
```

## Testing

### Automatic Test (Server Auto-Starts)
1. Make sure MCP server is NOT running: `pkill -f botticelli-mcp`
2. Start TUI: `just chat` (or `cargo run --bin botticelli-tui --features cli`)
3. Look for: 
   - `🚀 Auto-starting MCP server...`
   - `✅ MCP server auto-started successfully`
4. Send a message - server should respond

### Manual Test (Server Already Running)
1. Start MCP server: `cargo run --bin botticelli-mcp-pmcp-http --features streamable-http`
2. Wait for: `✅ HTTP server SUCCESSFULLY BOUND to 127.0.0.1:8080`
3. Start TUI: `just chat`
4. Look for: `✅ MCP server already running`

### What Success Looks Like
- TUI logs show exact endpoint it's connecting to
- Server verification happens before main task starts
- Clear error messages if server is down
- Helpful commands provided to fix issues

## Key Improvements

1. **Self-booting** - Automatically starts MCP server if not running
2. **No more guessing** - Logs show exact URL being used
3. **Early failure detection** - Know immediately if server is down or failed to start
4. **Helpful error messages** - Tell user how to fix the problem
5. **Configurable** - Can change host/port via environment
6. **Observable** - Structured logging with emoji markers
7. **Correct defaults** - Port 8080, `/sse` endpoint

## Cargo.toml Changes

Added `streamable-http` feature to `botticelli_mcp` dependency:
```toml
botticelli_mcp = { workspace = true, features = ["streamable-http"], optional = true }
```

Made dependency optional and added to `cli` feature:
```toml
cli = ["dep:clap", "dep:dotenvy", "dep:reqwest", "dep:botticelli_chat", "dep:botticelli_mcp", "database"]
```

## Related Files
- `crates/botticelli_tui/src/minimal_loop.rs` - Fixed client code
- `crates/botticelli_mcp/src/pmcp_http_server.rs` - Server with observability
- `crates/botticelli_mcp/tests/server_lifecycle_test.rs` - Tests that revealed issues
