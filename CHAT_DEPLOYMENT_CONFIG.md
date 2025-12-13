# Chat Deployment Configuration System

## Overview

Flexible configuration system for deploying Botticelli Chat in either local development or containerized production environments using a single `mode` flag.

## Quick Start

### Local Development
```bash
just chat-local        # Automatic service startup
just chat-local-check  # With health checks
```

### Container Deployment
```bash
just chat-container    # Run in container mode
```

## Configuration Architecture

### Precedence Order
1. **CLI Flags** - Highest priority, overrides all
2. **Environment Variables** - `.env` file
3. **TOML Config** - `chat.local-dev.toml`, `chat.container.toml`, etc.
4. **Defaults** - Fallback values

### Environment Modes

**Local Mode** (`mode = "Local"`):
- PostgreSQL on `localhost:5432`
- MCP server on `localhost:3000`
- Automatic service startup
- Database auto-creation
- Migrations auto-run

**Container Mode** (`mode = "Container"`):
- PostgreSQL on `postgres:5432`
- MCP server on `mcp-server:3000`
- Expects services in containers
- No auto-startup attempts

## Configuration Files

### Local Development (`chat.local-dev.toml`)
```toml
[environment]
mode = "Local"

[postgres]
host = "localhost"
port = 5432
database = "botticelli"
user = "postgres"

[mcp_server]
host = "localhost"
port = 3000

[mcp_client]
max_iterations = 10
tool_timeout_seconds = 30
```

### Container (`chat.container.toml`)
```toml
[environment]
mode = "Container"

[postgres]
host = "postgres"
port = 5432
database = "botticelli"
user = "postgres"

[mcp_server]
host = "mcp-server"
port = 3000
```

## Automatic Startup Sequence

When running `just chat-local`, the system automatically:

### 1. PostgreSQL Setup
- ✅ Checks if PostgreSQL is accessible
- ✅ Creates database if missing
- ✅ Runs diesel migrations if needed
- ❌ Fails with clear instructions if PostgreSQL not running

### 2. MCP Server Setup  
- ✅ Checks if MCP server is reachable
- ✅ Starts MCP server in background if needed (local mode only)
- ✅ Waits for server initialization
- ✅ Verifies health endpoint
- ❌ Fails with instructions if cannot start

### 3. Service Integration
- Database connection pooling
- MCP client initialization
- Narrative repository setup
- All connections validated

## Required Dependencies

### Runtime Requirements
- **PostgreSQL** - Must be running (`sudo systemctl start postgresql`)
- **MCP Server** - Auto-started in local mode
- **Diesel CLI** - For migrations: `cargo install diesel_cli --no-default-features --features postgres`

### Container Requirements
- PostgreSQL container
- MCP server container
- Network connectivity between containers

## Environment Variables

Override any config value via environment:

```bash
export DATABASE_URL="postgresql://user:pass@host:port/db"
export MCP_SERVER_URL="http://mcp-server:3000"
export ENVIRONMENT_MODE="Local"  # or "Container"
```

## CLI Flags

```bash
botticelli-chat --mode local             # Force local mode
botticelli-chat --postgres-host custom   # Override postgres host
botticelli-chat --mcp-port 8080          # Override MCP port
botticelli-chat --skip-health-checks     # Skip startup checks
```

## Troubleshooting

### PostgreSQL Connection Failed
```
Error: PostgreSQL connection failed: Connection refused

Please ensure PostgreSQL is running:
  sudo systemctl start postgresql
```

**Fix**: Start PostgreSQL service

### MCP Server Not Running
```
Error: MCP server not reachable

Starting MCP server in background...
```

**Local Mode**: Auto-starts MCP server
**Container Mode**: Ensure `mcp-server` container is running

### Database Does Not Exist
```
Info: Database does not exist, creating
Info: Database created successfully
Info: Running database migrations
```

**Auto-fixed**: System creates database and runs migrations

### Migrations Failed
```
Error: Failed to run migrations

Please install diesel_cli:
  cargo install diesel_cli --no-default-features --features postgres
```

**Fix**: Install diesel CLI tool

## Architecture Benefits

### For Development
- **Fast Iteration** - No container rebuilds
- **Direct Debugging** - Access to local services
- **Transparent Logs** - See all service output
- **Quick Troubleshooting** - Can restart services independently

### For Production
- **Isolation** - Services in containers
- **Portability** - Same config works anywhere
- **Scalability** - Container orchestration ready
- **Security** - Network isolation

### Single Flag Deploy
```bash
# Development
just chat-local

# Staging
just chat-staging

# Production  
just chat-production
```

All use same codebase, different config file.

## Implementation Status

### ✅ Completed (Phase C)
- [x] Config system with TOML support
- [x] Environment mode detection
- [x] Local/container differentiation
- [x] Automatic PostgreSQL setup
- [x] Automatic MCP server startup
- [x] Database auto-creation
- [x] Migration auto-run
- [x] Health check system
- [x] Service container
- [x] Connection pooling
- [x] CLI flag support

### 🚧 In Progress
- [ ] HTTP MCP server wrapper (axum)
- [ ] Container orchestration files
- [ ] Kubernetes manifests
- [ ] Production hardening

### 📋 Planned
- [ ] Config validation
- [ ] Secret management
- [ ] Multi-region support
- [ ] Auto-scaling config

## Files

**Core Config**:
- `crates/botticelli_chat/src/config/` - Config system implementation
- `crates/botticelli_chat/src/startup.rs` - Automatic service setup
- `crates/botticelli_chat/src/services.rs` - Lazy service initialization

**Config Files**:
- `chat.local-dev.toml` - Local development
- `chat.container.toml` - Container deployment
- `chat.staging.toml` - Staging environment
- `chat.toml` - Default values

**Binaries**:
- `crates/botticelli_chat/src/bin/chat.rs` - CLI entry point
- `crates/botticelli_mcp_server/src/bin/http_server.rs` - HTTP MCP wrapper

## Related Documentation

- `CHAT_CONFIG_COMPLETE.md` - Full config system design
- `CHAT_SYSTEM_COMPLETE.md` - Chat architecture overview
- `MCP.md` - MCP protocol details
- `POSTGRES.md` - Database setup guide
