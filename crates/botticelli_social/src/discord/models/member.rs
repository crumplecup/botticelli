//! Discord guild member models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stored guild member data.
///
/// Represents a user's membership in a specific guild with guild-specific data.
/// Uses composite key (guild_id, user_id).
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct GuildMemberRow {
    /// Guild ID
    pub guild_id: i64,
    /// User ID
    pub user_id: i64,

    nick: Option<String>,
    avatar: Option<String>,

    joined_at: DateTime<Utc>,
    premium_since: Option<DateTime<Utc>>,
    communication_disabled_until: Option<DateTime<Utc>>,

    deaf: Option<bool>,
    mute: Option<bool>,
    pending: Option<bool>,

    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) left_at: Option<DateTime<Utc>>,
}

/// Input for creating or updating a guild member record.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct NewGuildMember {
    pub(crate) guild_id: i64,
    pub(crate) user_id: i64,

    pub(crate) nick: Option<String>,
    pub(crate) avatar: Option<String>,

    pub(crate) joined_at: DateTime<Utc>,
    pub(crate) premium_since: Option<DateTime<Utc>>,
    pub(crate) communication_disabled_until: Option<DateTime<Utc>>,

    pub(crate) deaf: Option<bool>,
    pub(crate) mute: Option<bool>,
    pub(crate) pending: Option<bool>,

    pub(crate) left_at: Option<DateTime<Utc>>,
}

impl GuildMemberRow {
    /// Create a `GuildMemberRow` from a `NewGuildMember` input, using the provided timestamps.
    pub fn from_new(
        member: &NewGuildMember,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            guild_id: member.guild_id,
            user_id: member.user_id,
            nick: member.nick.clone(),
            avatar: member.avatar.clone(),
            joined_at: member.joined_at,
            premium_since: member.premium_since,
            communication_disabled_until: member.communication_disabled_until,
            deaf: member.deaf,
            mute: member.mute,
            pending: member.pending,
            left_at: member.left_at,
            created_at,
            updated_at,
        }
    }
}
