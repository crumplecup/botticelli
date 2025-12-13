# Botticelli Chat - Troubleshooting Guide

Common issues and their solutions for the Botticelli Chat system.

## Table of Contents

- [Quick Diagnostics](#quick-diagnostics)
- [Database Issues](#database-issues)
- [MCP Server Issues](#mcp-server-issues)
- [Configuration Issues](#configuration-issues)
- [Performance Issues](#performance-issues)
- [Container Issues](#container-issues)

---

## Quick Diagnostics

### Check System Status

```bash
# Check all services are running
docker-compose ps

# Check logs for errors
docker-compose logs --tail=50

# Check database connection
psql postgresql://botticelli:botticelli@localhost:5432/botticelli -c "SELECT 1"

# Check MCP server
curl http://localhost:3001/health
```

### Common Symptoms

| Symptom | Likely Cause | Quick Fix |
|---------|-------------|-----------|
| "Connection refused" | Service not running | `docker-compose up -d` |
| "Authentication failed" | Wrong credentials | Check `.env` or config |
| "Timeout" | Service overloaded | Increase timeout in config |
| "Not found" | Wrong service name | Check docker-compose.yml |

---

## Database Issues

### Issue: Cannot Connect to Database

**Error:**
```
connection to server at "localhost", port 5432 failed: Connection refused
```

**Causes:**
1. PostgreSQL not running
2. Wrong host/port
3. Firewall blocking connection
4. Using wrong config (local vs container)

**Solutions:**

```bash
# Check if PostgreSQL is running
docker-compose ps postgres

# Start PostgreSQL if stopped
docker-compose up -d postgres

# Check PostgreSQL logs
docker-compose logs postgres | tail -20

# Verify connection from host
psql postgresql://botticelli:botticelli@localhost:5432/botticelli

# Verify connection from container
docker-compose exec chat psql postgresql://botticelli:botticelli@postgres:5432/botticelli
```

**Configuration:**
- **Local dev**: Use `localhost:5432` in config
- **Container**: Use `postgres:5432` (service name)

### Issue: Authentication Failed

**Error:**
```
FATAL: password authentication failed for user "botticelli"
```

**Solutions:**

```bash
# Check environment variables
echo $DATABASE_URL

# Verify credentials in config
cat chat.local-dev.toml | grep -A 5 "\[database\]"

# Reset database password
docker-compose down
docker volume rm botticelli_postgres_data  # WARNING: Deletes all data!
docker-compose up -d postgres

# Re-run migrations
diesel migration run
```

### Issue: Migration Failed

**Error:**
```
diesel migration run failed: table already exists
```

**Solutions:**

```bash
# Check migration status
diesel migration list

# Revert last migration
diesel migration revert

# Revert all and start fresh (WARNING: Deletes data!)
diesel migration redo

# Check which migrations are pending
diesel migration pending

# Run specific migration
cd migrations/2025-12-08-212100-0000_create_bot_configs
diesel migration run
```

---

## MCP Server Issues

### Issue: MCP Server Not Responding

**Error:**
```
Error connecting to MCP server at http://localhost:3001
```

**Solutions:**

```bash
# Check if MCP server is running
docker-compose ps mcp-server

# Start MCP server
docker-compose up -d mcp-server

# Check MCP server logs
docker-compose logs mcp-server --tail=50

# Test health endpoint
curl http://localhost:3001/health

# Check from within container network
docker-compose exec chat curl http://mcp-server:3001/health
```

### Issue: MCP Request Timeout

**Error:**
```
MCP request timed out after 60 seconds
```

**Causes:**
1. Large narrative generation taking too long
2. LLM API rate limiting
3. Network issues
4. MCP server overloaded

**Solutions:**

```bash
# Increase timeout in config
[mcp]
timeout_seconds = 120  # Increase from 60 to 120

# Check MCP server resource usage
docker stats mcp-server

# Reduce narrative complexity
# Use smaller models (gemini-1.5-flash instead of gemini-1.5-pro)

# Check LLM API status
# Visit status pages for your providers
```

### Issue: Invalid MCP Response

**Error:**
```
Failed to parse MCP response: expected JSON object
```

**Solutions:**

```bash
# Check MCP server logs for errors
docker-compose logs mcp-server | grep ERROR

# Verify MCP server is healthy
curl -X POST http://localhost:3001/tools/call \
  -H "Content-Type: application/json" \
  -d '{"name":"generate_narrative","arguments":{"topic":"test"}}'

# Restart MCP server
docker-compose restart mcp-server

# Check MCP client configuration
# Ensure server_url is correct in config
```

---

## Configuration Issues

### Issue: Config File Not Found

**Error:**
```
Config file not found: chat.toml
```

**Solutions:**

```bash
# Specify config file explicitly
botticelli-chat --config chat.local-dev.toml

# Use environment variable
export BOTTICELLI_CONFIG=chat.local-dev.toml
botticelli-chat

# Check current directory
ls -la *.toml

# Use absolute path
botticelli-chat --config /path/to/chat.local-dev.toml
```

### Issue: Invalid Configuration

**Error:**
```
Configuration validation failed: missing required field 'database.url'
```

**Solutions:**

```bash
# Validate config file syntax
toml-cli check chat.local-dev.toml

# Check for typos in field names
cat chat.local-dev.toml

# Use example config as template
cp chat.local-dev.toml my-config.toml
# Edit my-config.toml with your settings

# Environment variable override
export DATABASE_URL="postgresql://botticelli:botticelli@localhost:5432/botticelli"
botticelli-chat
```

### Issue: Config Precedence Confusion

**Problem:** "I set the config in TOML but it's using different values"

**Precedence Order:**
1. CLI flags (highest priority)
2. Environment variables (`.env`)
3. Config file (TOML)
4. Default values (lowest priority)

**Check effective configuration:**

```bash
# See what config is actually being used
RUST_LOG=debug botticelli-chat 2>&1 | grep "Loaded config"

# Override with environment variable
export DATABASE_URL="postgresql://other:other@localhost:5432/other"
botticelli-chat  # Uses environment variable, not TOML

# Override with CLI flag
botticelli-chat --database-url "postgresql://cli:cli@localhost:5432/cli"  # Highest priority
```

---

## Performance Issues

### Issue: Slow Narrative Generation

**Symptoms:**
- Requests taking 30+ seconds
- High CPU usage
- Memory growing

**Solutions:**

```bash
# Use faster models
[llm]
default_model = "gemini-1.5-flash"  # Fast
# default_model = "gemini-1.5-pro"  # Slow but better quality

# Reduce max_tokens
max_tokens = 500  # Instead of 2000

# Enable response streaming
streaming = true

# Check rate limiting
# You may be hitting API rate limits

# Monitor resource usage
docker stats botticelli-chat
```

### Issue: Database Slow Queries

**Solutions:**

```bash
# Enable query logging
[database]
log_queries = true

# Check database size
docker-compose exec postgres psql -U botticelli -d botticelli -c "\dt+"

# Vacuum database
docker-compose exec postgres psql -U botticelli -d botticelli -c "VACUUM ANALYZE"

# Check for missing indexes
# Look at migrations - all important fields should be indexed

# Increase connection pool
[database]
max_connections = 20  # Increase from 5
```

### Issue: Memory Leak

**Symptoms:**
- Memory usage growing over time
- System becomes unresponsive

**Solutions:**

```bash
# Monitor memory usage
docker stats --no-stream

# Check for memory leaks in logs
docker-compose logs | grep "out of memory"

# Restart services
docker-compose restart

# Reduce cache size
[cache]
max_size_mb = 100  # Reduce cache size

# Enable connection pool limits
[database]
max_connections = 10
connection_timeout_seconds = 30
```

---

## Container Issues

### Issue: Container Won't Start

**Error:**
```
Error starting userland proxy: listen tcp4 0.0.0.0:5432: bind: address already in use
```

**Solutions:**

```bash
# Check what's using the port
lsof -i :5432
netstat -tulpn | grep 5432

# Stop conflicting service
sudo systemctl stop postgresql  # If you have local PostgreSQL

# Use different port in docker-compose.yml
ports:
  - "5433:5432"  # Map to different host port

# Update config to use new port
[database]
url = "postgresql://botticelli:botticelli@localhost:5433/botticelli"
```

### Issue: Volume Permission Denied

**Error:**
```
mkdir: cannot create directory '/var/lib/postgresql/data': Permission denied
```

**Solutions:**

```bash
# Fix volume permissions
docker-compose down
docker volume rm botticelli_postgres_data
docker-compose up -d

# Or set explicit permissions
sudo chown -R 999:999 ./data/postgres  # 999 = postgres user in container

# Use named volume (recommended)
# In docker-compose.yml:
volumes:
  postgres_data:  # Named volume

volumes:
  - postgres_data:/var/lib/postgresql/data
```

### Issue: Container Networking

**Error:**
```
Could not resolve host: postgres
```

**Solutions:**

```bash
# Check container network
docker network ls
docker network inspect botticelli_default

# Verify containers are on same network
docker-compose ps

# Restart networking
docker-compose down
docker-compose up -d

# Use explicit network in docker-compose.yml
networks:
  botticelli:
    name: botticelli-network

services:
  postgres:
    networks:
      - botticelli
```

---

## Debug Mode

### Enable Verbose Logging

```bash
# Maximum verbosity
export RUST_LOG=trace
botticelli-chat

# Module-specific logging
export RUST_LOG=botticelli_chat=debug,botticelli_mcp=trace
botticelli-chat

# With timestamps
export RUST_LOG_FORMAT=pretty
botticelli-chat
```

### Enable Distributed Tracing

```bash
# Start Jaeger
docker-compose up -d jaeger

# Check traces at http://localhost:16686

# Enable tracing in config
[observability]
tracing_enabled = true
jaeger_endpoint = "http://localhost:14268/api/traces"

# View traces for specific request
# Look for span names like:
# - handle_create_narrative
# - mcp_client::call_tool
# - database::query
```

---

## Getting Help

### Information to Include

When reporting issues, include:

```bash
# 1. System information
uname -a
docker --version
docker-compose --version

# 2. Service status
docker-compose ps

# 3. Recent logs
docker-compose logs --tail=100 > logs.txt

# 4. Configuration (remove sensitive data!)
cat chat.local-dev.toml

# 5. Error message (full stack trace)

# 6. Steps to reproduce
```

### Useful Commands

```bash
# Get all logs
docker-compose logs > full-logs.txt

# Follow logs in real-time
docker-compose logs -f

# Restart everything
docker-compose restart

# Nuclear option (WARNING: Deletes data!)
docker-compose down -v
docker-compose up -d
diesel migration run
```

---

## Common Workflows

### Fresh Start

```bash
# Complete reset (WARNING: Deletes all data!)
docker-compose down -v
docker volume prune -f
docker-compose up -d
diesel migration run
```

### Switching Between Local and Container

```bash
# Local development
export BOTTICELLI_CONFIG=chat.local-dev.toml
botticelli-chat

# Container deployment
export BOTTICELLI_CONFIG=chat.container.toml
docker-compose up -d
```

### Running Migrations

```bash
# Check pending migrations
diesel migration list

# Run all pending
diesel migration run

# Revert last migration
diesel migration revert

# Redo all migrations (WARNING: Deletes data!)
diesel migration redo
```

---

## Still Stuck?

1. Check the logs: `docker-compose logs --tail=100`
2. Enable debug mode: `RUST_LOG=debug`
3. Verify services are running: `docker-compose ps`
4. Check configuration: `cat chat.local-dev.toml`
5. Review this guide again
6. Open an issue on GitHub with full details
