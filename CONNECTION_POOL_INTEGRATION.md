# Connection Pool Integration Plan

## Problem Statement

Current implementation uses single `PgConnection` instances, leading to:
- Race conditions in concurrent tests
- No connection reuse across operations
- Manual mutex management in tests
- Poor scalability for multi-threaded actor server

## Solution: r2d2 Connection Pool

Diesel already supports r2d2 connection pooling. We need to integrate it properly.

## Dependencies

Already available in workspace:
- `diesel` with r2d2 features
- `r2d2` itself (referenced in root Cargo.toml)

Need to add to `botticelli_database/Cargo.toml`:
```toml
r2d2 = { workspace = true }
```

## Implementation Phases

### Phase 1: Add Pool Infrastructure to botticelli_database

**Files:**
- `crates/botticelli_database/Cargo.toml` - Add r2d2 dependency
- `crates/botticelli_database/src/connection.rs` - Add pool creation

**Changes:**

```rust
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Create a connection pool for the PostgreSQL database.
#[instrument(name = "database.create_pool")]
pub fn create_pool() -> DatabaseResult<DbPool> {
    let database_url = std::env::var("DATABASE_URL")?;
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    
    Pool::builder()
        .max_size(10) // Configurable default
        .build(manager)
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))
}

/// Get connection from pool (convenience for existing code).
pub fn get_connection(pool: &DbPool) -> DatabaseResult<DbConnection> {
    pool.get()
        .map_err(|e| DatabaseError::new(DatabaseErrorKind::Connection(e.to_string())))
}
```

**Export in lib.rs:**
```rust
pub use connection::{create_pool, get_connection, DbPool, DbConnection};
```

**Benefits:**
- Connection reuse
- Automatic connection management
- Built-in concurrency handling
- Configurable pool size

### Phase 2: Update botticelli_actor to Use Pool

**Files:**
- `crates/botticelli_actor/src/state_persistence.rs`
- `crates/botticelli_actor/src/execution_tracker.rs`

**Changes:**

```rust
use botticelli_database::{DbPool, get_connection};

pub struct DatabaseStatePersistence {
    pool: DbPool,  // Changed from individual connections
}

impl DatabaseStatePersistence {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
    
    // All methods get connection from pool:
    fn save_state(&self, task_id: &str, state: &str) -> Result<()> {
        let mut conn = get_connection(&self.pool)?;
        // Use conn...
    }
}
```

**Migration Path:**
1. Keep `establish_connection()` for backward compatibility
2. Add pool-based constructors (`::with_pool()`)
3. Update tests to use pools
4. Eventually deprecate single-connection usage

### Phase 3: Update Tests

**Files:**
- `tests/actor_database_persistence_test.rs`
- `tests/actor_execution_tracker_test.rs`

**Changes:**

```rust
use botticelli_database::create_pool;

#[tokio::test]
async fn test_concurrent_operations() {
    let pool = create_pool().expect("Pool creation");
    
    let persistence = DatabaseStatePersistence::new(pool.clone());
    
    // No more mutexes needed - pool handles concurrency
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let p = persistence.clone();
            tokio::spawn(async move {
                p.save_state(&format!("task_{}", i), "data").await
            })
        })
        .collect();
    
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
}
```

**Benefits:**
- Remove all test mutexes
- Simpler test code
- More realistic concurrency testing
- Faster parallel execution

### Phase 4: Server Integration

**Files:**
- `crates/botticelli_actor/src/server.rs`
- `crates/botticelli_actor/src/discord_server.rs`

**Changes:**

```rust
pub struct ActorServer<P: PlatformTrait> {
    config: ServerConfig,
    pool: DbPool,  // Shared across all operations
    // ...
}

impl<P: PlatformTrait> ActorServer<P> {
    pub fn new(config: ServerConfig, pool: DbPool) -> Self {
        Self { config, pool, /* ... */ }
    }
    
    async fn execute_actor(&self, actor: &Actor) -> Result<()> {
        // Each execution gets its own connection from pool
        let mut conn = get_connection(&self.pool)?;
        // ...
    }
}
```

**Benefits:**
- Single pool shared across all actors
- Efficient connection reuse
- Proper resource management
- Production-ready concurrency

### Phase 5: Configuration

**Files:**
- `crates/botticelli_actor/src/server_config.rs`

**Add pool configuration:**

```rust
#[derive(Debug, Clone, Builder)]
pub struct DatabaseConfig {
    #[builder(default = "10")]
    pub max_pool_size: u32,
    
    #[builder(default = "30")]
    pub connection_timeout_seconds: u64,
    
    #[builder(default = "Some(300)")]
    pub idle_timeout_seconds: Option<u64>,
}
```

**Benefits:**
- Tunable performance
- Environment-specific settings
- Production vs test configurations

## Testing Strategy

### Unit Tests
- Pool creation success/failure
- Connection acquisition
- Connection return to pool
- Pool exhaustion handling

### Integration Tests
- Concurrent operations (no races)
- Connection reuse verification
- Pool size limits
- Timeout behavior

### Performance Tests
- Benchmark pool vs single connection
- Measure connection reuse benefits
- Test under load

## Migration Checklist

- [x] Phase 1: Add r2d2 dependency to botticelli_actor
- [x] Phase 2: Update DatabaseStatePersistence to use pool
  - DatabaseStatePersistence now owns a `Pool<ConnectionManager<PgConnection>>`
  - All methods clone pool and spawn_blocking with pooled connections
  - Constructor changed to `new() -> ActorServerResult<Self>`
- [x] Phase 3: Update tests
  - All test calls updated to `new().expect("Failed to create persistence")`
  - Tests now automatically benefit from connection pooling
  - No mutex management needed in tests
- [x] Phase 4: Update actor-server binary
  - Updated imports and error handling for new API
- [ ] Phase 5: Add configuration (deferred - using defaults)
- [x] Run `just check botticelli_actor`
- [x] Run `just test-package botticelli_actor`
- [ ] Update documentation
- [ ] Commit changes

## Implementation Summary

### What We Did

**Simplified Approach:** Instead of adding pool infrastructure to botticelli_database, we integrated r2d2 directly into botticelli_actor where it's needed. This:
- Keeps database crate simple (just schema + operations)
- Puts connection management where state persistence lives
- Avoids unnecessary abstraction layers

**Key Changes:**
1. Added `r2d2 = "0.8"` to botticelli_actor/Cargo.toml
2. Updated DatabaseStatePersistence:
   - Added `pool: Pool<ConnectionManager<PgConnection>>` field
   - Constructor creates pool from DATABASE_URL
   - All async methods clone pool for spawn_blocking
   - Each operation gets connection via `pool.get()`
3. Updated all 27 test files to use new `.expect()` pattern
4. Fixed actor-server binary imports

**Database Operations:**
- All operations run in `spawn_blocking` with pooled connections
- Pool handles concurrency automatically
- No more race conditions in tests
- Connection reuse across operations

## Success Criteria

1. ✅ All tests pass without mutexes
2. ✅ No race conditions in concurrent tests  
3. ✅ Pool-based connection management
4. ✅ Clean compilation (zero errors)
5. ⚠️  Some clippy warnings remain (unused in non-discord builds)
6. ⏳ Documentation needs update

## Future Enhancements

- Connection pool metrics/monitoring
- Dynamic pool sizing based on load
- Health checks on connections
- Connection retry logic with backoff
- Per-actor connection quotas
- Configurable pool settings (currently hardcoded to max_size=10)
- Pool configuration in actor_server.toml

## Performance Notes

**Default Configuration:**
- Pool size: 10 connections
- No connection timeout (uses r2d2 defaults)
- No idle timeout (connections persist)

**Recommended Tuning:**
- Development: 5-10 connections sufficient
- Production: 20-50 connections depending on load
- Monitor pool exhaustion in logs
- Adjust based on actual concurrent actor count

**Connection Lifecycle:**
- Connections created lazily on first use
- Reused across operations automatically
- Returned to pool after each operation
- Pool manages connection health

## Lessons Learned

1. **Simplicity Wins**: Direct r2d2 integration in botticelli_actor was simpler than adding infrastructure to botticelli_database
2. **spawn_blocking Pattern**: Clone pool before spawn_blocking, get connection inside closure
3. **Error Handling**: Pool creation can fail, must handle at construction time
4. **Test Updates**: Bulk sed replacement worked well for updating 27 test files
5. **No More Mutexes**: Pool eliminates need for manual synchronization in tests
