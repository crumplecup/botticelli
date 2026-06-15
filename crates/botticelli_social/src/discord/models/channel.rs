//! Discord channel models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Discord channel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Text channel in a guild
    GuildText,
    /// Direct message channel
    Dm,
    /// Voice channel in a guild
    GuildVoice,
    /// Group direct message channel
    GroupDm,
    /// Category that contains channels
    GuildCategory,
    /// Announcement channel (formerly news channel)
    GuildAnnouncement,
    /// Thread in an announcement channel
    AnnouncementThread,
    /// Public thread
    PublicThread,
    /// Private thread
    PrivateThread,
    /// Stage voice channel
    GuildStageVoice,
    /// Guild directory channel
    GuildDirectory,
    /// Forum channel
    GuildForum,
    /// Media channel
    GuildMedia,
}

/// Stored channel data.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct ChannelRow {
    id: i64,
    guild_id: Option<i64>,
    name: Option<String>,
    channel_type: ChannelType,
    position: Option<i32>,

    topic: Option<String>,

    nsfw: Option<bool>,
    rate_limit_per_user: Option<i32>,
    bitrate: Option<i32>,
    user_limit: Option<i32>,

    parent_id: Option<i64>,
    owner_id: Option<i64>,
    message_count: Option<i32>,
    member_count: Option<i32>,
    archived: Option<bool>,
    auto_archive_duration: Option<i32>,
    archive_timestamp: Option<DateTime<Utc>>,
    locked: Option<bool>,
    invitable: Option<bool>,

    available_tags: Option<JsonValue>,
    default_reaction_emoji: Option<JsonValue>,
    default_thread_rate_limit: Option<i32>,
    default_sort_order: Option<i16>,
    default_forum_layout: Option<i16>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_message_at: Option<DateTime<Utc>>,

    last_read_message_id: Option<i64>,
    bot_has_access: Option<bool>,
}

/// Input for creating or updating a channel record.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct NewChannel {
    pub(crate) id: i64,
    pub(crate) guild_id: Option<i64>,
    pub(crate) name: Option<String>,
    pub(crate) channel_type: ChannelType,
    pub(crate) position: Option<i32>,

    pub(crate) topic: Option<String>,

    pub(crate) nsfw: Option<bool>,
    pub(crate) rate_limit_per_user: Option<i32>,
    pub(crate) bitrate: Option<i32>,
    pub(crate) user_limit: Option<i32>,

    pub(crate) parent_id: Option<i64>,
    pub(crate) owner_id: Option<i64>,
    pub(crate) message_count: Option<i32>,
    pub(crate) member_count: Option<i32>,
    pub(crate) archived: Option<bool>,
    pub(crate) auto_archive_duration: Option<i32>,
    pub(crate) archive_timestamp: Option<DateTime<Utc>>,
    pub(crate) locked: Option<bool>,
    pub(crate) invitable: Option<bool>,

    pub(crate) available_tags: Option<JsonValue>,
    pub(crate) default_reaction_emoji: Option<JsonValue>,
    pub(crate) default_thread_rate_limit: Option<i32>,
    pub(crate) default_sort_order: Option<i16>,
    pub(crate) default_forum_layout: Option<i16>,

    pub(crate) last_message_at: Option<DateTime<Utc>>,

    pub(crate) last_read_message_id: Option<i64>,
    pub(crate) bot_has_access: Option<bool>,
}

impl ChannelRow {
    /// Create a `ChannelRow` from a `NewChannel` input, using the provided timestamps.
    pub fn from_new(
        channel: &NewChannel,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: channel.id,
            guild_id: channel.guild_id,
            name: channel.name.clone(),
            channel_type: channel.channel_type,
            position: channel.position,
            topic: channel.topic.clone(),
            nsfw: channel.nsfw,
            rate_limit_per_user: channel.rate_limit_per_user,
            bitrate: channel.bitrate,
            user_limit: channel.user_limit,
            parent_id: channel.parent_id,
            owner_id: channel.owner_id,
            message_count: channel.message_count,
            member_count: channel.member_count,
            archived: channel.archived,
            auto_archive_duration: channel.auto_archive_duration,
            archive_timestamp: channel.archive_timestamp,
            locked: channel.locked,
            invitable: channel.invitable,
            available_tags: channel.available_tags.clone(),
            default_reaction_emoji: channel.default_reaction_emoji.clone(),
            default_thread_rate_limit: channel.default_thread_rate_limit,
            default_sort_order: channel.default_sort_order,
            default_forum_layout: channel.default_forum_layout,
            last_message_at: channel.last_message_at,
            last_read_message_id: channel.last_read_message_id,
            bot_has_access: channel.bot_has_access,
            created_at,
            updated_at,
        }
    }
}
