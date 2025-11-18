//! Discord guild member models.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for discord_guild_members table.
///
/// Represents a user's membership in a specific guild with guild-specific data.
/// Uses composite primary key (guild_id, user_id).
#[derive(Debug, Clone, Queryable, Selectable, Associations)]
#[diesel(belongs_to(super::guild::GuildRow, foreign_key = guild_id))]
#[diesel(belongs_to(super::user::UserRow, foreign_key = user_id))]
#[diesel(table_name = botticelli_database::schema::discord_guild_members)]
#[diesel(primary_key(guild_id, user_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildMemberRow {
    pub guild_id: i64,
    pub user_id: i64,

    // Member-specific data
    pub nick: Option<String>,
    pub avatar: Option<String>, // Guild-specific avatar

    // Timestamps
    pub joined_at: NaiveDateTime,
    pub premium_since: Option<NaiveDateTime>, // Server boost date
    pub communication_disabled_until: Option<NaiveDateTime>, // Timeout

    // Flags
    pub deaf: Option<bool>,
    pub mute: Option<bool>,
    pub pending: Option<bool>, // Passed membership screening

    // Metadata
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub left_at: Option<NaiveDateTime>,
}

/// Insertable struct for discord_guild_members table.
///
/// Used to create new guild member records in the database.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = botticelli_database::schema::discord_guild_members)]
pub struct NewGuildMember {
    pub guild_id: i64,
    pub user_id: i64,

    // Member-specific data
    pub nick: Option<String>,
    pub avatar: Option<String>,

    // Timestamps
    pub joined_at: NaiveDateTime,
    pub premium_since: Option<NaiveDateTime>,
    pub communication_disabled_until: Option<NaiveDateTime>,

    // Flags
    pub deaf: Option<bool>,
    pub mute: Option<bool>,
    pub pending: Option<bool>,

    // left_at is set when member leaves
    pub left_at: Option<NaiveDateTime>,
}
