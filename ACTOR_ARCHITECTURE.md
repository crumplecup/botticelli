# Actor Architecture Design

**Status**: Planning  
**Created**: 2025-11-23  
**Goal**: Design and implement a platform-agnostic actor system for social media automation

## Concept Overview

An **Actor** is a configured bot that:

- Consumes **knowledge** (data from narrative-produced database tables)
- Uses **skills** (reusable capabilities like scheduling, posting)
- Operates through **platform traits** (Discord, Twitter, Bluesky, etc.)
- Provides **configurable behavior** with sensible defaults

### Core Principle: Separation of Concerns

```
┌─────────────────────────────────────────┐
│  Actor (Platform-Agnostic Logic)        │
│  - Knowledge queries                    │
│  - Skill orchestration                  │
│  - Business rules                       │
└──────────────┬──────────────────────────┘
               │
               │ SocialMediaPlatform trait
               │
    ┌──────────┴──────────┬───────────────┐
    │                     │               │
┌───▼────┐         ┌──────▼──┐      ┌────▼─────┐
│Discord │         │ Twitter │      │ Bluesky  │
│  Impl  │         │  Impl   │      │   Impl   │
└────────┘         └─────────┘      └──────────┘
```

## Architecture Components

### 1. Actor (Core)

**Location**: `crates/botticelli_actor/`

**Responsibilities**:

- Load configuration from TOML
- Query knowledge tables via `botticelli_database`
- Execute skills with context
- Orchestrate platform-agnostic workflows

**Structure**:

```rust
pub struct Actor {
    config: ActorConfig,
    knowledge: Vec<KnowledgeTable>,
    skills: SkillRegistry,
    platform: Box<dyn SocialMediaPlatform>,
}
```

**Builder Pattern**:

```rust
let actor = Actor::builder()
    .config(ActorConfig::from_file("actor.toml")?)
    .platform(DiscordPlatform::new(token))
    .build()?;
```

### 2. Platform Trait

**Location**: `crates/botticelli_actor/src/platform.rs`

**Interface**:

```rust
#[async_trait]
pub trait SocialMediaPlatform: Send + Sync {
    /// Post content immediately
    async fn post(&self, content: Content) -> PlatformResult<PostId>;
    
    /// Schedule content for future posting
    async fn schedule(&self, content: Content, time: DateTime<Utc>) 
        -> PlatformResult<ScheduleId>;
    
    /// Delete a post
    async fn delete_post(&self, id: PostId) -> PlatformResult<()>;
    
    /// Get platform-specific metadata
    fn metadata(&self) -> PlatformMetadata;
}
```

**Content Type**:

```rust
pub struct Content {
    pub text: Option<String>,
    pub media: Vec<MediaAttachment>,
    pub metadata: HashMap<String, String>,
}

pub struct MediaAttachment {
    pub url: String,
    pub media_type: MediaType,
    pub alt_text: Option<String>,
}
```

### 3. Knowledge Tables

**Location**: `crates/botticelli_actor/src/knowledge.rs`

**Purpose**: Type-safe wrappers around database tables produced by narratives

**Structure**:

```rust
pub struct KnowledgeTable {
    pub name: String,
    pub schema: TableSchema,
}

impl KnowledgeTable {
    pub fn query(&self, conn: &mut PgConnection) 
        -> DatabaseResult<Vec<Row>>;
    
    pub fn filter(&self, conn: &mut PgConnection, conditions: &[Condition])
        -> DatabaseResult<Vec<Row>>;
}
```

**Integration with Narratives**:

- Narratives define table schemas in TOML
- Narratives create/populate tables via `botticelli_database`
- Actors reference table names in `actor.toml` knowledge array
- Actor queries tables at runtime

### 4. Skills System

**Location**: `crates/botticelli_actor/src/skill.rs`

**Trait**:

```rust
#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    
    async fn execute(&self, context: &SkillContext) 
        -> SkillResult<SkillOutput>;
}

pub struct SkillContext {
    pub knowledge: HashMap<String, Vec<Row>>,
    pub config: HashMap<String, String>,
    pub platform: Box<dyn SocialMediaPlatform>,
}
```

**Built-in Skills**:

- `ContentScheduling` - Schedule posts based on time windows
- `RateLimiting` - Enforce posting frequency limits
- `ContentFiltering` - Apply approval/quality checks
- `MediaProcessing` - Resize, optimize media

**Skill Registry**:

```rust
pub struct SkillRegistry {
    skills: HashMap<String, Box<dyn Skill>>,
}

impl SkillRegistry {
    pub fn register(&mut self, skill: Box<dyn Skill>);
    pub fn get(&self, name: &str) -> Option<&dyn Skill>;
    pub fn execute(&self, name: &str, context: &SkillContext) 
        -> SkillResult<SkillOutput>;
}
```

### 5. Configuration

**Location**: `crates/botticelli_actor/src/config.rs`

**TOML Schema** (`actor.toml`):

```toml
[actor]
name = "Post Scheduler"
description = "Schedules and posts content to social media"
knowledge = ["approved_posts_channel_1", "approved_posts_channel_2"]
skills = ["content_scheduling", "social_media_posting"]

[actor.config]
# User-customizable settings with defaults
max_posts_per_day = 10
min_interval_minutes = 60
retry_attempts = 3
timezone = "America/New_York"

[skills.content_scheduling]
enabled = true
schedule_window_start = "09:00"
schedule_window_end = "17:00"
randomize_within_window = true

[skills.social_media_posting]
enabled = true
include_hashtags = true
max_media_size_mb = 10
```

**Rust Types**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
pub struct ActorConfig {
    pub name: String,
    pub description: String,
    pub knowledge: Vec<String>,
    pub skills: Vec<String>,
    
    #[builder(default)]
    #[serde(default)]
    pub config: ActorSettings,
    
    #[builder(default)]
    #[serde(default)]
    pub skill_configs: HashMap<String, SkillConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
pub struct ActorSettings {
    #[builder(default = 10)]
    #[serde(default = "default_max_posts")]
    pub max_posts_per_day: u32,
    
    #[builder(default = 60)]
    #[serde(default = "default_min_interval")]
    pub min_interval_minutes: u32,
    
    // ... other settings with defaults
}
```

## Implementation Plan

### Phase 1: Core Trait System ✅

**Crate**: `crates/botticelli_actor`

1. [x] Create crate skeleton
2. [x] Define `SocialMediaPlatform` trait
3. [x] Define `Skill` trait
4. [x] Create `Content` and related types
5. [x] Add error types using `derive_more`
6. [x] Add comprehensive documentation

**Files**:

- `src/lib.rs` - Module declarations + pub use
- `src/platform.rs` - `SocialMediaPlatform` trait
- `src/skill.rs` - `Skill` trait + registry
- `src/content.rs` - `Content`, `MediaAttachment` types
- `src/error.rs` - Actor error types

**Tests**:

- `tests/platform_trait_test.rs` - Mock platform implementation (7 tests passing)
- `tests/skill_registry_test.rs` - Skill registration/execution (6 tests passing)

### Phase 2: Configuration System ✅

**Crate**: `crates/botticelli_actor`

1. [x] Design TOML schema
2. [x] Create `ActorConfig` with `TypedBuilder`
3. [x] Implement TOML deserialization
4. [x] Add validation logic
5. [x] Create configuration examples

**Files**:

- `src/config.rs` - Configuration types + loading
- `examples/actor.toml` - Example configuration

**Tests**:

- `tests/config_test.rs` - TOML loading, validation, defaults (13 tests passing)

### Phase 3: Knowledge Integration ✅

**Crate**: `crates/botticelli_actor`

1. [x] Create `KnowledgeTable` wrapper
2. [x] Implement query methods
3. [x] Add connection to `botticelli_database`
4. [x] Create knowledge context type (in SkillContext)

**Files**:

- `src/knowledge.rs` - Knowledge table abstraction

**Tests**:

- `tests/knowledge_test.rs` - Table creation and basic operations (3 tests passing)

### Phase 4: Actor Core ✓

**Crate**: `crates/botticelli_actor`

1. [ ] Implement `Actor` struct
2. [ ] Add builder pattern
3. [ ] Create execution loop
4. [ ] Add instrumentation
5. [ ] Implement lifecycle management

**Files**:

- `src/actor.rs` - Main actor implementation
- `src/executor.rs` - Execution orchestration

**Tests**:

- `tests/actor_test.rs` - Actor lifecycle, execution

### Phase 5: Built-in Skills ✓

**Crate**: `crates/botticelli_actor/src/skills/`

1. [ ] Implement `ContentScheduling` skill
2. [ ] Implement `RateLimiting` skill
3. [ ] Implement `ContentFiltering` skill
4. [ ] Add skill configuration
5. [ ] Document each skill

**Files**:

- `src/skills/mod.rs` - Skill module declarations
- `src/skills/scheduling.rs` - Content scheduling
- `src/skills/rate_limiting.rs` - Rate limiting
- `src/skills/filtering.rs` - Content filtering

**Tests**:

- `tests/skills_test.rs` - Each skill independently

### Phase 6: Discord Implementation ✓

**Crate**: `crates/botticelli_discord` (extend existing)

1. [ ] Implement `SocialMediaPlatform` for Discord
2. [ ] Map Discord API to platform trait
3. [ ] Handle Discord-specific constraints
4. [ ] Add Discord configuration

**Files**:

- `src/platform.rs` - Discord platform implementation
- `src/adapter.rs` - API mapping logic

**Tests**:

- `tests/discord_platform_test.rs` - Discord platform impl
- Integration tests with mock Discord API

### Phase 7: Integration & Examples ✓

**Top-level Integration**

1. [ ] Create end-to-end example
2. [ ] Document actor workflows
3. [ ] Add troubleshooting guide
4. [ ] Update main README

**Files**:

- `examples/post_scheduler.rs` - Complete example
- `ACTOR_GUIDE.md` - User guide (after implementation)

**Tests**:

- `tests/integration_test.rs` - Full workflow test

## Design Decisions

### Why Traits Over Concrete Types?

**Problem**: Multiple social media platforms with different APIs

**Solution**: `SocialMediaPlatform` trait allows:

- Platform implementations in separate crates
- Testing with mock platforms
- Runtime platform selection
- No coupling to specific platform

### Why Skills Are Separate?

**Problem**: Actors need reusable capabilities

**Solution**: Skill trait allows:

- Composition over inheritance
- User-defined custom skills
- Independent testing
- Clear separation of concerns

### Why Knowledge Is Table-Based?

**Problem**: Narratives produce structured data

**Solution**: Table references allow:

- Type-safe queries via `botticelli_database`
- Schema validation
- SQL optimization
- Clear data lineage (narrative → table → actor)

### Why Builder Pattern for Actor?

**Problem**: Many configuration options, optional dependencies

**Solution**: Builder pattern provides:

- Named parameters (self-documenting)
- Optional fields with defaults
- Compile-time validation
- Ergonomic API

## Design Decisions

### 1. Skill Discovery

**Question**: How do users discover available skills?

**Decision**: Registry with `.list()` method exposed via CLI

**Implementation**:

```rust
impl SkillRegistry {
    /// List all registered skills with descriptions.
    pub fn list(&self) -> Vec<SkillInfo> {
        self.skills
            .values()
            .map(|skill| SkillInfo {
                name: skill.name().to_string(),
                description: skill.description().to_string(),
            })
            .collect()
    }
}
```

**CLI Command**:

```bash
botticelli actor skills list
# Output:
# content_scheduling - Schedule posts based on time windows
# rate_limiting - Enforce posting frequency limits
# content_filtering - Apply approval/quality checks
# media_processing - Resize and optimize media attachments
```

### 2. Error Handling Strategy

**Question**: How should actor errors propagate?

**Decision**: Discriminate between recoverable and unrecoverable errors

**Error Categories**:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ActorErrorKind {
    // Recoverable errors (retry, skip, continue)
    #[display("Platform temporary failure: {}", _0)]
    PlatformTemporary(String),
    
    #[display("Rate limit exceeded: retry after {}s", _0)]
    RateLimitExceeded(u64),
    
    #[display("Content validation failed: {}", _0)]
    ValidationFailed(String),
    
    #[display("Resource temporarily unavailable: {}", _0)]
    ResourceUnavailable(String),
    
    // Unrecoverable errors (stop execution)
    #[display("Authentication failed: {}", _0)]
    AuthenticationFailed(String),
    
    #[display("Configuration invalid: {}", _0)]
    InvalidConfiguration(String),
    
    #[display("Platform permanently failed: {}", _0)]
    PlatformPermanent(String),
    
    #[display("Database connection lost: {}", _0)]
    DatabaseFailed(String),
}

impl ActorErrorKind {
    /// Check if error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PlatformTemporary(_)
                | Self::RateLimitExceeded(_)
                | Self::ValidationFailed(_)
                | Self::ResourceUnavailable(_)
        )
    }
}
```

**Execution Strategy**:

```rust
pub struct ExecutionConfig {
    /// Stop on unrecoverable errors
    pub stop_on_unrecoverable: bool,
    
    /// Max retries for recoverable errors
    pub max_retries: u32,
    
    /// Collect all errors vs fail fast
    pub continue_on_error: bool,
}

pub struct ExecutionResult {
    pub succeeded: Vec<SkillOutput>,
    pub failed: Vec<(String, ActorError)>,
    pub skipped: Vec<String>,
}
```

### 3. State Persistence

**Question**: Should actors persist their state?

**Decision**: Adapt existing `botticelli_cache` pattern - support both in-memory and disk-based caching

**Architecture**:

```rust
/// Cache strategy for actor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheStrategy {
    /// No caching - always re-query knowledge
    None,
    
    /// In-memory cache with TTL (faster, volatile)
    Memory,
    
    /// Disk-based cache with TTL (persistent, slower)
    Disk,
}

/// Actor state cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
pub struct ActorCacheConfig {
    #[builder(default = CacheStrategy::Memory)]
    #[serde(default = "default_strategy")]
    pub strategy: CacheStrategy,
    
    #[builder(default = 300)]
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
    
    #[builder(default = 1000)]
    #[serde(default = "default_max_size")]
    pub max_entries: usize,
    
    #[builder(default)]
    #[serde(default)]
    pub disk_path: Option<PathBuf>,
}
```

**Integration with `botticelli_cache`**:

- Reuse `CommandCache` for in-memory caching
- Extend with `DiskCache` trait implementation
- Actor state (knowledge queries, skill outputs) cached by key
- Cache keys: `{actor_name}:{knowledge_table}:{query_hash}`

**TOML Configuration**:

```toml
[actor.cache]
strategy = "memory"  # "none", "memory", "disk"
ttl_seconds = 300
max_entries = 1000
disk_path = ".actor_cache"  # Only for disk strategy
```

### 4. Concurrent Execution

**Question**: Can multiple actors run simultaneously?

**Decision**: Multi-threaded with locking (Option B)

**Rationale**:

- Different actors may target different platforms
- Parallelism improves throughput
- Actor-level locking prevents conflicts
- Shared resources (DB, platform APIs) need coordination

**Implementation**:

```rust
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;

pub struct ActorManager {
    actors: Arc<RwLock<HashMap<String, Arc<Actor>>>>,
}

impl ActorManager {
    pub async fn run_all(&self) -> Vec<Result<ExecutionResult, ActorError>> {
        let actors = self.actors.read().unwrap();
        let mut tasks = JoinSet::new();
        
        for (name, actor) in actors.iter() {
            let actor_clone = Arc::clone(actor);
            tasks.spawn(async move {
                actor_clone.execute().await
            });
        }
        
        let mut results = Vec::new();
        while let Some(result) = tasks.join_next().await {
            results.push(result.unwrap());
        }
        results
    }
}
```

**Synchronization Points**:

- Platform API calls (rate limiting via `botticelli_rate_limit`)
- Database access (connection pool via `botticelli_database`)
- Skill execution (actor-level lock)
- Cache updates (cache-level lock)

## Dependencies

**New Dependencies**:

- `async-trait` - Async trait support
- `tokio` - Already in workspace
- `serde` - Already in workspace
- `toml` - Already in workspace

**Internal Dependencies**:

- `botticelli_database` - Knowledge table queries
- `botticelli_error` - Error types
- `botticelli_discord` - Discord platform impl (extends existing)

## Success Criteria

### Minimal Viable Product (MVP)

- [ ] Core traits defined and documented
- [ ] Actor struct with builder pattern
- [ ] Configuration loading from TOML
- [ ] Knowledge table queries working
- [ ] At least 2 built-in skills implemented
- [ ] Discord platform implementation
- [ ] End-to-end example working
- [ ] All tests passing

### Future Enhancements

- Additional platform implementations (Twitter, Bluesky)
- Advanced scheduling algorithms
- Content recommendation engine
- Analytics and reporting
- Web UI for actor management
- Hot-reload of configurations
- Plugin system for third-party skills

## Testing Strategy

### Unit Tests

- Each trait method independently
- Configuration parsing and validation
- Skill execution logic
- Knowledge table queries

### Integration Tests

- Actor + mock platform
- Actor + real database
- Skill composition
- Error scenarios

### API Tests (Feature-Gated)

- Discord platform with real API
- Rate limiting validation
- Content posting verification

**Note**: API tests use `#[cfg_attr(not(feature = "api"), ignore)]`

## Timeline Estimate

**Phase 1-2**: Core traits + config ~ 1 day  
**Phase 3-4**: Knowledge + actor core ~ 2 days  
**Phase 5**: Built-in skills ~ 1 day  
**Phase 6**: Discord implementation ~ 1 day  
**Phase 7**: Integration + docs ~ 1 day

**Total**: ~6 days for MVP

## Related Documentation

- `NARRATIVE_TOML_SPEC.md` - Narrative system producing knowledge tables
- `DISCORD_SCHEMA.md` - Discord database schema
- `CLAUDE.md` - Code style and testing requirements

## Notes

- All public functions must have `#[instrument]`
- Use `derive_more` for all error types
- Builder pattern for all struct construction
- Tests go in `tests/` directory, never `#[cfg(test)]`
- No re-exports between workspace crates
- Feature-gate API tests to conserve rate limits

---

**Next Step**: Begin Phase 1 implementation when approved
