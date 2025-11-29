# Botticelli Actor

Platform-agnostic actor system for social media automation.

## Overview

This crate provides the core abstractions for building automated social media actors that can work across multiple platforms (Discord, Twitter, Bluesky, etc.). Actors orchestrate **skills** and **knowledge** to automate content posting, scheduling, filtering, and more.

## Architecture

- **Actors**: Configured bots that orchestrate skills and knowledge
- **Platforms**: Trait-based abstraction for social media APIs
- **Skills**: Reusable capabilities (scheduling, filtering, formatting, etc.)
- **Knowledge**: Database tables produced by narratives
- **Server**: Long-running task execution with scheduling and persistence

## Features

- `discord` - Discord platform support (enabled by default)

## Usage

### As a Library

```rust
use botticelli_actor::{Actor, ActorConfig, SkillRegistry, DiscordPlatform};
use std::sync::Arc;

// Load actor configuration
let config = ActorConfig::from_file("actor.toml")?;

// Create platform
let platform = DiscordPlatform::new("token", "channel_id")?;

// Build actor
let actor = Actor::builder()
    .config(config)
    .skills(SkillRegistry::new())
    .platform(Arc::new(platform))
    .build()?;

// Execute (requires database connection)
// let mut conn = establish_connection()?;
// actor.execute(&mut conn).await?;
```

### As a Server Binary

The `actor-server` binary provides a long-running server for scheduled task execution.

#### Installation

```bash
cargo install botticelli_actor --features discord
```

#### Configuration

Create an `actor_server.toml` configuration file:

```toml
[server]
check_interval_seconds = 60
max_consecutive_failures = 5

[[actors]]
name = "daily_poster"
config_file = "actors/daily_poster.toml"
channel_id = "1234567890"
enabled = true

[actors.schedule]
type = "Interval"
seconds = 86400  # 24 hours
```

#### Running

```bash
# Set environment variables
export DATABASE_URL="postgresql://user:pass@localhost/botticelli"
export DISCORD_TOKEN="your-discord-bot-token"

# Start the server
actor-server --config actor_server.toml

# Or specify credentials via CLI
actor-server \
  --config actor_server.toml \
  --database-url "postgresql://user:pass@localhost/botticelli" \
  --discord-token "your-discord-bot-token"

# Dry run mode (validate configuration)
actor-server --config actor_server.toml --dry-run
```

#### Schedule Types

**Interval**: Fixed periodic execution

```toml
[actors.schedule]
type = "Interval"
seconds = 3600  # Every hour
```

**Cron**: Cron expression (7-field format)

```toml
[actors.schedule]
type = "Cron"
expression = "0 0 9 * * * *"  # 9 AM daily
```

**Once**: One-time execution at specific time

```toml
[actors.schedule]
type = "Once"
at = "2025-12-31T23:59:59Z"
```

**Immediate**: Execute once on startup

```toml
[actors.schedule]
type = "Immediate"
```

## Actor Configuration

Individual actor configuration files define the actor's behavior:

```toml
[actor]
name = "Daily Content Poster"
description = "Posts daily curated content"

[knowledge]
table_name = "narrative_content"
query = "SELECT * FROM narrative_content WHERE posted = false LIMIT 1"

[[skills]]
name = "content_selection"
config = { strategy = "latest", limit = 1 }

[[skills]]
name = "duplicate_check"
config = { lookback_days = 7 }

[[skills]]
name = "content_formatter"
config = { max_length = 2000, add_hashtags = true }

[execution]
max_retries = 3
timeout_seconds = 60

[cache]
strategy = "memory"
ttl_seconds = 3600
```

## Built-in Skills

- **ContentSelectionSkill**: Select content from knowledge tables
- **DuplicateCheckSkill**: Prevent duplicate posts
- **ContentFormatterSkill**: Format content for platform limits
- **RateLimitingSkill**: Enforce posting intervals
- **ContentSchedulingSkill**: Advanced scheduling logic

## Platform Traits

The `Platform` trait enables cross-platform support:

```rust
#[async_trait]
pub trait Platform: Send + Sync {
    async fn post(&self, content: &ContentPost) -> ActorResult<PlatformMessage>;
    async fn validate_post(&self, content: &ContentPost) -> ActorResult<()>;
    fn capabilities(&self) -> Vec<PlatformCapability>;
    fn metadata(&self) -> PlatformMetadata;
}
```

## State Persistence

The actor server uses PostgreSQL for state persistence:

- **`actor_server_state`**: Task state and circuit breaker tracking
- **`actor_server_executions`**: Execution history and audit trail

State survives server restarts, ensuring reliable operation.

## Examples

See the `examples/` directory:

- `actor_server.toml` - Server configuration
- `crates/botticelli_actor/actors/daily_poster.toml` - Daily content posting
- `crates/botticelli_actor/actors/trending.toml` - Hourly trending topics
- `crates/botticelli_actor/actors/welcome.toml` - Startup welcome message

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
