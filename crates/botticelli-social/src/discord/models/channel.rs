//! Discord channel models.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

/// Discord channel type enum.
///
/// Maps to the discord_channel_type PostgreSQL ENUM.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    diesel::deserialize::FromSqlRow,
    diesel::expression::AsExpression,
)]
#[diesel(sql_type = botticelli_database::schema::sql_types::DiscordChannelType)]
pub enum ChannelType {
    GuildText,
    Dm,
    GuildVoice,
    GroupDm,
    GuildCategory,
    GuildAnnouncement,
    AnnouncementThread,
    PublicThread,
    PrivateThread,
    GuildStageVoice,
    GuildDirectory,
    GuildForum,
    GuildMedia,
}

impl
    diesel::serialize::ToSql<botticelli_database::schema::sql_types::DiscordChannelType, diesel::pg::Pg>
    for ChannelType
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        use std::io::Write;
        let s = match self {
            ChannelType::GuildText => "guild_text",
            ChannelType::Dm => "dm",
            ChannelType::GuildVoice => "guild_voice",
            ChannelType::GroupDm => "group_dm",
            ChannelType::GuildCategory => "guild_category",
            ChannelType::GuildAnnouncement => "guild_announcement",
            ChannelType::AnnouncementThread => "announcement_thread",
            ChannelType::PublicThread => "public_thread",
            ChannelType::PrivateThread => "private_thread",
            ChannelType::GuildStageVoice => "guild_stage_voice",
            ChannelType::GuildDirectory => "guild_directory",
            ChannelType::GuildForum => "guild_forum",
            ChannelType::GuildMedia => "guild_media",
        };
        out.write_all(s.as_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

impl
    diesel::deserialize::FromSql<
        botticelli_database::schema::sql_types::DiscordChannelType,
        diesel::pg::Pg,
    > for ChannelType
{
    fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"guild_text" => Ok(ChannelType::GuildText),
            b"dm" => Ok(ChannelType::Dm),
            b"guild_voice" => Ok(ChannelType::GuildVoice),
            b"group_dm" => Ok(ChannelType::GroupDm),
            b"guild_category" => Ok(ChannelType::GuildCategory),
            b"guild_announcement" => Ok(ChannelType::GuildAnnouncement),
            b"announcement_thread" => Ok(ChannelType::AnnouncementThread),
            b"public_thread" => Ok(ChannelType::PublicThread),
            b"private_thread" => Ok(ChannelType::PrivateThread),
            b"guild_stage_voice" => Ok(ChannelType::GuildStageVoice),
            b"guild_directory" => Ok(ChannelType::GuildDirectory),
            b"guild_forum" => Ok(ChannelType::GuildForum),
            b"guild_media" => Ok(ChannelType::GuildMedia),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

/// Database row for discord_channels table.
///
/// Represents a Discord channel (text, voice, thread, forum, etc.) with all settings and metadata.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, Associations)]
#[diesel(belongs_to(super::guild::GuildRow, foreign_key = guild_id))]
#[diesel(table_name = botticelli_database::schema::discord_channels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChannelRow {
    pub id: i64,
    pub guild_id: Option<i64>,
    pub name: Option<String>,
    pub channel_type: ChannelType,
    pub position: Option<i32>,

    // Topic and description
    pub topic: Option<String>,

    // Channel settings
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<i32>,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,

    // Thread-specific
    pub parent_id: Option<i64>,
    pub owner_id: Option<i64>,
    pub message_count: Option<i32>,
    pub member_count: Option<i32>,
    pub archived: Option<bool>,
    pub auto_archive_duration: Option<i32>,
    pub archive_timestamp: Option<NaiveDateTime>,
    pub locked: Option<bool>,
    pub invitable: Option<bool>,

    // Forum-specific
    pub available_tags: Option<JsonValue>,
    pub default_reaction_emoji: Option<JsonValue>,
    pub default_thread_rate_limit: Option<i32>,
    pub default_sort_order: Option<i16>,
    pub default_forum_layout: Option<i16>,

    // Timestamps
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_message_at: Option<NaiveDateTime>,

    // Bot tracking
    pub last_read_message_id: Option<i64>,
    pub bot_has_access: Option<bool>,
}

/// Insertable struct for discord_channels table.
///
/// Used to create new channel records in the database.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = botticelli_database::schema::discord_channels)]
pub struct NewChannel {
    pub id: i64,
    pub guild_id: Option<i64>,
    pub name: Option<String>,
    pub channel_type: ChannelType,
    pub position: Option<i32>,

    // Topic and description
    pub topic: Option<String>,

    // Channel settings
    pub nsfw: Option<bool>,
    pub rate_limit_per_user: Option<i32>,
    pub bitrate: Option<i32>,
    pub user_limit: Option<i32>,

    // Thread-specific
    pub parent_id: Option<i64>,
    pub owner_id: Option<i64>,
    pub message_count: Option<i32>,
    pub member_count: Option<i32>,
    pub archived: Option<bool>,
    pub auto_archive_duration: Option<i32>,
    pub archive_timestamp: Option<NaiveDateTime>,
    pub locked: Option<bool>,
    pub invitable: Option<bool>,

    // Forum-specific
    pub available_tags: Option<JsonValue>,
    pub default_reaction_emoji: Option<JsonValue>,
    pub default_thread_rate_limit: Option<i32>,
    pub default_sort_order: Option<i16>,
    pub default_forum_layout: Option<i16>,

    // Timestamps (last_message_at can be set)
    pub last_message_at: Option<NaiveDateTime>,

    // Bot tracking
    pub last_read_message_id: Option<i64>,
    pub bot_has_access: Option<bool>,
}
