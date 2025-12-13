# Chat MCP Integration - Findings & Next Steps

## Current Status

### ✅ What Works
- Configuration system loads correctly
- Chat interface starts without errors
- Error handling when MCP server unavailable (no crash!)
- Clear error message: "Cannot connect to MCP server at http://localhost:3000"

### ❌ What's Missing
- **MCP HTTP server not starting automatically**
- No process management for MCP server lifecycle
- Integration test confirms this (ignored test would fail)

## The Problem

When user types `create narrative`:
```
2025-12-09T00:59:12.031102Z ERROR: HTTP request failed 
error=reqwest::Error { kind: Request, url: "http://localhost:3000/tools/call", 
source: ConnectError("tcp connect error", 127.0.0.1:3000, 
Os { code: 111, kind: ConnectionRefused, message: "Connection refused" })) }
```

**Root Cause:** The MCP HTTP server (`botticelli-mcp-server-http`) is not running.

## Solutions

### Option A: Manual Startup (Current)
User must manually start MCP server:
```bash
# Terminal 1
cargo run --bin botticelli-mcp-server-http

# Terminal 2  
just chat-local
```

**Pros:** Simple, explicit control  
**Cons:** Requires two terminals, easy to forget

### Option B: Auto-Start as Child Process (Recommended)
Chat interface spawns MCP server as child:

```rust
// In startup.rs
async fn ensure_mcp_server_running(config: &ChatAppConfig) -> ChatResult<()> {
    let port = config.mcp_server.port;
    
    // Check if already running
    if tokio::net::TcpStream::connect(format!("localhost:{}", port))
        .await
        .is_ok()
    {
        info!("MCP server already running on port {}", port);
        return Ok(());
    }
    
    // Start as child process
    info!("Starting MCP HTTP server on port {}...", port);
    let mut cmd = tokio::process::Command::new("cargo")
        .args(&["run", "--bin", "botticelli-mcp-server-http", "--", 
                "--port", &port.to_string()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| ChatError::new(ChatErrorKind::IoError(
            format!("Failed to start MCP server: {}", e)
        )))?;
    
    // Wait for server to be ready (with timeout)
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if tokio::net::TcpStream::connect(format!("localhost:{}", port))
            .await
            .is_ok()
        {
            info!("MCP server ready");
            return Ok(());
        }
    }
    
    Err(ChatError::new(ChatErrorKind::IoError(
        "MCP server failed to start within 3 seconds".to_string()
    )))
}
```

**Pros:** Just works™, one command  
**Cons:** Process management complexity

### Option C: systemd/Daemon Service
MCP server runs as background service:

```bash
# One-time setup
just install-mcp-server  # Creates systemd service

# Then just use chat
just chat-local
```

**Pros:** Professional, production-ready  
**Cons:** Requires system setup, harder for development

### Option D: Integrated Server (Not Recommended)
Embed MCP server into chat binary:

**Pros:** Single process  
**Cons:** Violates separation of concerns, breaks container model

## Recommended Approach

**Short term (Development):** Option A - Manual startup with clear docs  
**Medium term (Local polish):** Option B - Auto-start child process  
**Long term (Production):** Option C - systemd service

## Implementation Plan

### Phase 1: Documentation (Immediate)
Update `README.md` and `CHAT_CONFIG_INTEGRATION_COMPLETE.md`:

```bash
# Local Development
# Terminal 1: Start MCP server
cargo run --bin botticelli-mcp-server-http

# Terminal 2: Start chat
just chat-local
```

### Phase 2: Auto-Start (Next Session)
1. Create `ensure_mcp_server_running()` in `startup.rs`
2. Add process handle to `ServiceContainer`
3. Cleanup on exit
4. Update integration test to verify

### Phase 3: Container (Later)
Docker Compose handles process orchestration:
```yaml
services:
  mcp-server:
    build: .
    command: botticelli-mcp-server-http
    
  chat:
    build: .
    command: botticelli-chat --mode container
    depends_on:
      - mcp-server
      - postgres
```

## Testing Strategy

### Integration Test Enhancement
```rust
#[tokio::test]
#[cfg(feature = "cli")]
async fn test_full_narrative_creation() {
    // Start MCP server
    let _server = spawn_mcp_server(3000).await.unwrap();
    
    // Create chat with services
    let config = ChatAppConfig::builder()
        .mode(EnvironmentMode::Local)
        .build()
        .unwrap();
    
    let services = Arc::new(ServiceContainer::new(config));
    let executor = CommandExecutor::with_services(services.clone());
    
    // Execute create narrative command
    let command = Command::Narrative(NarrativeCommand::Create {
        prompt: "sci-fi adventure".to_string(),
    });
    
    let response = executor.execute(command).await;
    assert!(response.is_ok(), "Create should succeed: {:?}", response);
}
```

## Current Workaround

Until auto-start is implemented:

```bash
# Terminal 1: Start dependencies
just postgres-start  # If not already running
cargo run --bin botticelli-mcp-server-http

# Terminal 2: Use chat
just chat-local
```

Or skip health checks and see errors inline:
```bash
just chat-local
# Prints clear error when MCP unavailable
# "Cannot connect to MCP server at http://localhost:3000"
```

## Error Message Quality

The current error is excellent:
```
System: Error: IO error: Cannot connect to MCP server at http://localhost:3000. 
Is it running? Check BOTTICELLI__MCP_SERVER__HOST and BOTTICELLI__MCP_SERVER__PORT
```

**Suggestions for improvement:**
1. Add hint about manual startup command
2. Link to documentation
3. Offer to auto-start (interactive mode)

## Summary

The system is working as designed - configuration, error handling, and service isolation are all correct. The missing piece is **MCP server process management**, which we should implement in the next session to create a seamless single-command experience.

For now, users must manually start the MCP HTTP server before using chat's narrative creation features.
