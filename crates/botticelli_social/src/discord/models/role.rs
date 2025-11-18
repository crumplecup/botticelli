//! Discord role models.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

/// Database row for discord_roles table.
///
/// Represents a Discord role within a guild, defining permissions and visual display.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, Associations)]
#[diesel(belongs_to(super::guild::GuildRow, foreign_key = guild_id))]
#[diesel(table_name = botticelli_database::schema::discord_roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RoleRow {
    pub id: i64,
    pub guild_id: i64,
    pub name: String,
    pub color: i32,
    pub hoist: Option<bool>, // Display separately in member list
    pub icon: Option<String>,
    pub unicode_emoji: Option<String>,
    pub position: i32,
    pub permissions: i64,
    pub managed: Option<bool>, // Managed by integration (bot, boost, etc.)
    pub mentionable: Option<bool>,

    // Role tags (bot, integration, premium subscriber)
    pub tags: Option<JsonValue>,

    // Timestamps
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Insertable struct for discord_roles table.
///
/// Used to create new role records in the database.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = botticelli_database::schema::discord_roles)]
pub struct NewRole {
    pub id: i64,
    pub guild_id: i64,
    pub name: String,
    pub color: i32,
    pub hoist: Option<bool>,
    pub icon: Option<String>,
    pub unicode_emoji: Option<String>,
    pub position: i32,
    pub permissions: i64,
    pub managed: Option<bool>,
    pub mentionable: Option<bool>,

    // Role tags
    pub tags: Option<JsonValue>,
}
