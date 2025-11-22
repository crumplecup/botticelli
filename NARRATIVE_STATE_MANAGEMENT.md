# Narrative State Management

## Lessons from botticelli_cache

The existing `botticelli_cache` crate provides excellent patterns we should follow:

### Architecture Patterns

1. **Configuration via Config struct**
   - `CommandCacheConfig` with sensible defaults (TTL, max_size, enabled)
   - Builder pattern via `derive_builder::Builder`
   - Getters/Setters via derive macros with `with_` prefix
   - Serde serialization support for persistence

2. **Typed entry storage**
   - `CacheEntry` wraps values with metadata (`value`, `created_at`, `ttl`)
   - Clean separation: `CacheKey` for lookup, `CacheEntry` for storage
   - Expiration logic encapsulated in entry methods (`is_expired()`, `time_remaining()`)
   - Derive patterns: `Debug, Clone, Getters`

3. **Comprehensive tracing**
   - All public methods use `#[instrument]` with relevant fields
   - Debug logs for cache hits/misses with timing info
   - Performance metrics (cache_size, time_remaining, eviction events)

4. **Smart eviction strategy**
   - LRU tracking via `access_order: Vec<CacheKey>`
   - Automatic cleanup of expired entries
   - Configurable max_size with eviction on overflow

### Key Differences for NarrativeState

- **Persistence**: Cache uses `Instant` (runtime-only) - we need wall-clock timestamps
- **Value types**: Cache stores arbitrary JSON - we need typed state (IDs, configs)
- **Key structure**: Cache uses hash-based keys - we need hierarchical keys (guild.narrative.resource)
- **Lifetime**: Cache is ephemeral (per-execution) - state persists across runs

### Recommended Patterns to Adopt

1. Use same config pattern: `StateConfig` with builder
2. Entry wrapper: `StateEntry { value, created_at, updated_at, metadata }`
3. Comprehensive instrumentation on all operations
4. Separation of concerns: key generation vs storage vs persistence

## Problem Statement

Currently, bot commands that create resources (like Discord channels) return IDs that are only available during the current narrative execution. When narratives run multiple times or reference resources created in previous runs, we have no way to persist and retrieve these IDs.

### Current Issues

1. **Channel IDs**: When `channels.create` runs, it returns a channel ID, but on subsequent runs we can't retrieve it
2. **Message IDs**: Similar issue with message IDs needed for pinning, editing, or replying
3. **Cross-Narrative References**: One narrative creates a resource, another needs to reference it later
4. **Idempotency**: Running the same narrative twice creates duplicate resources instead of reusing existing ones

## Requirements

### Core Functionality

1. **Persistent Key-Value Store**: Store arbitrary state between narrative runs
2. **Namespacing**: State should be scoped to avoid collisions (by guild, narrative, resource type, etc.)
3. **Automatic Capture**: Bot commands should automatically capture important IDs
4. **Manual Access**: Narratives should be able to explicitly set/get state values
5. **Cleanup**: Old state should be clearable/expirable

### Use Cases

**Discord Channel Management:**
```toml
# First run: Creates channel and stores ID
[[act]]
name = "setup_welcome"
[[act.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"
[act.input.args]
name = "welcome"
# After execution: state["discord.channel.welcome"] = "1234567890"

# Second run: Retrieves existing channel ID
[[act]]
name = "post_to_welcome"
[[act.input]]
type = "bot_command"
platform = "discord"
command = "messages.send"
[act.input.args]
channel_id = "{state.discord.channel.welcome}"  # Retrieves stored ID
content = "Hello!"
```

**Message Pinning:**
```toml
# Store message ID from send operation
[[act]]
name = "send_important_message"
output_var = "welcome_message_id"  # Explicitly capture to state
[[act.input]]
type = "bot_command"
command = "messages.send"
# ... args ...

# Use stored ID for pinning
[[act]]
name = "pin_message"
[[act.input]]
type = "bot_command"
command = "messages.pin"
[act.input.args]
message_id = "{state.welcome_message_id}"
```

## Design Options

### Option 1: Database-Backed State Store

**Implementation:**
- New table: `narrative_state (key TEXT PRIMARY KEY, value JSONB, updated_at TIMESTAMP)`
- Scoped keys: `{guild_id}.{narrative_name}.{resource_type}.{resource_name}`
- Accessed via trait: `StateRepository`

**Pros:**
- Durable across process restarts
- Queryable for debugging/cleanup
- Transactional consistency with content tables
- Natural fit with existing database architecture

**Cons:**
- Requires database feature
- Slight performance overhead
- Need migration for new table

### Option 2: File-Based State Store

**Implementation:**
- JSON file per guild/narrative: `.botticelli/state/{guild_id}/{narrative_name}.json`
- Simple key-value structure
- Optional file locking for concurrent access

**Pros:**
- No database dependency
- Human-readable/editable
- Simple implementation
- Portable across environments

**Cons:**
- Less robust (file corruption, permissions issues)
- No transaction support
- Harder to query/aggregate
- Concurrent access challenges

### Option 3: Hybrid Approach

**Implementation:**
- State stored in database when available
- Falls back to file-based when database disabled
- Unified trait interface: `StateStore`

**Pros:**
- Works in all configurations
- Durability where needed
- Flexibility for different use cases

**Cons:**
- More complex implementation
- Need to maintain two backends
- Potential inconsistency between modes

## Recommended Approach: Option 1 (Database-Backed)

Given Botticelli's existing architecture and primary use cases, database-backed state is the best fit:

1. **Already Required**: Most narratives use database for content generation anyway
2. **Integration**: Natural extension of existing repository pattern
3. **Reliability**: Transactions and durability are important for production bots
4. **Tooling**: Can use existing database tools for debugging and cleanup

### Implementation Plan

#### Phase 1: Core State Storage

1. **Create state table migration**
   ```sql
   CREATE TABLE narrative_state (
       key TEXT PRIMARY KEY,
       value JSONB NOT NULL,
       created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
   );
   
   CREATE INDEX idx_narrative_state_key_prefix ON narrative_state(key text_pattern_ops);
   ```

2. **Add StateRepository trait** (in `botticelli_interface`)
   ```rust
   pub trait StateRepository {
       fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;
       fn set(&self, key: &str, value: serde_json::Value) -> Result<()>;
       fn delete(&self, key: &str) -> Result<()>;
       fn list_with_prefix(&self, prefix: &str) -> Result<Vec<(String, serde_json::Value)>>;
       fn clear_with_prefix(&self, prefix: &str) -> Result<usize>;
   }
   ```

3. **Implement PostgresStateRepository** (in `botticelli_database`)

#### Phase 2: Bot Command Integration

1. **Update BotCommand trait** to return structured output
   ```rust
   pub struct CommandOutput {
       pub success: bool,
       pub message: String,
       pub captured_ids: HashMap<String, String>,  // e.g., {"channel_id": "123"}
   }
   ```

2. **Auto-capture in executor**: When bot commands complete, automatically store relevant IDs

3. **Add state interpolation**: Support `{state.key}` syntax in TOML args

#### Phase 3: TOML Syntax Enhancement

1. **Output variable binding**
   ```toml
   [[act]]
   name = "create_channel"
   output_var = "support_channel"  # Binds to state["support_channel"]
   ```

2. **State references in args**
   ```toml
   [act.input.args]
   channel_id = "{state.support_channel}"
   ```

3. **Conditional execution based on state**
   ```toml
   [[act]]
   name = "create_if_missing"
   skip_if = "{state.channel_exists}"
   ```

#### Phase 4: Tooling and Cleanup

1. **CLI commands**
   - `botticelli state list [--prefix PREFIX]`
   - `botticelli state get KEY`
   - `botticelli state set KEY VALUE`
   - `botticelli state clear [--prefix PREFIX]`

2. **Automatic cleanup strategies**
   - TTL/expiration support
   - Cleanup on narrative deletion
   - Guild-scoped cleanup

## Key Design Decisions

### State Key Namespacing

Use hierarchical keys with dots:
- `discord.{guild_id}.channel.{channel_name}` - Channel IDs
- `discord.{guild_id}.message.{message_name}` - Message IDs
- `narrative.{narrative_name}.{var_name}` - Narrative-scoped variables
- `global.{key}` - Cross-narrative shared state

### JSON Value Storage

Store values as JSONB for flexibility:
```json
{
  "id": "1234567890",
  "name": "welcome",
  "created_at": "2025-11-21T23:00:00Z",
  "metadata": {...}
}
```

### State Scope and Isolation

- **Guild-scoped**: Most Discord state should be per-guild
- **Narrative-scoped**: Temporary variables within a narrative run
- **Global**: Rarely needed, but available for cross-guild coordination

## Migration Path

1. **Phase 1**: Add state table, implement repository, basic get/set
2. **Phase 2**: Update executor to support state interpolation
3. **Phase 3**: Update bot commands to auto-capture IDs
4. **Phase 4**: Add CLI tooling and cleanup strategies
5. **Phase 5**: Update existing narratives to use state for idempotency

## Example: Updated publish_welcome.toml

```toml
[metadata]
name = "publish_welcome"
description = "Complete welcome channel setup with state management"

# Act 1: Setup channel (idempotent - reuses if exists)
[[act]]
name = "setup_channel"
output_var = "welcome_channel"  # Stores to state["welcome_channel"]
skip_if = "{state.discord.${TEST_GUILD_ID}.channel.welcome}"  # Skip if already exists

[[act.input]]
type = "bot_command"
platform = "discord"
command = "channels.create"
[act.input.args]
guild_id = "${TEST_GUILD_ID}"
name = "welcome"
topic = "Welcome to Botticelli!"

# Act 2: Clear existing messages (uses stored channel ID)
[[act]]
name = "clear_channel"
[[act.input]]
type = "bot_command"
platform = "discord"
command = "messages.bulk_delete"
[act.input.args]
channel_id = "{state.welcome_channel.id}"  # Uses stored ID
limit = 100

# Act 3-4: Generate content (unchanged)
# ...

# Act 5: Publish message
[[act]]
name = "publish_message"
output_var = "welcome_message"  # Stores message ID

[[act.input]]
type = "bot_command"
platform = "discord"
command = "messages.send"
[act.input.args]
channel_id = "{state.welcome_channel.id}"
content = "{act.select_best.output}"

# Act 6: Pin message (uses stored message ID)
[[act]]
name = "pin_message"
[[act.input]]
type = "bot_command"
platform = "discord"
command = "messages.pin"
[act.input.args]
channel_id = "{state.welcome_channel.id}"
message_id = "{state.welcome_message.id}"
```

## Next Steps

1. Create database migration for narrative_state table
2. Implement StateRepository trait and PostgreSQL backend
3. Add state interpolation to narrative executor
4. Update bot command return types to capture IDs
5. Update TOML parser to support output_var and state references
6. Add CLI commands for state management
7. Update documentation and examples
