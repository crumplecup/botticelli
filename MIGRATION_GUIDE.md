# Migration Guide: Environment Variables to Config Files

This guide helps you migrate from environment variable configuration to the new TOML-based configuration system.

## Why Migrate?

**Old System (Environment Variables):**
```bash
export DATABASE_URL="postgresql://..."
export MCP_SERVER_URL="http://..."
export RUST_LOG="debug"
# 20+ environment variables scattered across .env files
```

**New System (TOML Config):**
```toml
[database]
url = "postgresql://..."

[mcp]
server_url = "http://..."

[observability]
log_level = "debug"
```

**Benefits:**
- ✅ Single source of truth
- ✅ Type checking and validation
- ✅ Environment-specific configs (local, staging, production)
- ✅ Better documentation
- ✅ Easier to version control
- ✅ Clear precedence order

---

## Migration Steps

### Step 1: Identify Your Current Configuration

Check your existing environment variables:

```bash
# Check .env file
cat .env

# Check active environment
env | grep -E "DATABASE|MCP|RUST_LOG|OPENAI|ANTHROPIC"

# Common variables to look for:
# - DATABASE_URL
# - MCP_SERVER_URL
# - RUST_LOG
# - ANTHROPIC_API_KEY (still in .env)
# - GEMINI_API_KEY (still in .env)
```

### Step 2: Choose Your Configuration Template

**Local Development:**
```bash
cp chat.local-dev.toml my-chat.toml
```

**Container Deployment:**
```bash
cp chat.container.toml my-chat.toml
```

**Staging Environment:**
```bash
cp chat.staging.toml my-chat.toml
```

### Step 3: Map Environment Variables to TOML

| Old Environment Variable | New TOML Location | Example |
|-------------------------|-------------------|---------|
| `DATABASE_URL` | `[database].url` | `"postgresql://..."` |
| `MCP_SERVER_URL` | `[mcp].server_url` | `"http://localhost:3001"` |
| `RUST_LOG` | `[observability].log_level` | `"debug"` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `[observability].jaeger_endpoint` | `"http://localhost:14268/api/traces"` |

**Full Mapping Table:**

```bash
# Database
DATABASE_URL → [database].url
DB_MAX_CONNECTIONS → [database].max_connections
DB_MIN_CONNECTIONS → [database].min_connections

# MCP Server
MCP_SERVER_URL → [mcp].server_url
MCP_TIMEOUT_SECONDS → [mcp].timeout_seconds
MCP_MAX_RETRIES → [mcp].max_retries

# LLM
DEFAULT_LLM_PROVIDER → [llm].default_provider
DEFAULT_MODEL → [llm].default_model
RATE_LIMIT_TIER → [llm].rate_limit_tier

# Observability
RUST_LOG → [observability].log_level
OTEL_ENDPOINT → [observability].jaeger_endpoint
TRACING_ENABLED → [observability].tracing_enabled

# Server
HOST → [server].host
PORT → [server].port
```

### Step 4: Update Your Configuration

Edit `my-chat.toml`:

```toml
[environment]
mode = "local"  # or "staging", "production"

[database]
url = "postgresql://botticelli:botticelli@localhost:5432/botticelli"
max_connections = 5
min_connections = 1
connection_timeout_seconds = 30

[mcp]
server_url = "http://localhost:3001"
timeout_seconds = 60
max_retries = 3
retry_delay_ms = 1000

[llm]
default_provider = "gemini"
default_model = "gemini-1.5-flash"
rate_limit_tier = "free"

[observability]
tracing_enabled = true
log_level = "debug"
jaeger_endpoint = "http://localhost:14268/api/traces"

[server]
host = "127.0.0.1"
port = 8080
enable_colors = true

[features]
enable_bot_management = true
enable_social_scheduling = true
enable_narrative_validation = true
```

### Step 5: Keep Secrets in .env

**IMPORTANT:** API keys and secrets should stay in `.env` files, not in TOML!

```bash
# .env - Keep these secret!
ANTHROPIC_API_KEY=sk-ant-...
GEMINI_API_KEY=AIza...
OPENAI_API_KEY=sk-...
GROQ_API_KEY=gsk_...
```

**DO NOT put secrets in TOML files that might be committed to git!**

### Step 6: Test Your Configuration

```bash
# Test with new config
botticelli-chat --config my-chat.toml

# Verify database connection
botticelli-chat --config my-chat.toml --validate-db

# Verify MCP connection
curl http://localhost:3001/health

# Check logs for config loading
RUST_LOG=debug botticelli-chat --config my-chat.toml 2>&1 | grep "Loaded config"
```

### Step 7: Update Scripts and Commands

**Before:**
```bash
# Old way
export DATABASE_URL="postgresql://..."
export MCP_SERVER_URL="http://..."
botticelli-chat
```

**After:**
```bash
# New way
botticelli-chat --config chat.local-dev.toml

# Or set default
export BOTTICELLI_CONFIG=chat.local-dev.toml
botticelli-chat
```

**Update docker-compose.yml:**

```yaml
# Before
services:
  chat:
    environment:
      - DATABASE_URL=postgresql://...
      - MCP_SERVER_URL=http://...

# After
services:
  chat:
    volumes:
      - ./chat.container.toml:/app/chat.toml:ro
    command: ["--config", "/app/chat.toml"]
```

### Step 8: Clean Up Old Environment Variables

```bash
# Remove from .env (keep API keys!)
sed -i '/DATABASE_URL/d' .env
sed -i '/MCP_SERVER_URL/d' .env
sed -i '/RUST_LOG/d' .env

# Or just recreate .env with only secrets
cat > .env << 'EOF'
# API Keys - DO NOT COMMIT!
ANTHROPIC_API_KEY=your-key-here
GEMINI_API_KEY=your-key-here
OPENAI_API_KEY=your-key-here
GROQ_API_KEY=your-key-here
EOF
```

---

## Configuration Precedence

The new system has clear precedence:

1. **CLI flags** (highest priority)
   ```bash
   botticelli-chat --database-url "postgresql://override"
   ```

2. **Environment variables**
   ```bash
   export DATABASE_URL="postgresql://env"
   ```

3. **Config file (TOML)**
   ```toml
   [database]
   url = "postgresql://config"
   ```

4. **Default values** (lowest priority)
   ```rust
   // Built-in defaults
   ```

**Example:**
```bash
# Config file says: localhost
# .env says: postgres
# CLI says: prod-db

botticelli-chat --database-url "postgresql://prod-db"
# Uses: prod-db (CLI wins!)
```

---

## Environment-Specific Configurations

### Local Development

**File:** `chat.local-dev.toml`

```toml
[environment]
mode = "local"

[database]
url = "postgresql://botticelli:botticelli@localhost:5432/botticelli"

[mcp]
server_url = "http://localhost:3001"

[observability]
log_level = "debug"  # Verbose for debugging
```

**Usage:**
```bash
botticelli-chat --config chat.local-dev.toml
```

### Container Deployment

**File:** `chat.container.toml`

```toml
[environment]
mode = "production"

[database]
url = "postgresql://botticelli:botticelli@postgres:5432/botticelli"

[mcp]
server_url = "http://mcp-server:3001"

[observability]
log_level = "info"  # Less verbose
```

**Usage:**
```bash
docker-compose up -d
# Container uses chat.container.toml automatically
```

### Staging

**File:** `chat.staging.toml`

```toml
[environment]
mode = "staging"

[database]
url = "postgresql://botticelli:botticelli@staging-db:5432/botticelli_staging"

[observability]
log_level = "debug"  # More verbose for debugging staging issues
```

---

## Validation

### Before Migration

```bash
# Check current setup works
export DATABASE_URL="postgresql://..."
botticelli-chat
# Should work
```

### After Migration

```bash
# Check new config works
botticelli-chat --config chat.local-dev.toml
# Should work the same way

# Validate config syntax
toml-cli check chat.local-dev.toml

# Test database connection
botticelli-chat --config chat.local-dev.toml --validate-db

# Test MCP connection
curl http://localhost:3001/health
```

---

## Common Issues

### Issue: Config Not Found

**Error:**
```
Config file not found: chat.toml
```

**Solution:**
```bash
# Specify config explicitly
botticelli-chat --config chat.local-dev.toml

# Or set environment variable
export BOTTICELLI_CONFIG=chat.local-dev.toml
```

### Issue: Wrong Host Names

**Problem:** Using `localhost` in container mode

**Solution:**
```toml
# Local dev (host machine)
[database]
url = "postgresql://localhost:5432/..."

# Container (docker network)
[database]
url = "postgresql://postgres:5432/..."  # Use service name!
```

### Issue: Secrets in Config File

**Problem:** Accidentally committed API keys to git

**Solution:**
```bash
# Remove from TOML
# Put secrets in .env only

# Add to .gitignore
echo ".env" >> .gitignore
echo "chat.local-dev.toml" >> .gitignore  # If you added secrets there

# Remove from git history
git filter-branch --force --index-filter \
  'git rm --cached --ignore-unmatch chat.local-dev.toml' \
  --prune-empty --tag-name-filter cat -- --all
```

---

## Rollback Plan

If you need to roll back to environment variables:

```bash
# 1. Keep your old .env file
cp .env .env.backup

# 2. Convert TOML back to .env if needed
# Extract values manually from TOML

# 3. Use old method
unset BOTTICELLI_CONFIG
export DATABASE_URL="postgresql://..."
export MCP_SERVER_URL="http://..."
botticelli-chat
```

---

## Migration Checklist

- [ ] Back up current `.env` file
- [ ] Choose config template (local/container/staging)
- [ ] Create new TOML config file
- [ ] Map environment variables to TOML
- [ ] Keep API keys in `.env` (not TOML!)
- [ ] Test configuration locally
- [ ] Update docker-compose.yml
- [ ] Update deployment scripts
- [ ] Update documentation
- [ ] Clean up old environment variables
- [ ] Test in staging
- [ ] Deploy to production
- [ ] Remove old .env variables (keep secrets!)

---

## Quick Reference

### Files to Create

| Environment | Config File | Use Case |
|------------|------------|----------|
| Local Dev | `chat.local-dev.toml` | Local machine development |
| Container | `chat.container.toml` | Docker deployment |
| Staging | `chat.staging.toml` | Pre-production testing |

### Files to Keep

| File | Purpose | Should Commit? |
|------|---------|----------------|
| `.env` | API keys and secrets | ❌ Never commit |
| `chat.*.toml` | Configuration templates | ✅ Yes (without secrets) |
| `.env.example` | Example secrets file | ✅ Yes |

### Command Changes

| Old Command | New Command |
|------------|-------------|
| `botticelli-chat` | `botticelli-chat --config chat.local-dev.toml` |
| `export DATABASE_URL=...` | Edit `chat.local-dev.toml` |
| Check `.env` for settings | Check TOML file |

---

## Example: Full Migration

**Before:**
```bash
# .env
DATABASE_URL=postgresql://botticelli:botticelli@localhost:5432/botticelli
MCP_SERVER_URL=http://localhost:3001
RUST_LOG=debug
ANTHROPIC_API_KEY=sk-ant-secret
```

**After:**

```bash
# .env - ONLY SECRETS
ANTHROPIC_API_KEY=sk-ant-secret
GEMINI_API_KEY=your-key
```

```toml
# chat.local-dev.toml - CONFIGURATION
[environment]
mode = "local"

[database]
url = "postgresql://botticelli:botticelli@localhost:5432/botticelli"

[mcp]
server_url = "http://localhost:3001"

[observability]
log_level = "debug"
```

```bash
# Run with new config
botticelli-chat --config chat.local-dev.toml
```

**Success!** ✅

---

## Need Help?

See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for common issues and solutions.
