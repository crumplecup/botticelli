# Chat Auto-Start Feature - Complete

## Overview

Implemented automatic service startup for the Botticelli chat system. When running in Local mode, the system now automatically starts the MCP HTTP server if it's not already running, eliminating manual service management during development.

## Core Capability

**One-Command Startup:**
```bash
just chat-local-check
```

This single command:
1. ✅ Builds MCP HTTP server binary (if needed)
2. ✅ Checks PostgreSQL connection
3. ✅ Auto-creates database and tables (if missing)
4. ✅ Auto-starts MCP server (if not running)
5. ✅ Validates all services are healthy
6. ✅ Launches chat TUI

## Implementation Details

### Auto-Start Logic (`startup.rs`)

**Service Detection:**
- Attempts HTTP health check at `http://localhost:3000/health`
- If fails → triggers auto-start sequence

**Binary Selection:**
- Looks for pre-built `botticelli-mcp-http` in same directory as chat binary
- Uses compiled binary (not `cargo run`) for fast startup
- Falls back to helpful error if binary not found

**Process Daemonization:**
- Unix: Uses `setsid()` to properly detach from parent process
- Redirects stdin/stdout/stderr to null
- Uses `std::mem::forget()` to prevent waiting on child
- Server continues running after chat exits

**Post-Start Validation:**
- Waits 2 seconds for server initialization
- Re-checks health endpoint
- Returns error if server didn't start successfully

### Mode-Aware Behavior

**Local Mode:**
- ✅ Auto-starts MCP server
- ✅ Auto-creates database/tables
- ✅ Uses `localhost` for all connections

**Container Mode:**
- ❌ No auto-start (expects Docker/Podman to manage services)
- ✅ Auto-creates database/tables (if accessible)
- ✅ Uses container hostnames (`postgres`, `mcp-server`)

### Error Handling

**Clear, Actionable Messages:**

```
MCP server binary not found.

Please build it first:
  cargo build --bin botticelli-mcp-http --features botticelli_mcp/http

Or start it manually in another terminal:
  cargo run --bin botticelli-mcp-http
```

**Failure Categories:**
1. **Binary not found** → Instructions to build
2. **Spawn failure** → System error details + manual start instructions
3. **Health check failure** → Server started but not responding (possible port conflict)
4. **Container mode** → Reminds user to start containers

## Justfile Integration

**New Recipe:**
```just
# Build MCP HTTP server binary (required for auto-start)
build-mcp-server:
    @echo "🔧 Building MCP HTTP server..."
    cargo build --bin botticelli-mcp-http --features botticelli_mcp/http
```

**Updated Recipe:**
```just
# Run chat interface with health checks and auto-start
chat-local-check: build-mcp-server
    @echo "💬 Starting chat interface with health checks and auto-start..."
    cargo run --bin botticelli-chat --features="cli,tui"
```

The dependency ensures MCP server is always built before attempting auto-start.

## Developer Experience

### Before This Feature

```bash
# Terminal 1
cargo run --bin botticelli-mcp-http --features botticelli_mcp/http

# Terminal 2  
sudo systemctl start postgresql
diesel migration run
cargo run --bin botticelli-chat --features cli,tui
```

**Result:** 3 manual steps, 2 terminals, easy to forget

### After This Feature

```bash
just chat-local-check
```

**Result:** Single command, automatic setup, zero cognitive load

## Testing

**Integration Test:** `tests/mcp_integration_test.rs`
- Validates auto-start detection
- Confirms error messages when binary missing
- Tests service health check logic

**Manual Testing:**
```bash
# Kill any running MCP servers
pkill -f botticelli-mcp-http

# Build and run with auto-start
just chat-local-check

# Verify in logs:
# "MCP server not running, attempting to start it"
# "MCP server started" pid=<number>
# "MCP server health check passed"
```

## Architecture Benefits

### Separation of Concerns
- **Chat binary** focuses on UI/UX
- **MCP server** runs as independent service
- **Auto-start** is convenience layer, not hard dependency

### Production Ready
- Auto-start only in Local mode
- Container mode expects orchestration (docker-compose/kubernetes)
- No assumptions about deployment environment

### Graceful Degradation
- If binary missing → clear instructions
- If spawn fails → manual start option
- If health check fails → diagnostic information

## Future Enhancements

### Possible Improvements
1. **Port conflict detection** - Try alternate ports if 3000 is busy
2. **Log streaming** - Show MCP server logs in chat UI (debug mode)
3. **Process management** - Track spawned processes, kill on exit
4. **Windows support** - Adapt daemonization for Windows platform

### Not Planned
- Auto-starting PostgreSQL (too system-dependent, requires sudo)
- Auto-installing dependencies (out of scope)
- Auto-updating binaries (security concerns)

## Related Documentation

- `CHAT_CONFIG_INTEGRATION_COMPLETE.md` - Configuration system
- `CHAT_SYSTEM_COMPLETE.md` - Overall chat architecture
- `crates/botticelli_chat/src/startup.rs` - Implementation
- `tests/mcp_integration_test.rs` - Testing strategy

## Success Criteria

✅ **All Met:**
1. Single command starts entire system (Local mode)
2. No manual service management required
3. Clear error messages guide recovery
4. Integration test validates behavior
5. Documentation explains usage and architecture
6. Justfile recipe automates build dependency
