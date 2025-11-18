//! Guild (Discord server) models.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for discord_guilds table.
///
/// Represents a Discord guild (server) with all metadata, settings, and bot-specific state.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name = botticelli_database::schema::discord_guilds)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildRow {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub splash: Option<String>,
    pub owner_id: i64,

    // Guild features
    pub features: Option<Vec<Option<String>>>,
    pub description: Option<String>,
    pub vanity_url_code: Option<String>,

    // Member counts
    pub member_count: Option<i32>,
    pub approximate_member_count: Option<i32>,
    pub approximate_presence_count: Option<i32>,

    // Guild settings
    pub afk_channel_id: Option<i64>,
    pub afk_timeout: Option<i32>,
    pub system_channel_id: Option<i64>,
    pub rules_channel_id: Option<i64>,
    pub public_updates_channel_id: Option<i64>,

    // Verification and content filtering
    pub verification_level: Option<i16>,
    pub explicit_content_filter: Option<i16>,
    pub mfa_level: Option<i16>,

    // Premium features
    pub premium_tier: Option<i16>,
    pub premium_subscription_count: Option<i32>,

    // Server boost progress
    pub max_presences: Option<i32>,
    pub max_members: Option<i32>,
    pub max_video_channel_users: Option<i32>,

    // Status flags
    pub large: Option<bool>,
    pub unavailable: Option<bool>,

    // Timestamps
    pub joined_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub left_at: Option<NaiveDateTime>,

    // Bot-specific metadata
    pub bot_permissions: Option<i64>,
    pub bot_active: Option<bool>,
}

/// Insertable struct for discord_guilds table.
///
/// Used to create new guild records in the database.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = botticelli_database::schema::discord_guilds)]
pub struct NewGuild {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub splash: Option<String>,
    pub owner_id: i64,

    // Guild features
    pub features: Option<Vec<Option<String>>>,
    pub description: Option<String>,
    pub vanity_url_code: Option<String>,

    // Member counts
    pub member_count: Option<i32>,
    pub approximate_member_count: Option<i32>,
    pub approximate_presence_count: Option<i32>,

    // Guild settings
    pub afk_channel_id: Option<i64>,
    pub afk_timeout: Option<i32>,
    pub system_channel_id: Option<i64>,
    pub rules_channel_id: Option<i64>,
    pub public_updates_channel_id: Option<i64>,

    // Verification and content filtering
    pub verification_level: Option<i16>,
    pub explicit_content_filter: Option<i16>,
    pub mfa_level: Option<i16>,

    // Premium features
    pub premium_tier: Option<i16>,
    pub premium_subscription_count: Option<i32>,

    // Server boost progress
    pub max_presences: Option<i32>,
    pub max_members: Option<i32>,
    pub max_video_channel_users: Option<i32>,

    // Status flags
    pub large: Option<bool>,
    pub unavailable: Option<bool>,

    // Timestamps
    pub joined_at: Option<NaiveDateTime>,
    pub left_at: Option<NaiveDateTime>,

    // Bot-specific metadata
    pub bot_permissions: Option<i64>,
    pub bot_active: Option<bool>,
}
