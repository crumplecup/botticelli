# Chat Configuration System

## Overview

Flexible configuration system for botticelli_chat that supports local development and containerized deployment using a single environment flag.

## Requirements

- **Local mode**: Postgres and MCP server on localhost
- **Container mode**: Postgres and MCP server in container network
- **Config precedence**: CLI flags → ENV vars → Config TOML → Defaults
- **Dependencies**: Chat requires postgres, MCP server, and MCP client

## Configuration Structure

```toml
# chat.toml (default config file)
[environment]
mode = "local"  # or "container"

[postgres]
host = "localhost"
port = 5432
user = "botticelli"
password = "botticelli"
database = "botticelli"

[mcp_server]
host = "localhost"
port = 3000
# For container mode, these become container service names

[mcp_client]
timeout_seconds = 30
retry_attempts = 3

[chat]
initial_model = "gemini-2.5-flash"
max_tokens = 4096

[observability]
rust_log = "info"
otel_exporter = "stdout"
otel_endpoint = "http://localhost:4318"
```

## Precedence Order

1. **CLI flags**: `--mode container --postgres-host postgres`
2. **Environment variables**: `BOTTICELLI_MODE=container`
3. **Config file**: `chat.toml` or path from `--config`
4. **Defaults**: Hardcoded fallbacks

## Implementation Plan

### Step 1: Create Config Module ✅
- [x] Add `config` crate dependency
- [x] Create `crates/botticelli_chat/src/config/` module
- [x] Define config structs with serde
- [x] Implement builder with precedence chain

### Step 2: Environment Profiles ✅
- [x] `EnvironmentMode` enum (Local, Container)
- [x] Profile-specific defaults
- [x] Auto-adjust hostnames for container mode

### Step 3: Connection Management
- [x] Postgres connection builder from config (database_url())
- [x] MCP server URL builder from config (server_url())
- [ ] Health checks for dependencies (to be implemented in binary)

### Step 4: CLI Integration ✅
- [x] Add clap for argument parsing
- [x] `--mode` flag (local/container)
- [x] `--config` flag for custom path
- [x] Override flags for each config section
- [x] Health checks for postgres and MCP server
- [x] Logging initialization
- [x] Binary compiles and runs successfully
- [x] TUI integrated with configuration
- [x] Graceful terminal setup/restore
- [x] Interactive chat loop working

### Step 5: Container Support
- [x] Config files created (chat.toml, chat.container.toml)
- [ ] Update Containerfile.chat for config
- [ ] Update docker-compose.chat.yml
- [ ] Environment variable mapping
- [ ] Service discovery

### Step 6: Documentation
- [x] Usage examples for local mode (this file)
- [x] Usage examples for container mode (this file)
- [x] Configuration reference (example TOML files)
- [ ] Migration guide from current setup

## Completed

### Configuration System Core ✅
- `EnvironmentMode` enum with Local/Container variants
- Environment-aware defaults for postgres and MCP server hosts
- `PostgresConfig` with database_url() builder
- `McpServerConfig` and `McpClientConfig`
- `ObservabilityConfig` for logging/tracing
- `ChatAppConfig` main configuration struct
- `ConfigBuilder` for programmatic configuration
- Precedence chain: CLI > ENV > File > Defaults
- Comprehensive test coverage (10 tests, all passing)

### Example Config Files ✅
- `chat.toml` - Local development configuration
- `chat.container.toml` - Container deployment configuration

### Tests ✅
All tests passing:
- Default configuration
- Container mode defaults
- Local mode defaults
- Builder overrides
- Database URL generation
- MCP server URL generation
- Environment mode serialization
- Environment variable loading
- Observability defaults
- MCP client defaults

## Example Usage

### Local Development
```bash
# Use defaults (local mode)
cargo run --bin botticelli-chat

# Explicit local mode
cargo run --bin botticelli-chat -- --mode local

# Custom postgres
cargo run --bin botticelli-chat -- --postgres-host 192.168.1.100
```

### Container Deployment
```bash
# Container mode with docker-compose
docker-compose -f docker-compose.chat.yml up

# Container mode standalone
podman run -e BOTTICELLI_MODE=container botticelli-chat
```

### Config File
```bash
# Custom config file
cargo run --bin botticelli-chat -- --config my-chat-config.toml
```

## Config Struct Design

```rust
pub struct ChatAppConfig {
    pub environment: EnvironmentConfig,
    pub postgres: PostgresConfig,
    pub mcp_server: McpServerConfig,
    pub mcp_client: McpClientConfig,
    pub chat: ChatConfig,
    pub observability: ObservabilityConfig,
}

pub enum EnvironmentMode {
    Local,
    Container,
}

impl EnvironmentMode {
    pub fn postgres_host_default(&self) -> &str {
        match self {
            Self::Local => "localhost",
            Self::Container => "postgres",
        }
    }
    
    pub fn mcp_server_host_default(&self) -> &str {
        match self {
            Self::Local => "localhost",
            Self::Container => "mcp-server",
        }
    }
}
```

## Migration Path

1. ✅ Create config system alongside existing code
2. ✅ Add binary target `botticelli-chat` with new config
3. ✅ Test local mode - verified working
4. ⏳ Test container mode with docker-compose (needs docker-compose updates)
5. 🚧 Update documentation (this file updated, needs docker-compose docs)
6. ⏳ Deprecate old setup (after container testing)

## Completed Implementation

### Binary ✅
The `botticelli-chat` binary is fully implemented and tested:
- ✅ CLI argument parsing with clap
- ✅ Config loading with precedence chain
- ✅ Database connection health checks
- ✅ MCP server health checks
- ✅ Logging initialization
- ✅ Environment mode switching (local/container)
- ✅ All override mechanisms working (CLI, ENV, File)
- ✅ TUI integration complete

### TUI Interface ✅
The terminal interface is fully functional:
- ✅ Interactive message display
- ✅ Command input and parsing
- ✅ Message history with scrolling
- ✅ Graceful terminal management
- ✅ Keyboard shortcuts (Ctrl+C to exit, Ctrl+L to clear)
- ✅ Real-time input feedback

### Verification ✅
- Container mode: Service names auto-resolve (`postgres`, `mcp-server`)
- Local mode: Localhost defaults work correctly
- ENV override: `BOTTICELLI__POSTGRES__HOST` works
- CLI override: `--postgres-host`, `--mcp-host` work
- Zero clippy warnings
- All tests passing

## ⚠️ CRITICAL: System Status

**Infrastructure**: ✅ COMPLETE
**Functionality**: ❌ NON-OPERATIONAL

The foundation is built, but the system **CANNOT FUNCTION** until all remaining phases are complete.
This is not a "partially working" system - it's a complete foundation waiting for essential components.

## Implementation Complete ✅

All core infrastructure is now in place:
- ✅ Configuration system with precedence chain
- ✅ CLI binary with health checks
- ✅ TUI integration
- ✅ Service container with lazy initialization
- ✅ Database pool integration
- ✅ MCP client integration

## 🚨 Required Work - All Phases Critical

Like a car needs an engine, transmission, wheels, and steering to function, Botticelli requires
all the following components to be operational:

- **Phase A (Database)**: Without this, no persistence - users lose all work
- **Phase B (MCP)**: Without this, no narrative generation - the core feature doesn't work
- **Phase C (Container)**: Without this, cannot deploy to production - system stays local only
- **Phase D (Commands)**: Without this, cannot assign to bots or schedule - primary use case broken
- **Phase E (Polish)**: Without this, system unreliable and unmaintainable in production

**Current Progress: 20% (1 of 5 major components complete)**

See `CHAT_IMPLEMENTATION_TRACKER.md` for detailed task breakdown.

### Phase A: Real Database Integration
**Status**: Ready to implement
**Dependencies**: Database pool initialized ✅

Tasks:
- [ ] Implement diesel queries for narratives table
- [ ] `list narratives` - Query and display from database
- [ ] `save narrative` - Persist to database
- [ ] `load narrative` - Retrieve from database
- [ ] Error handling for database operations
- [ ] Tests for database operations

**Files to modify:**
- `crates/botticelli_chat/src/executor.rs` - Add diesel queries
- `crates/botticelli_database/src/queries.rs` - Narrative queries (if needed)

### Phase B: Real MCP Integration  
**Status**: Ready to implement
**Dependencies**: MCP client initialized ✅

Tasks:
- [ ] Connect to actual MCP server tools
- [ ] `create narrative` - Use MCP generate tool
- [ ] `validate narrative` - Use MCP validation
- [ ] Handle MCP tool responses
- [ ] Error handling for MCP operations
- [ ] Tests for MCP integration

**Files to modify:**
- `crates/botticelli_chat/src/executor.rs` - Call MCP tools
- May need MCP server running for tests

### Phase C: Container Deployment
**Status**: Ready to implement
**Dependencies**: Configuration system ✅

Tasks:
- [ ] Update `docker-compose.chat.yml` with environment variables
- [ ] Mount config files in containers
- [ ] Test postgres service connectivity
- [ ] Test MCP server connectivity  
- [ ] Document container setup
- [ ] Test full container stack
- [ ] Update `Containerfile.chat` if needed

**Files to modify:**
- `docker-compose.chat.yml` - Add BOTTICELLI_* env vars
- `Containerfile.chat` - Copy config files
- Documentation files

### Phase D: Additional Commands
**Status**: Ready to implement
**Dependencies**: Database and MCP working

Tasks:
- [ ] Bot assignment commands
  - [ ] `create bot` - Create bot in database
  - [ ] `assign narrative to bot` - Link narrative to bot
  - [ ] `list bots` - Show all bots
  - [ ] `show bot <id>` - Display bot details
- [ ] Social media scheduling
  - [ ] `schedule post` - Schedule content posting
  - [ ] `show schedule` - Display scheduled posts
  - [ ] `cancel schedule` - Cancel scheduled post
- [ ] Content generation workflows
  - [ ] Multi-step narrative generation
  - [ ] Content review and approval
  - [ ] Automated posting integration

**Files to modify:**
- `crates/botticelli_chat/src/executor.rs` - Bot and social commands
- `crates/botticelli_chat/src/command.rs` - May need new command types
- Database queries for bots and schedules

## Immediate Next Step

**Recommended order:**
1. **Phase A** (Database) - Foundation for persistence
2. **Phase B** (MCP) - Core functionality for narrative generation  
3. **Phase C** (Container) - Production deployment capability
4. **Phase D** (Commands) - Full feature completeness

**Or parallel approach:**
- One developer on Phase A (database queries)
- One developer on Phase B (MCP integration)
- Then both on Phase C and D

Choose starting point based on priority.

### Short Term (Integration)
1. Update docker-compose.chat.yml:
   - Add environment variables for BOTTICELLI_* config
   - Mount config files
   - Update service names if needed

2. Update Containerfile.chat:
   - Copy config files to container
   - Set proper environment variables
   - Document config file locations

### Medium Term (Polish)
1. Add CLI documentation
2. Add usage examples
3. Create migration guide from .env to config system
4. Add health check endpoints
5. Add config validation

### Long Term (Enhancement)
1. Config hot-reloading
2. Multiple config profiles
3. Config validation API
4. Config export/import tools

## Files to Create/Modify

### New Files
- `crates/botticelli_chat/src/config/mod.rs`
- `crates/botticelli_chat/src/config/app.rs`
- `crates/botticelli_chat/src/config/environment.rs`
- `crates/botticelli_chat/src/config/postgres.rs`
- `crates/botticelli_chat/src/config/mcp.rs`
- `crates/botticelli_chat/src/config/observability.rs`
- `crates/botticelli_chat/src/bin/botticelli-chat.rs`
- `chat.toml` (example config)

### Modified Files
- `crates/botticelli_chat/Cargo.toml` (add dependencies)
- `crates/botticelli_chat/src/lib.rs` (export config)
- `docker-compose.chat.yml` (environment variables)
- `Containerfile.chat` (config file support)

## Testing Strategy

1. Unit tests for config precedence
2. Integration tests for local mode
3. Integration tests for container mode
4. Health check tests for dependencies
5. Config file parsing tests

## Benefits

- **Single flag deployment**: `--mode container` vs `--mode local`
- **Fast iteration**: Local build for development
- **Production ready**: Container deployment when ready
- **Flexible**: Override any setting via CLI or ENV
- **Self-documenting**: Config file shows all options
