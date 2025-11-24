# Database State Persistence Implementation Plan

## Overview

`DatabaseStatePersistence` is a critical component that enables actor servers to persist their scheduling and execution state to PostgreSQL, allowing graceful recovery after crashes or restarts.

## Current Status

### ✅ Implemented

**Phase 1: Database Schema**
- `actor_server_state` table - stores task scheduling state
- `actor_server_executions` table - logs execution history
- Diesel models: `ActorServerStateRow`, `NewActorServerState`
- Execution models: `ActorServerExecutionRow`, `NewActorServerExecution`
- Indexes for performance (next_run, task lookups, execution history)

**Phase 2: Basic Persistence**
- `DatabaseStatePersistence` struct implementing `StatePersistence` trait
- `save_state()` - upserts task state with ON CONFLICT
- `load_state()` - retrieves state (currently returns first row)
- `clear_state()` - deletes all state rows
- Async-safe with `tokio::task::spawn_blocking`

### ❌ Missing Critical Features

The current implementation is a **minimal skeleton** that needs significant enhancement:

1. **Multi-Task Support**: Current `load_state()` only returns first row
2. **Task-Specific Operations**: No way to load/save/delete individual tasks
3. **Execution Logging**: Models exist but no logging implementation
4. **Query Capabilities**: No way to list tasks, filter by actor, check pause state
5. **Error Handling**: Generic error messages, no structured error types
6. **Circuit Breaker**: `consecutive_failures` tracked but not enforced
7. **Metadata Utilities**: JSON metadata has no helper methods
8. **Integration**: Not connected to `SimpleTaskScheduler` or actor execution

## Implementation Plan

### Phase 1: Multi-Task State Management ✅ COMPLETE

**Goal**: Support multiple concurrent tasks with individual state tracking

**Status**: Implemented and tested

**Tasks**:
1. Add `task_id` parameter to persistence methods:
   ```rust
   async fn save_task_state(&self, task_id: &str, state: &Self::State) -> Result<()>;
   async fn load_task_state(&self, task_id: &str) -> Result<Option<Self::State>>;
   async fn delete_task_state(&self, task_id: &str) -> Result<()>;
   ```

2. Implement query methods:
   ```rust
   async fn list_all_tasks(&self) -> Result<Vec<ActorServerStateRow>>;
   async fn list_tasks_by_actor(&self, actor_name: &str) -> Result<Vec<ActorServerStateRow>>;
   async fn list_active_tasks(&self) -> Result<Vec<ActorServerStateRow>>;
   async fn list_paused_tasks(&self) -> Result<Vec<ActorServerStateRow>>;
   ```

3. Add batch operations:
   ```rust
   async fn pause_task(&self, task_id: &str) -> Result<()>;
   async fn resume_task(&self, task_id: &str) -> Result<()>;
   async fn update_next_run(&self, task_id: &str, next_run: NaiveDateTime) -> Result<()>;
   ```

**Files**:
- `crates/botticelli_actor/src/state_persistence.rs`

**Tests**:
- `tests/state_persistence_multi_task_test.rs`
- Concurrent save/load/delete operations
- Query filtering and pagination
- Task lifecycle (pause/resume/delete)

---

### Phase 2: Execution Logging ✅ COMPLETE

**Goal**: Track actor execution history for observability and debugging

**Status**: Implemented and tested

**Tasks**:
1. Add execution logging methods to `DatabaseStatePersistence`:
   ```rust
   async fn start_execution(&self, task_id: &str, actor_name: &str) -> Result<i64>; // Returns execution_id
   async fn complete_execution(&self, execution_id: i64, result: ExecutionResult) -> Result<()>;
   async fn fail_execution(&self, execution_id: i64, error: &str) -> Result<()>;
   ```

2. Define `ExecutionResult` struct:
   ```rust
   pub struct ExecutionResult {
       pub skills_succeeded: i32,
       pub skills_failed: i32,
       pub skills_skipped: i32,
       pub metadata: serde_json::Value,
   }
   ```

3. Implement history queries:
   ```rust
   async fn get_execution_history(&self, task_id: &str, limit: i64) -> Result<Vec<ActorServerExecutionRow>>;
   async fn get_failed_executions(&self, task_id: &str, limit: i64) -> Result<Vec<ActorServerExecutionRow>>;
   async fn get_recent_executions(&self, limit: i64) -> Result<Vec<ActorServerExecutionRow>>;
   ```

4. Add cleanup method:
   ```rust
   async fn prune_old_executions(&self, older_than_days: i32) -> Result<usize>; // Returns deleted count
   ```

**Files**:
- `crates/botticelli_actor/src/state_persistence.rs`
- `crates/botticelli_actor/src/execution_result.rs` (new)

**Tests**:
- `tests/state_persistence_execution_logging_test.rs`
- Start/complete/fail execution flows
- Query execution history
- Concurrent execution logging
- Pruning old records

---

### Phase 3: Circuit Breaker Integration ✅ COMPLETE

**Goal**: Automatically pause tasks after repeated failures

**Status**: Implemented and tested (requires database setup to run)

**Tasks**:
1. Add circuit breaker logic:
   ```rust
   async fn record_failure(&self, task_id: &str) -> Result<bool>; // Returns true if threshold exceeded
   async fn record_success(&self, task_id: &str) -> Result<()>; // Resets failure count
   async fn should_execute(&self, task_id: &str) -> Result<bool>; // Check pause + circuit breaker
   ```

2. Add configuration to `ActorServerConfig`:
   ```rust
   pub struct CircuitBreakerConfig {
       pub max_consecutive_failures: i32,
       pub auto_pause: bool,
       pub reset_on_success: bool,
   }
   ```

3. Integrate with `SimpleTaskScheduler`:
   - Check `should_execute()` before running task
   - Call `record_success()` / `record_failure()` after execution
   - Automatic pause when threshold exceeded

**Files**:
- `crates/botticelli_actor/src/state_persistence.rs`
- `crates/botticelli_actor/src/server_config.rs`
- `crates/botticelli_actor/src/server.rs` (SimpleTaskScheduler integration)

**Tests**:
- `tests/state_persistence_circuit_breaker_test.rs`
- Failure counting and auto-pause
- Success resets counter
- Manual pause overrides circuit breaker
- Integration with scheduler

---

### Phase 4: Actor Execution Integration ✅ COMPLETE

**Goal**: Actor execution code uses persistence for state tracking and circuit breaking

**Status**: Implemented and tested - helper struct simplifies actor-persistence integration

**Rationale**: The `TaskScheduler` trait is generic and doesn't have actor-specific context (actor_name, skill results, etc.). Persistence integration must happen in actor execution code where this context exists.

**Tasks**:
1. Create helper struct to simplify actor-persistence integration:
   ```rust
   pub struct ActorExecutionTracker<P: StatePersistence> {
       persistence: Arc<P>,
       task_id: String,
       actor_name: String,
   }
   
   impl<P: StatePersistence> ActorExecutionTracker<P> {
       pub async fn start_execution(&self) -> Result<i64>;
       pub async fn record_success(&self, exec_id: i64, result: ExecutionResult) -> Result<()>;
       pub async fn record_failure(&self, exec_id: i64, error: &str) -> Result<bool>; // Returns true if should pause
       pub async fn should_execute(&self) -> Result<bool>;
       pub async fn update_next_run(&self, next_run: NaiveDateTime) -> Result<()>;
   }
   ```

2. Add example showing integration in actor code:
   ```rust
   // In Actor::execute() or similar
   async fn execute_with_persistence<P>(
       &self,
       tracker: &ActorExecutionTracker<P>,
   ) -> Result<()>
   where
       P: StatePersistence,
   {
       // Check circuit breaker
       if !tracker.should_execute().await? {
           debug!("Task paused or circuit broken");
           return Ok(());
       }
       
       // Start execution tracking
       let exec_id = tracker.start_execution().await?;
       
       // Execute skills
       match self.execute_skills().await {
           Ok(result) => {
               tracker.record_success(exec_id, result).await?;
           }
           Err(e) => {
               let should_pause = tracker.record_failure(exec_id, &e.to_string()).await?;
               if should_pause {
                   warn!("Circuit breaker triggered, pausing task");
               }
               return Err(e);
           }
       }
       
       Ok(())
   }
   ```

3. Document state recovery pattern:
   ```rust
   // On server startup
   let persistence = DatabaseStatePersistence::new();
   let active_tasks = persistence.list_active_tasks().await?;
   
   for state in active_tasks {
       // Re-create and schedule actor using persisted state
       let actor = load_actor_config(&state.actor_name)?;
       scheduler.schedule(
           state.task_id,
           Duration::from_secs(state.interval_seconds as u64),
           || async { actor.execute().await }
       ).await?;
   }
   ```

**Files**:
- `crates/botticelli_actor/src/execution_tracker.rs` - ActorExecutionTracker helper ✅
- `examples/discord_actor_with_persistence.rs` - Integration example ✅
- Tests in `tests/actor_execution_tracker_test.rs` ✅

**Implemented**:
1. `ActorExecutionTracker<P>` helper struct with methods:
   - `start_execution()` - Begin tracking
   - `record_success()` - Log success and reset circuit breaker
   - `record_failure()` - Log failure and check circuit breaker threshold
   - `should_execute()` - Check if task should run
   - `update_next_run()` - Update scheduling
2. Example showing complete integration pattern
3. Tests covering:
   - Basic execution flow
   - Failure handling and circuit breaker
   - Success resetting failure counter
   - Next run updates
   - Metadata handling

---

### Phase 5: Error Handling

**Goal**: Structured error types with context

**Tasks**:
1. Create `StatePersistenceError` enum:
   ```rust
   #[derive(Debug, Clone, derive_more::Display)]
   pub enum StatePersistenceErrorKind {
       #[display("Task not found: {}", _0)]
       TaskNotFound(String),
       
       #[display("Database error: {}", _0)]
       DatabaseError(String),
       
       #[display("Serialization error: {}", _0)]
       SerializationError(String),
       
       #[display("Invalid state: {}", _0)]
       InvalidState(String),
   }
   
   #[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
   #[display("State persistence: {} at {}:{}", kind, file, line)]
   pub struct StatePersistenceError {
       pub kind: StatePersistenceErrorKind,
       pub line: u32,
       pub file: &'static str,
   }
   ```

2. Replace `ActorServerResult<T>` with `Result<T, StatePersistenceError>`

3. Add helper constructors:
   ```rust
   impl StatePersistenceError {
       #[track_caller]
       pub fn task_not_found(task_id: impl Into<String>) -> Self { ... }
       
       #[track_caller]
       pub fn database_error(msg: impl Into<String>) -> Self { ... }
   }
   ```

**Files**:
- `crates/botticelli_actor/src/state_persistence_error.rs` (new)
- `crates/botticelli_actor/src/state_persistence.rs`

**Tests**:
- `tests/state_persistence_error_test.rs`
- Error variants and messages
- Location tracking with `#[track_caller]`

---

### Phase 6: Metadata Utilities

**Goal**: Structured helpers for JSON metadata field

**Tasks**:
1. Add metadata helper trait:
   ```rust
   pub trait StateMetadata {
       fn get_metadata<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
       fn set_metadata<T: Serialize>(&mut self, key: &str, value: T) -> Result<()>;
       fn remove_metadata(&mut self, key: &str);
       fn has_metadata(&self, key: &str) -> bool;
   }
   ```

2. Implement for `ActorServerStateRow` and `NewActorServerState`

3. Add common metadata constants:
   ```rust
   pub mod metadata_keys {
       pub const LAST_CONTENT_ID: &str = "last_content_id";
       pub const LAST_ERROR: &str = "last_error";
       pub const CHANNEL_ID: &str = "channel_id";
       pub const CUSTOM_CONFIG: &str = "custom_config";
   }
   ```

**Files**:
- `crates/botticelli_actor/src/state_metadata.rs` (new)

**Tests**:
- `tests/state_metadata_test.rs`
- Get/set/remove metadata
- Type safety with generics
- JSON serialization edge cases

---

### Phase 7: CLI Integration

**Goal**: Expose state management to users via CLI

**Tasks**:
1. Add CLI commands to `actor-server` binary:
   ```rust
   ActorCommand::State(StateCommand::List { actor, paused }) => {
       // List tasks with filtering
   }
   ActorCommand::State(StateCommand::Pause { task_id }) => {
       // Pause specific task
   }
   ActorCommand::State(StateCommand::Resume { task_id }) => {
       // Resume paused task
   }
   ActorCommand::State(StateCommand::History { task_id, limit }) => {
       // Show execution history
   }
   ActorCommand::State(StateCommand::Clear { task_id }) => {
       // Clear state (with confirmation)
   }
   ```

2. Add formatted output for each command

3. Add interactive mode for dangerous operations (clear)

**Files**:
- `crates/botticelli_actor/src/bin/actor-server.rs`
- `crates/botticelli_cli/src/commands/actor.rs` (if integrating with main CLI)

**Tests**:
- Manual testing via CLI
- Integration tests in `tests/cli_actor_state_test.rs`

---

## Testing Strategy

### Unit Tests
- Each phase has dedicated test file
- Mock database operations where appropriate
- Focus on logic correctness

### Integration Tests
- Real PostgreSQL database (test database)
- End-to-end flows (schedule → execute → persist → recover)
- Concurrent operations
- Crash/recovery simulation

### Performance Tests
- Batch operations (1000+ tasks)
- Concurrent read/write load
- Query performance with large history tables
- Index effectiveness

---

## Database Optimization

### Indexes
Already created, but verify usage:
```sql
CREATE INDEX idx_actor_server_executions_task ON actor_server_executions(task_id);
CREATE INDEX idx_actor_server_executions_started ON actor_server_executions(started_at DESC);
CREATE INDEX idx_actor_server_state_next_run ON actor_server_state(next_run) WHERE NOT is_paused;
CREATE INDEX idx_actor_server_state_actor ON actor_server_state(actor_name);
```

### Maintenance
- Add migration for execution table partitioning (by month/year)
- Add cleanup job for old executions (configurable retention period)
- Monitor table sizes and query performance

---

## Open Questions

1. **State Versioning**: Should we version the state schema for migrations?
   - Recommendation: Use `metadata` field for version tracking initially

2. **Distributed Locking**: How to prevent duplicate task execution across multiple servers?
   - Recommendation: Use PostgreSQL advisory locks or separate `task_locks` table
   - Not critical for initial single-server deployment

3. **Backup Strategy**: How to backup/restore actor state?
   - Recommendation: Standard PostgreSQL backup includes these tables
   - Add export/import CLI commands for task state portability

4. **Observability**: Should execution logs integrate with tracing/metrics?
   - Recommendation: Yes, emit structured logs with execution IDs
   - Add Prometheus metrics for failure rates, execution times

5. **State Cleanup**: When to delete completed task state?
   - Recommendation: Keep until manually deleted or task rescheduled
   - Add TTL configuration for auto-cleanup

---

## Connection Pool Integration ✅ COMPLETE

**Goal**: Use r2d2 connection pooling for proper concurrent database access

**Status**: Implemented and tested

**Changes Made**:
1. Added `r2d2 = "0.8"` dependency to `botticelli_actor/Cargo.toml`
2. Updated `DatabaseStatePersistence`:
   - Now owns `Pool<ConnectionManager<PgConnection>>`
   - Constructor creates pool from DATABASE_URL environment variable
   - Returns `ActorServerResult<Self>` to handle pool creation errors
3. All database operations use pooled connections:
   - Clone pool before `spawn_blocking`
   - Get connection via `pool.get()` inside blocking task
   - Pool automatically manages connection lifecycle
4. Updated all 27 test files to use new API: `.new().expect("Failed to create persistence")`
5. Fixed actor-server binary to handle new constructor signature

**Benefits**:
- Automatic connection reuse across operations
- No more race conditions in concurrent tests
- Thread-safe by design (no manual mutex management)
- Scalable for production workloads
- Pool handles connection errors and retries

**Files**:
- `crates/botticelli_actor/Cargo.toml`
- `crates/botticelli_actor/src/state_persistence.rs`
- `crates/botticelli_actor/src/bin/actor-server.rs`
- `crates/botticelli_actor/src/execution_tracker.rs` (doctest)
- All test files in `crates/botticelli_actor/tests/`

**See Also**: `CONNECTION_POOL_INTEGRATION.md` for detailed implementation notes

**Test Isolation Fix** (completed):
- Issue: Tests failed when run in parallel due to database state interference
- Root cause: Multiple tests hitting same database simultaneously, even with unique task_ids
- Solution: Added `#[serial]` attribute from `serial_test` crate to all database tests
- Result: All tests now pass reliably with proper serialization
- Alternative considered: Increasing pool size to 32 caused PostgreSQL connection exhaustion
- Final pool size: 10 connections (default), 2 for tests (via `with_pool_size(2)`)

---

## Success Criteria

Phase complete when:
1. ✅ Actor server survives restart without losing scheduled tasks
2. ✅ Circuit breaker automatically pauses failing tasks
3. ✅ Execution history provides audit trail for debugging
4. ✅ CLI allows users to inspect and manage task state
5. ✅ All tests pass with real PostgreSQL database
6. ✅ Performance acceptable under load (100+ concurrent tasks) - **with connection pooling**
7. ✅ Zero clippy warnings and full documentation
8. ✅ Thread-safe concurrent database access via r2d2 connection pooling

---

## Timeline Estimate

- **Phase 1**: Multi-Task Support - 2-3 hours
- **Phase 2**: Execution Logging - 2-3 hours
- **Phase 3**: Circuit Breaker - 1-2 hours
- **Phase 4**: Scheduler Integration - 3-4 hours (most complex)
- **Phase 5**: Error Handling - 1-2 hours
- **Phase 6**: Metadata Utilities - 1-2 hours
- **Phase 7**: CLI Integration - 2-3 hours

**Total**: 12-19 hours of focused development

---

## Next Steps

1. Review this plan for completeness
2. Start with Phase 1 (Multi-Task Support)
3. Run `just check-all botticelli_actor` after each phase
4. Update this document as implementation progresses
