//! Guild (Discord server) models.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for discord_guilds table.
///
/// Represents a Discord guild (server) with all metadata, settings, and bot-specific state.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, derive_getters::Getters)]
#[diesel(table_name = botticelli_database::schema::discord_guilds)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildRow {
    /// Guild ID
    id: i64,
    /// Guild name
    name: String,
    /// Guild icon hash
    icon: Option<String>,
    /// Guild banner hash
    banner: Option<String>,
    /// Guild invite splash hash
    splash: Option<String>,
    /// Guild owner user ID
    owner_id: i64,

    // Guild features
    features: Option<Vec<Option<String>>>,
    description: Option<String>,
    vanity_url_code: Option<String>,

    // Member counts
    member_count: Option<i32>,
    approximate_member_count: Option<i32>,
    approximate_presence_count: Option<i32>,

    // Guild settings
    afk_channel_id: Option<i64>,
    afk_timeout: Option<i32>,
    system_channel_id: Option<i64>,
    rules_channel_id: Option<i64>,
    public_updates_channel_id: Option<i64>,

    // Verification and content filtering
    verification_level: Option<i16>,
    explicit_content_filter: Option<i16>,
    mfa_level: Option<i16>,

    // Premium features
    premium_tier: Option<i16>,
    premium_subscription_count: Option<i32>,

    // Server boost progress
    max_presences: Option<i32>,
    max_members: Option<i32>,
    max_video_channel_users: Option<i32>,

    // Status flags
    large: Option<bool>,
    unavailable: Option<bool>,

    // Timestamps
    joined_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    left_at: Option<NaiveDateTime>,

    // Bot-specific metadata
    bot_permissions: Option<i64>,
    bot_active: Option<bool>,
}

/// Insertable struct for discord_guilds table.
///
/// Used to create new guild records in the database.
#[allow(missing_docs)]
#[derive(
    Debug, Clone, Insertable, derive_getters::Getters, derive_builder::Builder, elicitation::Elicit,
)]
#[diesel(table_name = botticelli_database::schema::discord_guilds)]
#[builder(setter(into))]
pub struct NewGuild {
    /// Discord guild snowflake ID
    id: i64,
    /// Guild name
    name: String,
    /// Icon hash for guild avatar
    #[builder(default)]
    icon: Option<String>,
    /// Banner hash for guild banner image
    #[builder(default)]
    banner: Option<String>,
    /// Splash hash for invite splash image
    #[builder(default)]
    splash: Option<String>,
    /// User ID of guild owner
    owner_id: i64,

    /// Discord guild features enabled
    #[builder(default)]
    features: Option<Vec<Option<String>>>,
    /// Guild description text
    #[builder(default)]
    description: Option<String>,
    /// Vanity URL code if enabled
    #[builder(default)]
    vanity_url_code: Option<String>,

    /// Total member count
    #[builder(default)]
    member_count: Option<i32>,
    /// Approximate member count
    #[builder(default)]
    approximate_member_count: Option<i32>,
    /// Approximate presence count (online members)
    #[builder(default)]
    approximate_presence_count: Option<i32>,

    /// AFK voice channel ID
    #[builder(default)]
    afk_channel_id: Option<i64>,
    /// AFK timeout in seconds
    #[builder(default)]
    afk_timeout: Option<i32>,
    /// System messages channel ID
    #[builder(default)]
    system_channel_id: Option<i64>,
    /// Rules channel ID for community guilds
    #[builder(default)]
    rules_channel_id: Option<i64>,
    /// Public updates channel ID for community guilds
    #[builder(default)]
    public_updates_channel_id: Option<i64>,

    /// Verification level required for members
    #[builder(default)]
    verification_level: Option<i16>,
    /// Explicit content filter level
    #[builder(default)]
    explicit_content_filter: Option<i16>,
    /// MFA level required for moderation actions
    #[builder(default)]
    mfa_level: Option<i16>,

    /// Server boost premium tier (0-3)
    #[builder(default)]
    premium_tier: Option<i16>,
    /// Number of server boosts
    #[builder(default)]
    premium_subscription_count: Option<i32>,

    /// Maximum number of presences (null for large guilds)
    #[builder(default)]
    max_presences: Option<i32>,
    /// Maximum number of members
    #[builder(default)]
    max_members: Option<i32>,
    /// Maximum users in a video channel
    #[builder(default)]
    max_video_channel_users: Option<i32>,

    /// Whether guild is considered large (>250 members)
    #[builder(default)]
    large: Option<bool>,
    /// Whether guild is unavailable due to outage
    #[builder(default)]
    unavailable: Option<bool>,

    /// Timestamp when bot joined guild
    #[builder(default)]
    joined_at: Option<NaiveDateTime>,
    /// Timestamp when bot left guild
    #[builder(default)]
    left_at: Option<NaiveDateTime>,

    /// Bot's permission bitfield in this guild
    #[builder(default)]
    bot_permissions: Option<i64>,
    /// Whether bot is currently active in guild
    #[builder(default)]
    bot_active: Option<bool>,
}
