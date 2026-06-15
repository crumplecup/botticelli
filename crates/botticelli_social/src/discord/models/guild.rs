//! Guild (Discord server) models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stored guild data.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct GuildRow {
    /// Guild ID
    pub id: i64,
    /// Guild name
    pub name: String,
    /// Guild icon hash
    pub icon: Option<String>,
    /// Guild banner hash
    pub banner: Option<String>,
    /// Guild invite splash hash
    pub splash: Option<String>,
    /// Guild owner user ID
    pub owner_id: i64,

    features: Option<Vec<String>>,
    description: Option<String>,
    vanity_url_code: Option<String>,

    member_count: Option<i32>,
    approximate_member_count: Option<i32>,
    approximate_presence_count: Option<i32>,

    afk_channel_id: Option<i64>,
    afk_timeout: Option<i32>,
    system_channel_id: Option<i64>,
    rules_channel_id: Option<i64>,
    public_updates_channel_id: Option<i64>,

    verification_level: Option<i16>,
    explicit_content_filter: Option<i16>,
    mfa_level: Option<i16>,

    premium_tier: Option<i16>,
    premium_subscription_count: Option<i32>,

    max_presences: Option<i32>,
    max_members: Option<i32>,
    max_video_channel_users: Option<i32>,

    large: Option<bool>,
    unavailable: Option<bool>,

    joined_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) left_at: Option<DateTime<Utc>>,

    bot_permissions: Option<i64>,
    pub(crate) bot_active: Option<bool>,
}

/// Input for creating or updating a guild record.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters, derive_builder::Builder)]
#[builder(setter(into))]
pub struct NewGuild {
    /// Discord guild snowflake ID
    pub(crate) id: i64,
    /// Guild name
    pub(crate) name: String,
    /// Icon hash for guild avatar
    #[builder(default)]
    pub(crate) icon: Option<String>,
    /// Banner hash for guild banner image
    #[builder(default)]
    pub(crate) banner: Option<String>,
    /// Splash hash for invite splash image
    #[builder(default)]
    pub(crate) splash: Option<String>,
    /// User ID of guild owner
    pub(crate) owner_id: i64,

    /// Discord guild features enabled
    #[builder(default)]
    pub(crate) features: Option<Vec<String>>,
    /// Guild description text
    #[builder(default)]
    pub(crate) description: Option<String>,
    /// Vanity URL code if enabled
    #[builder(default)]
    pub(crate) vanity_url_code: Option<String>,

    /// Total member count
    #[builder(default)]
    pub(crate) member_count: Option<i32>,
    /// Approximate member count
    #[builder(default)]
    pub(crate) approximate_member_count: Option<i32>,
    /// Approximate presence count (online members)
    #[builder(default)]
    pub(crate) approximate_presence_count: Option<i32>,

    /// AFK voice channel ID
    #[builder(default)]
    pub(crate) afk_channel_id: Option<i64>,
    /// AFK timeout in seconds
    #[builder(default)]
    pub(crate) afk_timeout: Option<i32>,
    /// System messages channel ID
    #[builder(default)]
    pub(crate) system_channel_id: Option<i64>,
    /// Rules channel ID for community guilds
    #[builder(default)]
    pub(crate) rules_channel_id: Option<i64>,
    /// Public updates channel ID for community guilds
    #[builder(default)]
    pub(crate) public_updates_channel_id: Option<i64>,

    /// Verification level required for members
    #[builder(default)]
    pub(crate) verification_level: Option<i16>,
    /// Explicit content filter level
    #[builder(default)]
    pub(crate) explicit_content_filter: Option<i16>,
    /// MFA level required for moderation actions
    #[builder(default)]
    pub(crate) mfa_level: Option<i16>,

    /// Server boost premium tier (0-3)
    #[builder(default)]
    pub(crate) premium_tier: Option<i16>,
    /// Number of server boosts
    #[builder(default)]
    pub(crate) premium_subscription_count: Option<i32>,

    /// Maximum number of presences (null for large guilds)
    #[builder(default)]
    pub(crate) max_presences: Option<i32>,
    /// Maximum number of members
    #[builder(default)]
    pub(crate) max_members: Option<i32>,
    /// Maximum users in a video channel
    #[builder(default)]
    pub(crate) max_video_channel_users: Option<i32>,

    /// Whether guild is considered large (>250 members)
    #[builder(default)]
    pub(crate) large: Option<bool>,
    /// Whether guild is unavailable due to outage
    #[builder(default)]
    pub(crate) unavailable: Option<bool>,

    /// Timestamp when bot joined guild
    #[builder(default)]
    pub(crate) joined_at: Option<DateTime<Utc>>,
    /// Timestamp when bot left guild
    #[builder(default)]
    pub(crate) left_at: Option<DateTime<Utc>>,

    /// Bot's permission bitfield in this guild
    #[builder(default)]
    pub(crate) bot_permissions: Option<i64>,
    /// Whether bot is currently active in guild
    #[builder(default)]
    pub(crate) bot_active: Option<bool>,
}

impl GuildRow {
    /// Create a `GuildRow` from a `NewGuild` input, using the provided timestamps.
    pub fn from_new(guild: &NewGuild, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        Self {
            id: guild.id,
            name: guild.name.clone(),
            icon: guild.icon.clone(),
            banner: guild.banner.clone(),
            splash: guild.splash.clone(),
            owner_id: guild.owner_id,
            features: guild.features.clone(),
            description: guild.description.clone(),
            vanity_url_code: guild.vanity_url_code.clone(),
            member_count: guild.member_count,
            approximate_member_count: guild.approximate_member_count,
            approximate_presence_count: guild.approximate_presence_count,
            afk_channel_id: guild.afk_channel_id,
            afk_timeout: guild.afk_timeout,
            system_channel_id: guild.system_channel_id,
            rules_channel_id: guild.rules_channel_id,
            public_updates_channel_id: guild.public_updates_channel_id,
            verification_level: guild.verification_level,
            explicit_content_filter: guild.explicit_content_filter,
            mfa_level: guild.mfa_level,
            premium_tier: guild.premium_tier,
            premium_subscription_count: guild.premium_subscription_count,
            max_presences: guild.max_presences,
            max_members: guild.max_members,
            max_video_channel_users: guild.max_video_channel_users,
            large: guild.large,
            unavailable: guild.unavailable,
            joined_at: guild.joined_at,
            left_at: guild.left_at,
            bot_permissions: guild.bot_permissions,
            bot_active: guild.bot_active,
            created_at,
            updated_at,
        }
    }
}
