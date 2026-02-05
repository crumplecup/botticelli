//! Discord guild member models.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for discord_guild_members table.
///
/// Represents a user's membership in a specific guild with guild-specific data.
/// Uses composite primary key (guild_id, user_id).
#[derive(Debug, Clone, Queryable, Selectable, Associations, derive_getters::Getters)]
#[diesel(belongs_to(super::guild::GuildRow, foreign_key = guild_id))]
#[diesel(belongs_to(super::user::UserRow, foreign_key = user_id))]
#[diesel(table_name = botticelli_database::schema::discord_guild_members)]
#[diesel(primary_key(guild_id, user_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildMemberRow {
    /// Guild ID
    guild_id: i64,
    /// User ID
    user_id: i64,

    // Member-specific data
    nick: Option<String>,
    avatar: Option<String>, // Guild-specific avatar

    // Timestamps
    joined_at: NaiveDateTime,
    premium_since: Option<NaiveDateTime>, // Server boost date
    communication_disabled_until: Option<NaiveDateTime>, // Timeout

    // Flags
    deaf: Option<bool>,
    mute: Option<bool>,
    pending: Option<bool>, // Passed membership screening

    // Metadata
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    left_at: Option<NaiveDateTime>,
}

/// Insertable struct for discord_guild_members table.
///
/// Used to create new guild member records in the database.
#[allow(missing_docs)]
#[derive(
    Debug, Clone, Insertable, derive_getters::Getters, derive_builder::Builder, elicitation::Elicit,
)]
#[diesel(table_name = botticelli_database::schema::discord_guild_members)]
pub struct NewGuildMember {
    guild_id: i64,
    user_id: i64,

    // Member-specific data
    nick: Option<String>,
    avatar: Option<String>,

    // Timestamps
    joined_at: NaiveDateTime,
    premium_since: Option<NaiveDateTime>,
    communication_disabled_until: Option<NaiveDateTime>,

    // Flags
    deaf: Option<bool>,
    mute: Option<bool>,
    pending: Option<bool>,

    // left_at is set when member leaves
    left_at: Option<NaiveDateTime>,
}
