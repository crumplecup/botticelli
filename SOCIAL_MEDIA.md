# Social Media Platform API Integration

This document outlines Boticelli's social media integration strategy. We're implementing **Discord as the canary implementation** to learn from a complex, feature-rich platform before expanding to others.

## Why Discord as the Canary?

Discord is the ideal first platform for several reasons:

1. **Mature Rust ecosystem**: `serenity` has 136K downloads/month and extensive documentation
2. **Complex data model**: If we can handle Discord's complexity (guilds, channels, threads, roles, permissions, voice, interactions), simpler platforms will be easier
3. **Real-time events**: Discord's gateway-based architecture will teach us about event-driven social media bots
4. **Rich features**: Embeds, components (buttons/menus), slash commands, webhooks - comprehensive feature set
5. **Active development**: Discord's API is actively maintained and well-documented
6. **Clear use case**: Narrative-driven Discord bots posting multi-act content to channels

## Implementation Strategy

Each social media platform has fundamentally different APIs, data models, and interaction patterns. Rather than attempting a premature abstraction with traits, we'll:

1. **Build Discord fully** - Complete implementation with all features
2. **Learn patterns** - Identify what's common vs. platform-specific
3. **Extract commonalities** - Only abstract what genuinely generalizes
4. **Implement next platform** - Apply learnings to platform #2
5. **Refine architecture** - Adjust based on real multi-platform experience

This approach avoids the "premature abstraction" trap where we design a trait interface that doesn't actually fit the real platforms.

## Module Organization

All social media platform code lives under `src/social/`:

```
src/
├── social/
│   ├── mod.rs              # Re-exports all platform modules
│   ├── discord/
│   │   ├── mod.rs          # Discord module exports
│   │   ├── error.rs        # Discord-specific errors
│   │   ├── models/         # Diesel models
│   │   │   ├── mod.rs
│   │   │   ├── guild.rs
│   │   │   ├── channel.rs
│   │   │   ├── user.rs
│   │   │   ├── member.rs
│   │   │   └── role.rs
│   │   ├── repository.rs   # Database operations
│   │   ├── client.rs       # Serenity client setup
│   │   ├── handler.rs      # Event handler
│   │   ├── poster.rs       # Narrative poster
│   │   └── commands/       # Slash commands
│   │       ├── mod.rs
│   │       ├── ping.rs
│   │       ├── stats.rs
│   │       └── narrative.rs
│   ├── telegram/           # Future: Telegram implementation
│   ├── reddit/             # Future: Reddit implementation
│   └── ...
```

In `src/lib.rs`:

```rust
#[cfg(feature = "discord")]
mod social;

#[cfg(feature = "discord")]
pub use social::discord::{
    DiscordRepository, BoticelliBot, DiscordError, DiscordErrorKind,
    // ... other exports
};
```

## Development Progress

### ✅ Completed Steps

#### Step 1.1: Module Structure (Completed)

Created the social media module hierarchy:

**Files created:**
- `src/social/mod.rs` - Social media integration module with feature-gated platform submodules
- `src/social/discord/mod.rs` - Discord module structure with documentation and placeholder exports

**Changes made:**
- Added `mod social` to `src/lib.rs` with `#[cfg(feature = "discord")]`
- Added Discord exports section (commented out until implementations exist)
- Added `serenity = "0.12"` dependency to `Cargo.toml` as optional
- Created `discord` feature flag that depends on both `serenity` and `database`

**Verification:**
- `cargo check --features discord` passes ✓
- Module structure follows CLAUDE.md patterns ✓

#### Step 1.2: Database Schema Migrations (Completed)

Created Diesel migrations for 6 core Discord tables:

**Migrations created:**
1. `2025-11-15-233432-0000_create_discord_guilds` - Guild (server) information
2. `2025-11-15-233458-0000_create_discord_users` - Global user profiles
3. `2025-11-15-233523-0000_create_discord_channels` - All channel types with ENUM for channel_type
4. `2025-11-15-233550-0000_create_discord_guild_members` - Guild-specific member data
5. `2025-11-15-233608-0000_create_discord_roles` - Role hierarchies
6. `2025-11-15-233628-0000_create_discord_member_roles` - Member-role junction table

**Schema features:**
- `discord_guilds` has 44 columns (requires `64-column-tables` diesel feature)
- `discord_channel_type` ENUM with 13 channel types
- Proper foreign key relationships and CASCADE deletes
- Indexes for common query patterns
- JSONB columns for flexible Discord-specific data

**Changes made:**
- Added `64-column-tables` feature to diesel dependency in `Cargo.toml`
- All migrations include comprehensive indexes
- Schema auto-generated in `src/database/schema.rs`

**Verification:**
- `diesel migration run` completed successfully ✓
- All 6 tables created in PostgreSQL ✓
- Foreign key constraints validated ✓
- Indexes created ✓

#### Step 1.3: Diesel Models (Completed)

Implemented Diesel models for all core Discord entities:

**Files created:**
- `src/social/discord/models/mod.rs` - Module structure and re-exports
- `src/social/discord/models/guild.rs` - GuildRow, NewGuild (44 fields!)
- `src/social/discord/models/user.rs` - UserRow, NewUser
- `src/social/discord/models/channel.rs` - ChannelRow, NewChannel, ChannelType enum
- `src/social/discord/models/member.rs` - GuildMemberRow, NewGuildMember (composite PK)
- `src/social/discord/models/role.rs` - RoleRow, NewRole

**Key implementations:**
- **ChannelType enum** with manual ToSql/FromSql for PostgreSQL ENUM mapping
- **Composite primary keys** for GuildMemberRow (guild_id, user_id)
- **Associations** using `#[diesel(belongs_to(...))]` for foreign keys
- **JSONB support** for tags, available_tags, and other flexible fields
- **Array support** for guild features (Vec<Option<String>>)

**Patterns followed:**
- Row types derive: `Debug, Clone, Queryable, Identifiable, Selectable`
- New types derive: `Debug, Clone, Insertable`
- Associations for foreign key relationships
- Proper field ordering matching database schema
- NaiveDateTime for timestamp fields

**Changes made:**
- Made `schema` module `pub(crate)` in `src/database/mod.rs` for internal access
- Exported Discord types from `src/social/discord/mod.rs`
- Exported Discord types from `src/lib.rs` at crate level
- Implemented custom ToSql/FromSql for ChannelType PostgreSQL ENUM

**Verification:**
- All Discord models compile without errors ✓
- Custom ENUM serialization/deserialization works ✓
- Schema references resolve correctly ✓
- Follows CLAUDE.md patterns (Row/New types, derives, associations) ✓

#### Step 1.4: DiscordRepository (Completed)

Implemented comprehensive repository for Discord data operations:

**File created:**
- `src/social/discord/repository.rs` - Complete repository with CRUD operations for all entities

**Operations implemented:**
- **Guild operations**: store_guild, get_guild, list_active_guilds, mark_guild_left
- **User operations**: store_user, get_user
- **Channel operations**: store_channel, get_channel, list_guild_channels
- **Member operations**: store_guild_member, get_guild_member, list_guild_members, mark_member_left
- **Role operations**: store_role, get_role, list_guild_roles, assign_role, remove_role

**Key patterns:**
- Arc<Mutex<PgConnection>> for async safety (matches PostgresNarrativeRepository)
- Upsert patterns using `ON CONFLICT DO UPDATE` for idempotency
- Soft deletes with timestamp tracking (left_at)
- Proper error propagation with DatabaseError::from
- Optional() pattern for nullable queries
- Filter chains for complex queries

**Changes made:**
- Exported DiscordRepository and DiscordResult from `src/social/discord/mod.rs`
- Exported DiscordRepository and DiscordResult from `src/lib.rs` at crate level
- Cleaned up unused imports (DatabaseErrorKind, async_trait)

**Verification:**
- `cargo check --features discord` shows no Discord-specific warnings or errors ✓
- All CRUD operations follow Diesel best practices ✓
- Repository pattern matches PostgresNarrativeRepository ✓
- Connection pooling pattern established ✓

#### Step 1.5: Discord Error Handling (Completed)

Implemented Discord-specific error handling following CLAUDE.md patterns:

**File created:**
- `src/social/discord/error.rs` - Discord error types with location tracking

**Error types implemented:**
- **DiscordErrorKind** - 13 error variants covering all Discord operation types:
  - SerenityError - Serenity API errors (HTTP, gateway, rate limits)
  - DatabaseError - Database operation failures
  - GuildNotFound, ChannelNotFound, UserNotFound, RoleNotFound - Entity not found errors
  - InsufficientPermissions - Permission errors
  - InvalidId - Snowflake ID validation errors
  - ConnectionFailed - Gateway connection errors
  - InvalidToken - Bot token validation errors
  - MessageSendFailed - Message posting errors
  - InteractionFailed - Slash command/button errors
  - ConfigurationError - Missing env vars or invalid settings

- **DiscordError** - Wrapper struct with location tracking (line, file fields)
- **DiscordResult<T>** - Type alias for Result<T, DiscordError>

**Key patterns:**
- #[track_caller] for automatic location tracking in error constructors
- Display impl with descriptive messages for each variant
- From<serenity::Error> for automatic conversion from Serenity errors
- Follows CLAUDE.md error handling patterns exactly

**Integration:**
- Added Discord variant to BoticelliErrorKind with #[cfg(feature = "discord")]
- Added Display match arm for Discord errors
- Exported DiscordError, DiscordErrorKind, DiscordErrorResult from discord module
- Exported at crate level from src/lib.rs

**Verification:**
- `cargo check --features discord` shows no Discord-specific errors or warnings ✓
- Error types follow CLAUDE.md patterns (enum + wrapper struct + location tracking) ✓
- Integrated into crate-level error hierarchy ✓
- From implementations for external errors (serenity::Error) ✓

#### Step 2.1: Serenity Event Handler (Completed)

Implemented comprehensive event handler that captures Discord events and persists data:

**File created:**
- `src/social/discord/handler.rs` - Event handler implementing Serenity's EventHandler trait

**BoticelliHandler implementation:**
- **Gateway intents**: GUILDS | GUILD_MEMBERS | GUILD_MESSAGES | MESSAGE_CONTENT
- **Helper methods**:
  - to_db_id() - Convert Discord snowflake IDs (u64) to database IDs (i64)
  - store_guild() - Persist Guild with all 44 fields populated
  - store_channel() - Persist channels (guild and DM)
  - store_member() - Persist members (stores user first, then guild member)
  - store_role() - Persist roles with permissions
  - map_channel_type() - Convert Serenity ChannelType to our ChannelType enum

**EventHandler trait methods:**
- ready() - Log bot connection and guild count
- guild_create() - Store full guild data (channels, roles, members)
- guild_delete() - Soft delete guild with mark_guild_left()
- channel_create() - Store newly created channels
- guild_member_addition() - Store new members joining
- guild_member_removal() - Soft delete members with mark_member_left()
- guild_role_create() - Store newly created roles

**Key patterns:**
- Comprehensive logging with tracing (info, debug, error, warn levels)
- Error handling with structured logging (guild_id, user_id, error fields)
- Upsert approach via repository (all store methods handle duplicates)
- Soft deletes for guilds and members (preserves historical data)
- Full field population from Serenity models to database models

**Verification:**
- All event handler methods compile without errors ✓
- Proper async/await patterns throughout ✓
- Tracing integration for observability ✓
- Repository integration working ✓

#### Step 2.2: Bot Client Setup (Completed)

Implemented bot client that manages Serenity connection and lifecycle:

**File created:**
- `src/social/discord/client.rs` - Bot client with Serenity integration

**BoticelliBot struct:**
- Wraps Serenity Client with database repository
- Manages connection lifecycle (new, start)
- Provides repository access for external queries

**Key methods:**
- new(token, conn) - Initialize bot with token and database connection
  - Creates Arc<DiscordRepository> for shared access
  - Builds BoticelliHandler with repository
  - Configures gateway intents
  - Builds Serenity Client with handler
  - Returns comprehensive errors via DiscordError

- start() - Start the bot (blocks until shutdown)
  - Calls client.start().await
  - Handles errors with proper wrapping

**Error handling:**
- ConnectionFailed errors for client build/start failures
- Uses #[track_caller] via DiscordError::new()
- Detailed error messages with context

**Documentation:**
- Full doc comments with usage example
- Integration example with establish_connection()

**Verification:**
- Client compiles without errors ✓
- Proper async patterns ✓
- Error propagation working ✓
- Repository integration ✓

#### Step 2.3: CLI Integration (Completed)

Added Discord bot commands to the Boticelli CLI:

**Files modified:**
- `src/main.rs` - Added Discord subcommand and start_discord_bot function

**CLI structure:**
```bash
boticelli discord start [--token TOKEN]
```

**DiscordCommands enum:**
- Start - Starts the Discord bot with optional token argument

**start_discord_bot function:**
- Token resolution: CLI --token > DISCORD_TOKEN env var
- Establishes database connection via establish_connection()
- Creates BoticelliBot with token and connection
- User-friendly output with emojis:
  - 🤖 Starting message
  - ✓ Initialization success
  - 🚀 Connection status
  - Ctrl+C shutdown hint
- Starts bot (blocks until shutdown)

**Integration:**
- Added #[cfg(feature = "discord")] guards throughout
- Match arm in main() dispatches to start_discord_bot
- Proper error propagation to CLI

**Verification:**
- CLI compiles with discord feature ✓
- No Discord-specific errors ✓
- Follows existing CLI patterns (Run, List, Show commands) ✓

## Next Steps

**Phase 2 Complete! Ready for testing.**

**Next: Testing Phase**

1. **Test bot in Discord server**: Create test server, add bot, verify:
   - Guild data is stored on startup
   - Channels are stored
   - Members are stored
   - Role creations are captured
   - Member join/leave events work
   - Soft deletes function properly
2. **Phase 3**: Advanced bot features (slash commands, narrative posting)

## Reference: Platform Coverage

This section documents Rust crates for future platform implementations.

### Tier 1: Essential Platforms

#### X (formerly Twitter)

**Best Crate:** `xv2api` (v0.1.1)

- **Description:** X/Twitter V2 API Library
- **Features:** OAuth, rate-limiting, authentication, OAuth2, token-management, cache, posting, bearer-token
- **Status:** Active development
- **Note:** `egg-mode` (0.16.1, 1.6K downloads/month) is abandoned but was historically popular

**Recommendation:** Use `xv2api` for new projects. The V2 API support is essential as Twitter/X has deprecated older API versions.

#### Discord

**Best Crate:** `serenity` (v0.12.4, 136K downloads)

- **Description:** Discord API client library
- **Tags:** discord-bot, discord-api, api
- **Alternatives:**
  - `twilight-model` (v0.17.0, 51K downloads) - Models for the Twilight ecosystem
  - `twilight-http` (v0.17.0, 46K downloads) - REST API client for Twilight ecosystem
  - `songbird` (v0.5.0, 4.9K downloads) - Voice API specialization

**Recommendation:** Use `serenity` for general Discord bots and applications. Consider the Twilight ecosystem (`twilight-model`, `twilight-http`) if you need a more modular approach or better performance at scale.

#### Telegram

**Best Crate:** `teloxide` (v0.17.0, 35K downloads)

- **Description:** An elegant Telegram bots framework for Rust
- **Features:** Comprehensive framework for building Telegram bots
- **Alternatives:**
  - `grammers-client` (v0.8.1, 3.3K downloads) - High-level client with MTProto support
  - `frankenstein` (v0.45.0, 1.4K downloads) - Straightforward bot API client
  - `conogram` (v0.2.19, 1.2K downloads) - Async wrapper for Bot API

**Recommendation:** Use `teloxide` for its elegant API and strong community support. Use `grammers-client` if you need full MTProto protocol access beyond bot functionality.

#### Reddit

**Best Crate:** `roux` (v2.2.15, 250 downloads)

- **Description:** (a)synchronous Reddit API wrapper
- **Features:** Both sync and async capabilities
- **Alternatives:**
  - `rraw` (v1.2.1) - Async Reddit API wrapper using Tokio
  - `orca` (v0.7.0) - Reddit API client

**Recommendation:** Use `roux` for its dual sync/async support and maturity. Use `rraw` if you're building a purely async application with Tokio.

#### YouTube

**Best Crate:** `google-youtube3` (v6.0.0+20240626, 1.4K downloads)

- **Description:** A complete library to interact with YouTube (protocol v3)
- **Features:** Full YouTube Data API v3 support
- **Alternatives:**
  - `rustypipe` (v0.11.4, 1.3K downloads) - Client for public YouTube/YouTube Music API (Innertube), no API key needed
  - `invidious` (v0.7.8, 350 downloads) - Invidious API wrapper, no tokens required

**Recommendation:** Use `google-youtube3` for official API access with authentication. Use `rustypipe` or `invidious` for anonymous public data access without API keys.

### Tier 2: Decentralized & Alternative Platforms

#### Mastodon

**Best Crate:** `megalodon` (v1.0.3, 800 downloads)

- **Description:** Mastodon and Pleroma API client library for Rust
- **Features:** REST, WebSocket, streaming support
- **Alternatives:**
  - `mastodon-async` (v1.3.2) - Async wrapper around Mastodon API
  - `elefren` (v0.22.0, 100 downloads) - Mastodon API wrapper

**Recommendation:** Use `megalodon` for its comprehensive feature set including WebSocket streaming and Pleroma compatibility. Use `mastodon-async` if you prefer a simpler async-focused API.

#### Bluesky

**Best Crate:** `bsky-sdk` (v0.1.22, 950 downloads)

- **Description:** ATrium-based SDK for Bluesky
- **Features:** High-level SDK built on the Atrium library ecosystem
- **Alternatives:**
  - `atrium-api` (v0.25.6, 3.1K downloads) - Core API library for AT Protocol
  - `jacquard` (v0.9.0, 1.9K downloads) - Powerful AT Protocol client library
  - `aerostream` (v0.16.5, 3.2K downloads) - EventStream-based client

**Recommendation:** Use `bsky-sdk` for the most user-friendly experience. Use `atrium-api` directly if you need lower-level protocol control or are building infrastructure. Use `aerostream` for real-time event streaming.

### Tier 3: Meta Platforms

#### Facebook

**Best Crate:** `facebook_api_rs` (v0.1.2, 380 downloads)

- **Description:** Client library for Facebook Graph API v23.0
- **Features:** Full native and WebAssembly support, OAuth, covers both Facebook and Instagram
- **Alternatives:**
  - `fb_poster` (v0.1.9, 500 downloads) - Unofficial client for Facebook post uploads
  - `serde-metaform` (v1.0.1, 400 downloads) - Form encoder for Meta batch requests

**Recommendation:** Use `facebook_api_rs` for its comprehensive Graph API coverage and WASM support. The single crate handles both Facebook and Instagram APIs.

#### Instagram

**Best Crate:** `facebook_api_rs` (v0.1.2, 380 downloads)

- **Description:** Client library for Facebook Graph API v23.0 with Instagram support
- **Features:** OAuth, graph-api, WASM support
- **Alternatives:**
  - `instagram-graph-api` (v0.1.1, 1 download) - Instagram-specific Graph API
  - `rocketapi` (v0.1.1, 180 downloads) - Unofficial SDK for Instagram private API

**Recommendation:** Use `facebook_api_rs` since Instagram is part of the Meta Graph API ecosystem. Only use `rocketapi` if you need unofficial private API access (use with caution and review ToS).

#### Threads (Meta)

**Best Crate:** `rusty_meta_threads` (v0.8.1)

- **Description:** Community Rust SDK for integrating with Meta Threads API
- **Tags:** meta-thread, thread-api, sdk
- **Note:** Only dedicated crate for Threads API

**Recommendation:** Use `rusty_meta_threads` as the sole option. Monitor Meta's official Threads API announcements as this is a newer platform.

### Tier 4: Additional Platforms

#### TikTok

**Best Crate:** `tiktok-business` (v0.5.0) for business API, `tiktoklive` (v0.0.19, 1.0K downloads) for live streaming

- **Business API:** OAuth support, official business functionality
- **Live Streaming:** Real-time events (comments, gifts) without credentials
- **Alternatives:**
  - `tiktok_rust` (v0.0.13, 480 downloads) - Post content and retrieve creator info
  - `tiktokapi-v2` (v0.5.1) - TikTok API v2 library

**Recommendation:** Use `tiktok-business` for official business/creator API access. Use `tiktoklive` for monitoring live streams without authentication.

#### LinkedIn

**Best Crate:** `linkedin-api` (v0.5.0, 280 downloads)

- **Description:** Rust wrapper for the LinkedIn API
- **Features:** Async support, Voyager platform bindings
- **Alternatives:**
  - `proxycurl-linkedin-rs` (v0.1.0) - Proxycurl API client (third-party service)
  - `oauth2-linkedin` (v0.2.0) - OAuth 2.0 authentication only

**Recommendation:** Use `linkedin-api` for direct API access. Consider `proxycurl-linkedin-rs` if you need public profile scraping without official API limits (paid service).

#### Pinterest

**Best Crate:** `pinterest-api` (v0.2.0, 230 downloads)

- **Description:** Pinterest API library
- **Features:** OAuth, Pinterest client, API client
- **Alternatives:**
  - `oauth2-pinterest` (v0.2.0) - OAuth 2.0 integration only
  - `pinterest-login` (v0.2.0-alpha.1) - Browser automation for login

**Recommendation:** Use `pinterest-api` for official API access. The ecosystem is less mature than other platforms.

## Discord Implementation Plan

See [DISCORD_SCHEMA.md](DISCORD_SCHEMA.md) for the complete database schema design.

### Phase 1: Foundation (Current)

**Goal**: Basic Discord bot infrastructure with database persistence

#### Step 1.1: Add Dependencies

```toml
[dependencies]
serenity = { version = "0.12", default-features = false, features = ["client", "gateway", "rustls_backend", "model", "cache"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[features]
discord = ["dep:serenity"]
```

#### Step 1.2: Database Schema Migrations

Create Diesel migrations for Discord core entities:

1. `discord_guilds` - Track guilds the bot is in
2. `discord_channels` - All channel types (text, voice, threads, forums)
3. `discord_users` - Global user profiles
4. `discord_guild_members` - Guild-specific member data
5. `discord_roles` - Role hierarchies
6. `discord_member_roles` - Role assignments

**Files to create**:
- `migrations/XXXXXX_create_discord_guilds/up.sql`
- `migrations/XXXXXX_create_discord_guilds/down.sql`
- (Repeat for each table)

#### Step 1.3: Diesel Models

Create Rust models matching the schema in `src/social/discord/models/`:

- `guild.rs` - `Guild`, `NewGuild`, `GuildRow`
- `channel.rs` - `Channel`, `NewChannel`, `ChannelRow`
- `user.rs` - `User`, `NewUser`, `UserRow`
- `member.rs` - `Member`, `NewMember`, `MemberRow`
- `role.rs` - `Role`, `NewRole`, `RoleRow`

Follow Boticelli patterns: separate `Row` types for database, `New` types for inserts, main types for business logic.

#### Step 1.4: Repository Layer

Create `src/social/discord/repository.rs` following the `NarrativeRepository` pattern:

```rust
pub struct DiscordRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DiscordRepository {
    pub async fn store_guild(&self, guild: &serenity::model::guild::Guild) -> Result<()>;
    pub async fn get_guild(&self, guild_id: GuildId) -> Result<Option<Guild>>;
    pub async fn store_channel(&self, channel: &serenity::model::channel::GuildChannel) -> Result<()>;
    pub async fn list_guild_channels(&self, guild_id: GuildId) -> Result<Vec<Channel>>;
    // ... etc
}
```

#### Step 1.5: Error Handling

Create `src/social/discord/error.rs` following CLAUDE.md patterns:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DiscordErrorKind {
    SerenityError(String),
    DatabaseError(String),
    GuildNotFound(i64),
    ChannelNotFound(i64),
    // ...
}

pub struct DiscordError {
    pub kind: DiscordErrorKind,
    pub line: u32,
    pub file: &'static str,
}
```

Add `Discord(DiscordError)` variant to `BoticelliErrorKind`.

### Phase 2: Basic Bot Functionality

**Goal**: Bot can connect, respond to events, and log to database

#### Step 2.1: Event Handler

Create `src/social/discord/handler.rs`:

```rust
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;

pub struct BoticelliHandler {
    repository: Arc<DiscordRepository>,
}

#[async_trait]
impl EventHandler for BoticelliHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Bot connected as {}", ready.user.name);
        // Store guilds in database
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Store message in database
        // Process commands
    }

    async fn guild_create(&self, ctx: Context, guild: Guild) {
        // Store guild in database
    }

    // ... other events
}
```

#### Step 2.2: Client Setup

Create `src/social/discord/client.rs`:

```rust
pub struct BoticelliBot {
    client: serenity::Client,
    repository: Arc<DiscordRepository>,
}

impl BoticelliBot {
    pub async fn new(token: String, database_url: String) -> Result<Self> {
        let repository = Arc::new(DiscordRepository::new(&database_url)?);
        let handler = BoticelliHandler::new(repository.clone());

        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let client = Client::builder(&token, intents)
            .event_handler(handler)
            .await?;

        Ok(Self { client, repository })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
```

#### Step 2.3: CLI Integration

Update `src/bin/boticelli.rs` to include Discord bot command:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands

    #[cfg(feature = "discord")]
    Discord {
        #[command(subcommand)]
        command: DiscordCommands,
    },
}

#[cfg(feature = "discord")]
#[derive(Subcommand)]
enum DiscordCommands {
    /// Start the Discord bot
    Start {
        /// Discord bot token
        #[arg(env = "DISCORD_TOKEN")]
        token: String,
    },
}
```

### Phase 3: Message Management

**Goal**: Store and query messages, attachments, embeds

#### Step 3.1: Message Schema

Add migrations and models for:
- `discord_messages`
- `discord_message_mentions`
- `discord_message_attachments`
- `discord_message_embeds`
- `discord_embed_fields`
- `discord_message_reactions`

#### Step 3.2: Message Storage

Extend `DiscordRepository` with message methods:

```rust
impl DiscordRepository {
    pub async fn store_message(&self, msg: &Message) -> Result<i64>;
    pub async fn get_channel_messages(&self, channel_id: i64, limit: i32) -> Result<Vec<Message>>;
    pub async fn search_messages(&self, guild_id: i64, query: &str) -> Result<Vec<Message>>;
}
```

#### Step 3.3: Event Handler Updates

Update `BoticelliHandler` to store messages:

```rust
async fn message(&self, ctx: Context, msg: Message) {
    if let Err(e) = self.repository.store_message(&msg).await {
        error!("Failed to store message: {}", e);
    }
}
```

### Phase 4: Slash Commands & Interactions

**Goal**: Register and respond to slash commands

#### Step 4.1: Command Schema

Add migrations for:
- `discord_application_commands`
- `discord_command_executions`

#### Step 4.2: Command Framework

Create `src/social/discord/commands/`:

```rust
// src/social/discord/commands/mod.rs
pub mod ping;
pub mod stats;
pub mod narrative;

pub use ping::PingCommand;
pub use stats::StatsCommand;
pub use narrative::NarrativeCommand;
```

#### Step 4.3: Command Registration

```rust
// src/social/discord/commands/ping.rs
pub struct PingCommand;

impl PingCommand {
    pub fn register() -> CreateCommand {
        CreateCommand::new("ping")
            .description("Check bot latency")
    }

    pub async fn execute(ctx: &Context, interaction: &CommandInteraction) -> Result<()> {
        interaction.create_response(ctx, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Pong!")
        )).await?;
        Ok(())
    }
}
```

#### Step 4.4: Interaction Handler

```rust
async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
    if let Interaction::Command(cmd) = interaction {
        let result = match cmd.data.name.as_str() {
            "ping" => PingCommand::execute(&ctx, &cmd).await,
            "stats" => StatsCommand::execute(&ctx, &cmd).await,
            _ => Ok(()),
        };

        if let Err(e) = result {
            error!("Command execution failed: {}", e);
        }

        // Store execution in database
        self.repository.store_command_execution(&cmd, result.is_ok()).await;
    }
}
```

### Phase 5: Narrative Integration

**Goal**: Post narrative acts to Discord channels

#### Step 5.1: Narrative Post Schema

Migration for `discord_narrative_posts` (already in DISCORD_SCHEMA.md)

#### Step 5.2: Discord Poster

Create `src/social/discord/poster.rs`:

```rust
pub struct DiscordNarrativePoster {
    repository: Arc<DiscordRepository>,
}

impl DiscordNarrativePoster {
    pub async fn post_act(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        channel_id: ChannelId,
        execution_id: i32,
        act_number: i32,
        content: &str,
    ) -> Result<MessageId> {
        let msg = channel_id.send_message(ctx, CreateMessage::new()
            .content(content)
        ).await?;

        self.repository.store_narrative_post(
            execution_id,
            act_number,
            guild_id.0 as i64,
            channel_id.0 as i64,
            msg.id.0 as i64,
        ).await?;

        Ok(msg.id)
    }
}
```

#### Step 5.3: Narrative Command

```rust
// /narrative execute <narrative_name> <channel>
pub struct NarrativeCommand;

impl NarrativeCommand {
    pub async fn execute(
        ctx: &Context,
        interaction: &CommandInteraction,
        narrative_repo: &NarrativeRepository,
        discord_poster: &DiscordNarrativePoster,
    ) -> Result<()> {
        // Load narrative
        // Execute with driver
        // Post each act to channel
        // Track in discord_narrative_posts
    }
}
```

### Phase 6: Advanced Features

**Goal**: Components, embeds, scheduled posts

#### Step 6.1: Component Support

Add migrations and handlers for buttons, select menus

#### Step 6.2: Embed Builder

```rust
pub struct DiscordEmbedBuilder {
    title: Option<String>,
    description: Option<String>,
    fields: Vec<(String, String, bool)>,
    color: Option<u32>,
}
```

#### Step 6.3: Scheduled Posts

```rust
pub struct DiscordScheduler {
    repository: Arc<DiscordRepository>,
}

impl DiscordScheduler {
    pub async fn schedule_post(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        content: String,
        scheduled_for: DateTime<Utc>,
    ) -> Result<i32>;

    pub async fn process_pending_posts(&self, ctx: &Context) -> Result<()>;
}
```

### Testing Strategy

Each phase should include:

1. **Unit tests** for models and conversions
2. **Integration tests** with test database
3. **Mock Serenity client** for handler testing
4. **Manual testing** in a test Discord server

Test file structure:
- `tests/social_discord_repository_test.rs`
- `tests/social_discord_handler_test.rs`
- `tests/social_discord_commands_test.rs`
- `tests/social_discord_narrative_integration_test.rs`

## Platform Priorities for Future Implementation

Once Discord is complete, consider these platforms next:

### Priority 1: Text-Based Platforms (Similar to Discord)

1. **Telegram** (`teloxide`) - Bot-based, similar patterns to Discord
2. **Reddit** (`roux`) - Forum-style, simpler than Discord
3. **Mastodon** (`megalodon`) - Decentralized, well-documented API

### Priority 2: Content Platforms

4. **YouTube** (`google-youtube3`) - Video platform, different model
5. **Bluesky** (`bsky-sdk`) - Microblogging, growing platform

### Priority 3: Complex Auth Platforms

6. **X/Twitter** (`xv2api`) - Complex auth, rate limits
7. **Facebook/Instagram** (`facebook_api_rs`) - Graph API, strict policies
8. **LinkedIn** (`linkedin-api`) - Professional network
9. **TikTok** (`tiktok-business`) - Video platform, limited API
10. **Threads** (`rusty_meta_threads`) - Newer platform
11. **Pinterest** (`pinterest-api`) - Image-focused

## Lessons to Extract from Discord

After completing Discord implementation, document:

1. **Common patterns**: What generalizes across platforms?
2. **Database design**: Which tables/patterns are reusable?
3. **Event handling**: How do different platforms handle real-time events?
4. **Authentication**: Different auth flows and token management
5. **Rate limiting**: How to implement platform-agnostic rate limiting
6. **Error handling**: Platform-specific vs. generic errors
7. **Testing strategies**: What worked for mocking/testing?
8. **CLI patterns**: How to expose platform functionality

These learnings will inform whether/how to create shared abstractions for future platforms.
