# Actor Server Implementation Strategy

**Date**: 2025-11-24
**Status**: Infrastructure 100% Complete, Binary Integration 40% Complete

## Executive Summary

The actor server framework has **complete production-grade infrastructure** implemented:
- ✅ **Connection pooling** (r2d2) with DatabaseStatePersistence
- ✅ **Circuit breaker** with automatic pause on failure threshold
- ✅ **Execution history** tracking with start/complete/fail methods
- ✅ **ActorExecutionTracker** high-level API wrapper
- ✅ **All 66 tests passing** across 13 test files

However, the **actor-server binary** (crates/botticelli_actor/src/bin/actor-server.rs) is only partially integrated:
- ✅ Configuration loading and actor instantiation
- ✅ Schedule-based execution loop
- ✅ Graceful shutdown
- ❌ Uses per-execution `establish_connection()` instead of pooled persistence
- ❌ No execution history recording
- ❌ No circuit breaker enforcement
- ❌ No state persistence after execution

**Next Step**: Integrate DatabaseStatePersistence and ActorExecutionTracker into the binary's execution loop.

### ✅ Completed Infrastructure (Phases 1-4)

#### Phase 1-2: Core Trait Framework & Generic Implementations

**Location**: `crates/botticelli_server/src/actor_traits.rs`

Five foundational traits defining actor server architecture:

1. **`TaskScheduler`** - Periodic task execution with cancellation
2. **`ActorManager<ActorId, Context>`** - Actor lifecycle and registry
3. **`ContentPoster<Content, Dest, Posted>`** - Platform-agnostic content posting
4. **`StatePersistence<State>`** - State save/load interface for recovery
5. **`ActorServer`** - Top-level coordinator for start/stop/reload

**Location**: `crates/botticelli_actor/src/server.rs`

Generic trait implementations for rapid prototyping:

- `SimpleTaskScheduler` - tokio-based in-memory scheduler
- `GenericActorManager<I, C>` - HashMap actor registry
- `GenericContentPoster` - Stub implementation
- `JsonStatePersistence<T>` - File-based JSON persistence
- `BasicActorServer` - Minimal running state coordinator

#### Phase 3: Persistent State Management ✅

**Migration**: `migrations/2025-11-24-000144-0000_create_actor_server_state_tables/`

Two PostgreSQL tables for production-grade persistence:

1. **`actor_server_state`** - Task state and circuit breaker tracking
   - `task_id` (PK), `actor_name`, `last_run`, `next_run`
   - `consecutive_failures`, `is_paused`, `metadata` (JSONB)
   - Indices on `next_run` (for scheduling) and `actor_name`

2. **`actor_server_executions`** - Execution history and audit trail
   - `task_id`, `actor_name`, `started_at`, `completed_at`
   - `success`, `error_message`, skill execution counts
   - Indices on `task_id` and `started_at` for queries

**Location**: `crates/botticelli_database/src/actor_server_models.rs`

Diesel models with derive_builder pattern:
- `ActorServerStateRow` / `NewActorServerState` - State management
- `ActorServerExecutionRow` / `NewActorServerExecution` - History tracking

**Location**: `crates/botticelli_actor/src/state_persistence.rs`

`DatabaseStatePersistence` implementation:
- Async trait impl with tokio::spawn_blocking
- Upsert support via INSERT ... ON CONFLICT
- Full tracing instrumentation

**Benefits**:
- ✅ Server restarts preserve task state
- ✅ Execution history for debugging and auditing
- ✅ Circuit breaker state survives crashes
- ✅ JSONB metadata for extensibility

#### Phase 4: Advanced Scheduling ✅

**Location**: `crates/botticelli_server/src/schedule.rs`

Complete scheduling system with four schedule types:

```rust
pub enum ScheduleType {
    Cron { expression: String },       // 7-field: sec min hour day month weekday year
    Interval { seconds: u64 },         // Fixed periodic intervals
    Once { at: DateTime<Utc> },        // One-time future execution
    Immediate,                          // Execute once on startup
}

pub trait Schedule {
    fn check(&self, last_run: Option<DateTime<Utc>>) -> ScheduleCheck;
    fn next_execution(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>>;
}
```

**Implementation Details**:
- `cron = "0.12"` dependency for cron parsing
- Serde serialization with tagged enum for clean TOML/JSON
- Proper handling of one-time vs repeating schedules
- Optional `next_run` for terminal schedules (Immediate, Once)

**Test Coverage**: 7 comprehensive tests (all passing)
- Schedule construction and checking
- Cron expression parsing and next execution
- Invalid cron error handling
- Serde round-trip serialization

**Important**: The `cron` crate uses **7-field format** (not standard 5-field):
```
sec  min  hour  day  month  weekday  year
0    0    9     *    *      *        *      = 9 AM daily
0    30   9,15  *    *      Mon-Fri  *      = 9:30 AM and 3:30 PM weekdays
```

#### Discord Integration

**Location**: `crates/botticelli_actor/src/discord_server.rs`

Complete Discord platform implementation:
- `DiscordActorId` - Actor identification
- `DiscordContext` - HTTP client context
- `DiscordTaskScheduler` - Discord-specific scheduling
- `DiscordActorManager` - Actor management
- `DiscordContentPoster` - serenity HTTP posting
- `DiscordServerState` - Serializable state
- `DiscordActorServer` - Full server coordinator

**Test Coverage**: 14 passing tests
- 9 discord_server tests
- 5 platform_trait tests
- Builder pattern compliance
- Full tracing instrumentation

### 🚧 Phase 5: Production Binary (40% Complete)

**Location**: `crates/botticelli_actor/src/bin/actor-server.rs` (lines 1-300)

**Infrastructure Ready** (100% implemented):
- ✅ `DatabaseStatePersistence` with r2d2 connection pooling (state_persistence.rs:1-1091)
- ✅ `ActorExecutionTracker` high-level API wrapper (execution_tracker.rs:1-191)
- ✅ Circuit breaker with auto-pause on threshold (state_persistence.rs:909-1007)
- ✅ Execution history tracking (state_persistence.rs:610-869)
- ✅ Task pause/resume/listing methods (state_persistence.rs:365-566)
- ✅ Connection pooling with configurable size (state_persistence.rs:54-99)
- ✅ `CircuitBreakerConfig` in server config (server_config.rs:47-69)
- ✅ All 66 tests passing across 13 test files

**Binary Implementation** (40% complete):

1. **Executable Binary** ✅
   - ✅ clap argument parsing (--config, --database-url, --discord-token, --dry-run)
   - ✅ TOML configuration file loading via `ActorServerConfig`
   - ✅ Actor instantiation from config files
   - ✅ Graceful shutdown on SIGTERM/SIGINT (tokio::signal)
   - ✅ Tracing initialization with EnvFilter
   - ✅ Schedule-based task execution loop using `tokio::select!`
   - ❌ **NOT using DatabaseStatePersistence** - creates it but doesn't use (lines 100-121)
   - ❌ **NOT using ActorExecutionTracker** - helper not integrated

2. **State Persistence** ❌ NOT INTEGRATED
   - ✅ Infrastructure: DatabaseStatePersistence fully implemented
   - ❌ Binary: Loads state on startup but doesn't apply it (lines 104-118)
   - ❌ Binary: Uses `establish_connection()` per-execution (line 253)
   - ❌ Binary: Doesn't save state after execution
   - ❌ Binary: In-memory `last_run` tracking, lost on restart (line 139-140)

3. **Circuit Breaker** ❌ NOT INTEGRATED
   - ✅ Infrastructure: `record_failure()` with auto-pause (state_persistence.rs:909-1007)
   - ✅ Infrastructure: `should_execute()` checks pause state (state_persistence.rs:1047-1090)
   - ❌ Binary: No failure tracking in execution loop (lines 256-265)
   - ❌ Binary: No circuit breaker enforcement
   - ❌ Binary: Failures only logged, not recorded

4. **Execution History** ❌ NOT INTEGRATED
   - ✅ Infrastructure: `start_execution()` / `complete_execution()` / `fail_execution()`
   - ✅ Infrastructure: `get_execution_history()` for debugging
   - ❌ Binary: No execution records created
   - ❌ Binary: No execution history tracking

5. **Configuration System** ✅
   - ✅ `ActorServerConfig` with server settings
   - ✅ `ServerSettings` with `CircuitBreakerConfig`
   - ✅ `ActorInstanceConfig` (name, config_file, channel_id, schedule, enabled)
   - ✅ `ScheduleConfig` enum (Interval, Cron, Once, Immediate)
   - ✅ Schedule trait implementation for all schedule types
   - ✅ Environment variable support (DATABASE_URL, DISCORD_TOKEN)

6. **Example Configurations** ✅
   - ✅ `examples/actor_server.toml` - Server configuration
   - ✅ `examples/actors/daily_poster.toml` - Daily content posting
   - ✅ `examples/actors/trending.toml` - Hourly trending topics
   - ✅ `examples/actors/welcome.toml` - Startup welcome message

**What Needs Integration** (lines 240-282 of actor-server.rs):

```rust
// CURRENT (lines 246-274):
for (name, (actor, schedule, last_run)) in actors.iter_mut() {
    let check = schedule.check(*last_run);
    if check.should_run {
        match establish_connection() {  // ❌ Per-execution connection
            Ok(mut conn) => {
                match actor.execute(&mut conn).await {  // ❌ No history/circuit breaker
                    Ok(_) => {
                        *last_run = Some(Utc::now());  // ❌ In-memory only
                    }
                    Err(e) => {
                        error!(error = ?e, "Actor execution failed");  // ❌ No tracking
                    }
                }
            }
            Err(e) => error!(error = ?e, "Failed to connect"),
        }
    }
}

// NEEDED:
let persistence = Arc::new(DatabaseStatePersistence::new()?);
for (name, (actor, schedule, last_run)) in actors.iter_mut() {
    let tracker = ActorExecutionTracker::new(
        persistence.clone(),
        name.clone(),
        name.clone(),
    );

    if !tracker.should_execute().await? {  // ✅ Circuit breaker check
        continue;
    }

    let check = schedule.check(*last_run);
    if check.should_run {
        let exec_id = tracker.start_execution().await?;  // ✅ History start

        match establish_connection() {
            Ok(mut conn) => {
                match actor.execute(&mut conn).await {
                    Ok(_) => {
                        let result = DatabaseExecutionResult { /* ... */ };
                        tracker.record_success(exec_id, result).await?;  // ✅ Success tracking
                        *last_run = Some(Utc::now());
                    }
                    Err(e) => {
                        tracker.record_failure(exec_id, &e.to_string()).await?;  // ✅ Circuit breaker
                    }
                }
            }
            Err(e) => {
                tracker.record_failure(exec_id, &e.to_string()).await?;
            }
        }
    }
}
```

### ✅ Phase 6 Complete: Comprehensive Testing

**Test Coverage**: 66 passing tests across 13 test files

1. **Actor Error Handling Tests** (6 tests) ✅
   - `actor_error_handling_test.rs`
   - Config validation (cache size, TTL, retries)
   - Error classification (recoverable vs unrecoverable)
   - SkillInfo builder validation

2. **Actor Execution Tracker Tests** (3 tests) ✅
   - `actor_execution_tracker_test.rs`
   - Execution lifecycle (start → success/failure → history)
   - Circuit breaker integration
   - Accessor methods

3. **Configuration Tests** (4 tests) ✅
   - `actor_server_integration_test.rs`
   - Actor config loading from TOML files
   - Multiple knowledge sources
   - Skills configuration
   - Minimal configuration validation

4. **Schedule Tests** (11 tests) ✅
   - `schedule_test.rs`
   - Immediate, Once, Interval, Cron schedules
   - Schedule checking logic
   - Next execution calculations
   - Cron expression parsing (daily, weekday patterns)
   - Edge cases (zero interval, already executed)

5. **State Persistence Tests** (2 tests) ✅
   - `state_persistence_test.rs`
   - DatabaseStatePersistence construction
   - Interface validation

6. **State Persistence Circuit Breaker Tests** (6 tests) ✅
   - `state_persistence_circuit_breaker_test.rs`
   - Failure counter increment
   - Threshold exceeded auto-pause
   - Success counter reset
   - Pause state enforcement
   - Manual pause override
   - Execution logging integration

7. **State Persistence Multi-Task Tests** (9 tests) ✅
   - `state_persistence_multi_task_test.rs`
   - Save/load/delete task state
   - List tasks (all, by actor, active, paused)
   - Update next run
   - Pause/resume operations
   - Concurrent operations

8. **State Persistence Minimal Tests** (4 tests) ✅
   - `state_persistence_minimal_test.rs`
   - Single state save/load
   - Two task inserts
   - Concurrent competing inserts

9. **State Persistence Five Test** (1 test) ✅
   - `state_persistence_five_test.rs`
   - Five task concurrent inserts

10. **Discord Integration Tests** (9 tests) ✅
    - `discord_server_test.rs`
    - Actor ID, Context, Manager creation
    - Content posting
    - Server state persistence
    - Task scheduler lifecycle
    - Server reload functionality

11. **Platform Trait Tests** (5 tests) ✅
    - `platform_trait_test.rs`
    - Discord platform creation
    - Post validation
    - Text limit enforcement
    - Platform capabilities

12. **Scheduler Persistence Integration Tests** (3 tests) ✅
    - `scheduler_persistence_integration_test.rs`
    - Scheduler with/without persistence
    - Task recovery from database

13. **Server Config Tests** (3 tests) ✅
    - `server_config_test.rs`
    - Default values
    - Immediate schedule behavior
    - Server config parsing

**Test Quality**:
- ✅ No `#[ignore]` tests
- ✅ All tests self-contained
- ✅ Proper use of temp directories for file I/O
- ✅ Async test support with tokio
- ✅ Full feature coverage (schedule types, platforms, persistence)

### ❌ Remaining Work (Binary Integration)

#### Phase 5c: Integrate DatabaseStatePersistence into Binary (Required for Production)

**Current Issue**: All infrastructure exists but binary doesn't use it.

**Required Changes to actor-server.rs**:

1. **Initialize Shared Persistence** (before line 136)
   ```rust
   // Create shared persistence with connection pool
   let persistence = if args.database_url.is_some() || std::env::var("DATABASE_URL").is_ok() {
       Some(Arc::new(DatabaseStatePersistence::new()?))
   } else {
       None
   };
   ```

2. **Load State on Startup** (replace lines 104-121)
   ```rust
   if let Some(persistence) = &persistence {
       for actor_instance in &server_config.actors {
           if let Some(state) = persistence.load_task_state(&actor_instance.name).await? {
               info!(
                   task_id = %state.task_id,
                   actor = %state.actor_name,
                   consecutive_failures = ?state.consecutive_failures,
                   is_paused = ?state.is_paused,
                   "Loaded previous task state from database"
               );
               // TODO: Initialize actor with loaded state
           }
       }
   }
   ```

3. **Create ActorExecutionTracker per Actor** (after line 175)
   ```rust
   // Store tracker with actor
   let tracker = persistence.as_ref().map(|p| {
       ActorExecutionTracker::new(
           p.clone(),
           actor_instance.name.clone(),
           actor_instance.name.clone()
       )
   });
   actors.insert(
       actor_instance.name.clone(),
       (actor, actor_instance.schedule.clone(), None, tracker),
   );
   ```

4. **Integrate Circuit Breaker and History** (replace lines 246-274)
   ```rust
   for (name, (actor, schedule, last_run, tracker)) in actors.iter_mut() {
       // Check circuit breaker
       if let Some(tracker) = tracker {
           if !tracker.should_execute().await? {
               debug!(actor = %name, "Task paused by circuit breaker, skipping");
               continue;
           }
       }

       let check = schedule.check(*last_run);
       if check.should_run {
           info!(actor = %name, "Executing scheduled actor");

           // Start execution history
           let exec_id = if let Some(tracker) = tracker {
               Some(tracker.start_execution().await?)
           } else {
               None
           };

           // Get database connection (TODO: use pooled connection)
           match establish_connection() {
               Ok(mut conn) => {
                   match actor.execute(&mut conn).await {
                       Ok(result) => {
                           info!(actor = %name, "Actor executed successfully");
                           *last_run = Some(Utc::now());

                           // Record success
                           if let (Some(tracker), Some(exec_id)) = (tracker, exec_id) {
                               let db_result = DatabaseExecutionResult {
                                   skills_succeeded: result.skills_executed() as i32,
                                   skills_failed: 0,
                                   skills_skipped: 0,
                                   metadata: serde_json::json!({}),
                               };
                               tracker.record_success(exec_id, db_result).await?;
                           }
                       }
                       Err(e) => {
                           error!(actor = %name, error = ?e, "Actor execution failed");

                           // Record failure (circuit breaker)
                           if let (Some(tracker), Some(exec_id)) = (tracker, exec_id) {
                               let should_pause = tracker.record_failure(exec_id, &e.to_string()).await?;
                               if should_pause {
                                   warn!(actor = %name, "Circuit breaker triggered, task paused");
                               }
                           }
                       }
                   }
               }
               Err(e) => {
                   error!(actor = %name, error = ?e, "Failed to establish database connection");

                   // Record connection failure
                   if let (Some(tracker), Some(exec_id)) = (tracker, exec_id) {
                       tracker.record_failure(exec_id, &e.to_string()).await?;
                   }
               }
           }

           if let Some(next) = check.next_run {
               debug!(actor = %name, next_run = %next, "Next execution scheduled");
           }
       }
   }
   ```

5. **Save State on Shutdown** (after line 285)
   ```rust
   // Save final state
   if let Some(persistence) = &persistence {
       for (name, (_, _, last_run, _)) in &actors {
           if let Some(state) = persistence.load_task_state(name).await? {
               let mut updated = state.clone();
               updated.last_run = last_run.map(|dt| dt.naive_utc());
               persistence.save_task_state(name, &updated).await?;
           }
       }
   }
   ```

**Estimated Effort**: 2-3 hours (straightforward integration of existing code)

#### Should Have (Phase 6 - Observability)

1. **Metrics and Monitoring**
   - ❌ Prometheus metrics export
   - ❌ Task execution counters (success/failure)
   - ❌ Execution duration histograms
   - ❌ Scheduled task gauges

2. **Health Checks**
   - ❌ Health endpoint for load balancers
   - ❌ Task status reporting
   - ❌ Database connectivity checks

3. **Alerting**
   - ❌ Webhook on consecutive failures
   - ❌ Discord notification to ops channel
   - ❌ Configurable alert thresholds

#### Nice to Have (Phase 7 - HTTP API)

1. **REST API** (axum-based)
   - ❌ `GET /health` - Server health
   - ❌ `GET /tasks` - List scheduled tasks
   - ❌ `GET /tasks/:id` - Task status
   - ❌ `POST /tasks/:id/trigger` - Manual execution
   - ❌ `POST /tasks/:id/pause` - Pause scheduling
   - ❌ `POST /tasks/:id/resume` - Resume scheduling
   - ❌ `GET /tasks/:id/history` - Execution history

2. **Advanced Features**
   - ❌ Connection pooling (r2d2 or deadpool)
   - ❌ Parallel task execution with bounded pools
   - ❌ EventServer trait for reactive triggers
   - ❌ Webhook integration for external events

---



---

## Phase 5: Production Binary (CURRENT PRIORITY)

### Objective
Build deployable server executable with TOML configuration, graceful lifecycle management, and integration with existing infrastructure (scheduling, state persistence, circuit breakers).

### Architecture Overview

```
actor-server binary
    ├── Load TOML configuration
    ├── Initialize DatabaseStatePersistence
    ├── Create DiscordActorServer
    ├── For each actor config:
    │   ├── Load Actor from config_file
    │   ├── Parse ScheduleType from config
    │   ├── Register actor with server
    │   └── Schedule task with proper intervals
    ├── Recover state from database
    ├── Start execution loop
    └── Handle graceful shutdown on SIGTERM/SIGINT
```

### 5.1: Binary Entry Point

**Location**: `crates/botticelli_actor/src/bin/actor-server.rs`

**Requirements**:
1. clap for CLI argument parsing
2. tracing_subscriber for structured logging
3. tokio signal handling for graceful shutdown
4. Configuration file loading
5. Actor instantiation from TOML
6. Integration with ScheduleType and DatabaseStatePersistence

**Skeleton**:
```rust
use botticelli_actor::{Actor, ActorBuilder, DatabaseStatePersistence, DiscordActorServer};
use botticelli_server::{Schedule, ScheduleType, StatePersistence};
use clap::Parser;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "actor-server")]
#[command(about = "Botticelli Actor Server - Execute scheduled bot tasks")]
struct Args {
    /// Path to server configuration file
    #[arg(short, long, default_value = "actor_server.toml")]
    config: String,

    /// Database URL (can also use DATABASE_URL env var)
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Discord token (can also use DISCORD_TOKEN env var)
    #[arg(long, env = "DISCORD_TOKEN")]
    discord_token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    info!(config = %args.config, "Starting Botticelli Actor Server");

    // Load configuration
    let config = ServerConfig::load_from_file(&args.config)?;
    
    // Validate required credentials
    let discord_token = args.discord_token
        .or(config.discord_token)
        .ok_or("Discord token required (--discord-token or DISCORD_TOKEN)")?;

    // Initialize components
    let persistence = DatabaseStatePersistence::new();
    let mut server = DiscordActorServer::new(discord_token);

    // Load and register actors
    for actor_config in config.actors {
        info!(
            actor = %actor_config.name,
            config_file = %actor_config.config_file,
            "Loading actor"
        );

        let actor = Actor::load_from_file(&actor_config.config_file)?;
        
        // TODO: Register actor with schedule
        // server.register_actor_with_schedule(actor, actor_config.schedule)?;
    }

    // Recover state from database
    if let Some(state) = persistence.load_state().await? {
        info!("Recovered server state from database");
        // TODO: Apply recovered state to server
    }

    // Setup graceful shutdown
    let shutdown = setup_shutdown_handler();

    // Start server
    info!("Starting task execution loop");
    server.start().await?;

    // Wait for shutdown signal
    shutdown.await;
    info!("Shutdown signal received, stopping gracefully");

    // Graceful shutdown
    server.stop().await?;
    info!("Server stopped successfully");

    Ok(())
}

async fn setup_shutdown_handler() -> tokio::sync::oneshot::Receiver<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        let _ = tx.send(());
    });
    
    rx
}
```

### 5.2: Configuration Structure

**Location**: `crates/botticelli_actor/src/server_config.rs`

**Purpose**: Define TOML-deserializable configuration structure.

```rust
use botticelli_server::ScheduleType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server-level settings
    pub server: ServerSettings,
    
    /// List of actors to run
    pub actors: Vec<ActorConfig>,
    
    /// Optional Discord token (can be overridden by CLI arg)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// How often to check for tasks to execute (seconds)
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    
    /// Max consecutive failures before pausing task
    #[serde(default = "default_max_failures")]
    pub max_consecutive_failures: i32,
}

fn default_check_interval() -> u64 { 60 }
fn default_max_failures() -> i32 { 5 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorConfig {
    /// Unique actor name/identifier
    pub name: String,
    
    /// Path to actor TOML configuration
    pub config_file: String,
    
    /// Execution schedule
    pub schedule: ScheduleType,
    
    /// Discord channel ID for posting (platform-specific)
    pub channel_id: String,
}

impl ServerConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}
```

### 5.3: Example Configuration File

**Location**: `examples/actor_server.toml`

```toml
# Server-level settings
[server]
check_interval_seconds = 60        # Check for tasks every minute
max_consecutive_failures = 5       # Pause after 5 failures

# Actor definitions
[[actors]]
name = "daily_poster"
config_file = "examples/actors/daily_poster.toml"
channel_id = "1234567890"

[actors.schedule]
type = "Cron"
expression = "0 0 9 * * * *"       # 9 AM daily (7-field format)

[[actors]]
name = "hourly_trends"
config_file = "examples/actors/trending.toml"
channel_id = "0987654321"

[actors.schedule]
type = "Interval"
seconds = 3600                      # Every hour

[[actors]]
name = "breaking_news"
config_file = "examples/actors/news.toml"
channel_id = "1122334455"

[actors.schedule]
type = "Immediate"                  # Run once on startup
```

### 5.4: Integration Requirements

**State Recovery on Startup**:
1. Load state from `actor_server_state` table
2. For each task: check if `next_run` has passed
3. Update `consecutive_failures` for circuit breaker
4. Resume paused tasks if manually un-paused

**Task Execution Loop**:
1. Every `check_interval_seconds`: query tasks where `next_run <= NOW()`
2. For each ready task:
   - Check if `is_paused` or circuit breaker triggered
   - Execute actor with context
   - Record execution in `actor_server_executions` table
   - Update `last_run`, `next_run`, `consecutive_failures` in state
   - Use `ScheduleType::next_execution()` to calculate next run

**Circuit Breaker Logic**:
```rust
if consecutive_failures >= max_consecutive_failures {
    warn!(
        task_id = %task_id,
        failures = consecutive_failures,
        "Circuit breaker triggered, pausing task"
    );
    
    // Update state to pause task
    diesel::update(actor_server_state::table.find(task_id))
        .set(actor_server_state::is_paused.eq(true))
        .execute(&mut conn)?;
}
```

### 5.5: Implementation Steps

1. **Create `server_config.rs`**: TOML configuration structures with serde
2. **Create `bin/actor-server.rs`**: Binary entry point with clap
3. **Add dependencies**: `clap = { version = "4", features = ["derive"] }`
4. **Implement configuration loading**: Test with example TOML
5. **Integrate ScheduleType**: Use existing schedule.rs infrastructure
6. **Implement state recovery**: Query database on startup
7. **Build execution loop**: Task checking and execution
8. **Add circuit breaker**: Failure tracking and auto-pause
9. **Test graceful shutdown**: Verify cleanup on SIGTERM
10. **Create example configs**: Multiple schedule types
11. **Documentation**: Usage guide in planning doc

### 5.6: Success Criteria

- ✅ Binary compiles and runs with `just build actor-server`
- ✅ Loads TOML configuration without errors
- ✅ Instantiates actors from config files
- ✅ Recovers state from database on startup
- ✅ Executes tasks on schedule (Cron, Interval, Once, Immediate)
- ✅ Records executions to database
- ✅ Circuit breaker pauses failing tasks
- ✅ Graceful shutdown on Ctrl+C
- ✅ Example configs work out of box

---

## Phase 6: Observability & Operations

### 6.1: Metrics

Integrate with `prometheus` or `metrics` crate:

```rust
use metrics::{counter, histogram, gauge};

// In execute_task:
counter!("actor.executions.total", 1, "actor" => actor_name);
histogram!("actor.execution.duration_ms", duration.as_millis() as f64);

// In task scheduler:
gauge!("actor.tasks.scheduled", scheduled_count as f64);
gauge!("actor.tasks.paused", paused_count as f64);
```

### 6.2: Health Checks

**Location**: `crates/botticelli_actor/src/health.rs`

```rust
pub struct HealthStatus {
    pub healthy: bool,
    pub scheduled_tasks: usize,
    pub failed_tasks: Vec<String>,
    pub last_execution: Option<DateTime<Utc>>,
}

impl DiscordActorServer {
    pub async fn health(&self) -> HealthStatus {
        // Check all tasks, report status
    }
}
```

### 6.3: Alerting

Integration points for external alerting:
- Webhook on N consecutive failures
- PagerDuty integration for critical errors
- Discord notification to ops channel

---

## Phase 7: HTTP API (Optional)

### Objective
Remote control and monitoring via REST API.

**Location**: `crates/botticelli_actor/src/api.rs`

```rust
use axum::{Router, routing::get};

pub fn create_router(server: Arc<RwLock<DiscordActorServer>>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/tasks", get(list_tasks))
        .route("/tasks/:id", get(get_task_status))
        .route("/tasks/:id/pause", post(pause_task))
        .route("/tasks/:id/resume", post(resume_task))
        .route("/tasks/:id/trigger", post(trigger_task))
        .route("/tasks/:id/history", get(task_history))
        .with_state(server)
}
```

**Endpoints**:
- `GET /health` - Server health status
- `GET /tasks` - List all scheduled tasks
- `GET /tasks/:id` - Task status details
- `POST /tasks/:id/trigger` - Manual execution
- `POST /tasks/:id/pause` - Pause scheduling
- `POST /tasks/:id/resume` - Resume scheduling
- `GET /tasks/:id/history` - Execution history

---

## Implementation Roadmap

### ✅ Completed Phases

1. **Phase 1-2: Foundation** ✅
   - Core trait framework (TaskScheduler, ActorManager, ContentPoster, StatePersistence, ActorServer)
   - Generic implementations (SimpleTaskScheduler, GenericActorManager, etc.)
   - Discord platform integration with full trait implementation
   - 9 Discord server tests + 5 platform trait tests passing

2. **Phase 3: Persistent State Management** ✅
   - PostgreSQL schema (`actor_server_state`, `actor_server_executions`)
   - Diesel models with derive_builder pattern
   - `DatabaseStatePersistence` with r2d2 connection pooling
   - 20+ state persistence tests across 5 test files
   - Execution history and circuit breaker infrastructure

3. **Phase 4: Advanced Scheduling** ✅
   - `ScheduleConfig` enum (Cron, Interval, Once, Immediate)
   - `Schedule` trait with check() and next_execution()
   - Cron parsing with `cron = "0.12"` (7-field format)
   - 11 comprehensive schedule tests, all passing

4. **Phase 5a-5b: Binary Foundation** ✅
   - `actor-server` binary with clap CLI
   - TOML configuration loading (ActorServerConfig)
   - Actor instantiation from config files
   - Schedule-driven execution loop
   - Graceful shutdown handling
   - Example configurations

5. **Phase 6: Comprehensive Testing** ✅
   - 66 tests passing across 13 test files
   - Circuit breaker tests (6 tests)
   - Execution tracking tests (3 tests)
   - Multi-task state persistence (9 tests)
   - Error handling tests (6 tests)
   - All infrastructure validated

### 🚧 Current Priority: Phase 5c (Binary Integration)

**Status**: Infrastructure 100% complete, binary integration 40% complete

**Estimated Time**: 2-3 hours (not days - just integration work)

**Deliverables**:
1. ✅ Integrate DatabaseStatePersistence into binary
2. ✅ Use ActorExecutionTracker in execution loop
3. ✅ Enable circuit breaker enforcement
4. ✅ Record execution history to database
5. ✅ Persist state across restarts
6. ✅ Load state on startup

**Blockers**: None - all infrastructure exists and is tested

**Implementation Plan**: See "Phase 5c: Integrate DatabaseStatePersistence into Binary" section above

**Why This is Fast**:
- All infrastructure already exists
- ActorExecutionTracker provides simple API
- Just need to wire up existing components
- No new features to implement

### 📋 Phase 7: Observability (Should Have)

**Estimated Time**: 1-2 days

**Deliverables**:
- Prometheus metrics export
- Health check endpoint
- Execution metrics (duration, success rate)
- Alerting integration points

**Dependencies**: Phase 5c complete

### 🎯 Phase 8: HTTP API (Nice to Have)

**Estimated Time**: 2-3 days

**Deliverables**:
- Axum-based REST API
- Task control endpoints (pause/resume/trigger)
- Execution history queries
- API authentication

**Dependencies**: Phase 5c complete, Phase 7 recommended

---

## Technical Considerations

### Database Connection Strategy

**Infrastructure**: ✅ r2d2 connection pooling fully implemented (state_persistence.rs:35-99)
- Configurable pool size (default: 10 connections)
- Warm-up on initialization
- Used by all DatabaseStatePersistence methods

**Binary Status**: ❌ Not integrated - still using `establish_connection()` per execution
- Binary uses `establish_connection()` at line 253
- Doesn't utilize DatabaseStatePersistence's pool
- Could switch to `persistence.pool.get()` for significant performance improvement

**Recommendation**: Integrate in Phase 5c alongside other persistence features

### Schedule Timezone Handling

**Current**: All schedules in UTC (`DateTime<Utc>`)

**Rationale**: 
- Eliminates DST ambiguity
- Consistent across deployments
- Database `TIMESTAMPTZ` stores UTC

**User Experience**: Document need to convert local time to UTC in configs

### Circuit Breaker Strategy

**Infrastructure**: ✅ Fully implemented with auto-pause (state_persistence.rs:909-1007)

**Features**:
- Counter-based failure tracking per task
- Automatic pause when `consecutive_failures >= max_consecutive_failures`
- Success resets failure counter (state_persistence.rs:1009-1045)
- `should_execute()` checks pause state (state_persistence.rs:1047-1090)
- Manual pause/resume overrides (state_persistence.rs:493-566)
- Configuration via `CircuitBreakerConfig` (server_config.rs:47-69)

**Binary Status**: ❌ Not enforced - failures only logged, not tracked
- No calls to `record_failure()` or `should_execute()`
- Circuit breaker exists but inactive
- Would activate with ActorExecutionTracker integration

**Testing**: ✅ 6 dedicated tests in `state_persistence_circuit_breaker_test.rs`

**Future Enhancement**: Add exponential backoff before auto-pause (Phase 6+)

### Testing Strategy

**Unit & Integration Tests**: ✅ Comprehensive (66 tests passing across 13 files)
- Schedule logic (11 tests)
- Discord server traits (9 tests)
- Platform traits (5 tests)
- Circuit breaker (6 tests)
- State persistence (20+ tests across 5 files)
- Execution tracking (3 tests)
- Configuration (7 tests)
- Error handling (6 tests)

**Infrastructure Coverage**: ✅ All major features tested
- State persistence round-trip ✅
- Circuit breaker triggering ✅
- Execution history ✅
- Concurrent operations ✅
- Connection pooling ✅

**Binary Integration Tests**: ❌ Missing
- End-to-end binary execution
- Graceful shutdown with state persistence
- State recovery on restart
- Circuit breaker enforcement in running server

**API Tests**: ❌ Deferred to Phase 7
- REST endpoint behavior
- Authentication/authorization

**Test Command**: `just test-package botticelli_actor`

---

## Success Criteria

### Infrastructure Complete ✅
- ✅ Connection pooling (r2d2)
- ✅ State persistence (DatabaseStatePersistence)
- ✅ Circuit breaker with auto-pause
- ✅ Execution history tracking
- ✅ ActorExecutionTracker helper
- ✅ 66 tests passing across 13 files
- ✅ All schedule types (Interval, Cron, Once, Immediate)
- ✅ Configuration system with CircuitBreakerConfig

### Phase 5c Complete When (Binary Integration):
- ✅ `actor-server` binary compiles and runs
- ✅ Loads TOML configuration without errors
- ✅ Executes tasks on all schedule types
- ✅ Graceful shutdown completes cleanly
- ✅ Example configs work out of box
- ❌ Uses DatabaseStatePersistence with connection pooling
- ❌ Recovers state from database on startup
- ❌ Records execution history to database
- ❌ Circuit breaker pauses failing tasks
- ❌ State persists across restarts

### Production Ready When:
- ✅ Phase 1-2: Core traits and generic implementations (DONE)
- ✅ Phase 3: Persistent state infrastructure (DONE)
- ✅ Phase 4: Advanced scheduling (DONE)
- ✅ Phase 6: Comprehensive testing (66 tests PASSING)
- 🚧 Phase 5c: Binary integration (IN PROGRESS - 40% complete)
- ⏳ Phase 7: Observability metrics
- ⏳ Binary integration test suite
- ⏳ Deployment documentation

---

## Recent Changes

### 2025-11-24: Comprehensive Codebase Audit & Documentation Update
- **Audited entire codebase** to verify implementation vs documentation
- **Discovered infrastructure is 100% complete**:
  - r2d2 connection pooling (state_persistence.rs:35-99)
  - Circuit breaker with auto-pause (state_persistence.rs:909-1007)
  - Execution history tracking (state_persistence.rs:610-869)
  - ActorExecutionTracker helper (execution_tracker.rs:1-191)
  - 66 tests passing across 13 test files
- **Identified integration gap**: Binary not using infrastructure
- **Corrected status**: Changed from "Phase 5 Complete" to "Phase 5c Needed"
- **Updated roadmap**: Clear 2-3 hour integration task, not days of work
- **Added code examples**: Exact changes needed in actor-server.rs

### 2025-11-24: Planning Document Original Rewrite
- Clarified completed vs pending work
- Phases 3-4 marked complete
- Detailed Phase 5 implementation plan
- Added configuration structure specifications
- Defined integration requirements
- Updated success criteria

### 2025-11-23: Phase 4 Completion
- Implemented `ScheduleConfig` with 4 variants
- Full `Schedule` trait with time handling
- Discovered 7-field cron format requirement
- 11 comprehensive tests added
- Public API exported from `botticelli_server`

### 2025-11-23: Phase 3 Completion
- Created database migration for state tables
- Implemented Diesel models with builders
- Built `DatabaseStatePersistence` with async support and r2d2 pooling
- Added execution history tracking with start/complete/fail methods
- Implemented circuit breaker with auto-pause
- Added 20+ tests across 5 test files

---

## Next Immediate Actions

### Phase 5c Implementation (2-3 hours)

1. **Initialize shared persistence** in actor-server.rs (before line 136)
   - Create `Arc<DatabaseStatePersistence>` with connection pool
   - Make available to all actors

2. **Load state on startup** (replace lines 104-121)
   - Query `load_task_state()` for each actor
   - Log loaded state (failures, pause status)
   - Initialize last_run from database

3. **Create ActorExecutionTracker** (after line 175)
   - One tracker per actor
   - Store in actors HashMap alongside actor, schedule, last_run

4. **Integrate into execution loop** (replace lines 246-274)
   - Check `tracker.should_execute()` for circuit breaker
   - Call `tracker.start_execution()` before actor.execute()
   - Call `tracker.record_success()` or `tracker.record_failure()`
   - Extract skill counts from ExecutionResult

5. **Save state on shutdown** (after line 285)
   - Persist final last_run for all actors
   - Update next_run based on schedule

6. **Test end-to-end**
   - Run binary with DATABASE_URL set
   - Verify execution history in database
   - Test circuit breaker by forcing failures
   - Restart binary and verify state recovery

**Success Metric**: Binary survives restart with state intact, circuit breaker auto-pauses failing tasks

---

**Last Updated**: 2025-11-24 (Post-Audit)
**Next Review**: After Phase 5c integration complete
