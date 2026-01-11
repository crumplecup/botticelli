//! Discord role models.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

/// Database row for discord_roles table.
///
/// Represents a Discord role within a guild, defining permissions and visual display.
#[derive(
    Debug, Clone, Queryable, Identifiable, Selectable, Associations, derive_getters::Getters,
)]
#[diesel(belongs_to(super::guild::GuildRow, foreign_key = guild_id))]
#[diesel(table_name = botticelli_database::schema::discord_roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RoleRow {
    id: i64,
    guild_id: i64,
    name: String,
    color: i32,
    hoist: Option<bool>, // Display separately in member list
    icon: Option<String>,
    unicode_emoji: Option<String>,
    position: i32,
    permissions: i64,
    managed: Option<bool>, // Managed by integration (bot, boost, etc.)
    mentionable: Option<bool>,

    // Role tags (bot, integration, premium subscriber)
    tags: Option<JsonValue>,

    // Timestamps
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

/// Insertable struct for discord_roles table.
///
/// Used to create new role records in the database.
#[allow(missing_docs)]
#[derive(Debug, Clone, Insertable, derive_getters::Getters, derive_builder::Builder)]
#[diesel(table_name = botticelli_database::schema::discord_roles)]
pub struct NewRole {
    id: i64,
    guild_id: i64,
    name: String,
    color: i32,
    hoist: Option<bool>,
    icon: Option<String>,
    unicode_emoji: Option<String>,
    position: i32,
    permissions: i64,
    managed: Option<bool>,
    mentionable: Option<bool>,

    // Role tags
    tags: Option<JsonValue>,
}
