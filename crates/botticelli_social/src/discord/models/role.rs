//! Discord role models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Stored role data.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct RoleRow {
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

    tags: Option<JsonValue>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Input for creating or updating a role record.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct NewRole {
    pub(crate) id: i64,
    pub(crate) guild_id: i64,
    pub(crate) name: String,
    pub(crate) color: i32,
    pub(crate) hoist: Option<bool>,
    pub(crate) icon: Option<String>,
    pub(crate) unicode_emoji: Option<String>,
    pub(crate) position: i32,
    pub(crate) permissions: i64,
    pub(crate) managed: Option<bool>,
    pub(crate) mentionable: Option<bool>,

    pub(crate) tags: Option<JsonValue>,
}

impl RoleRow {
    /// Create a `RoleRow` from a `NewRole` input, using the provided timestamps.
    pub fn from_new(role: &NewRole, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        Self {
            id: role.id,
            guild_id: role.guild_id,
            name: role.name.clone(),
            color: role.color,
            hoist: role.hoist,
            icon: role.icon.clone(),
            unicode_emoji: role.unicode_emoji.clone(),
            position: role.position,
            permissions: role.permissions,
            managed: role.managed,
            mentionable: role.mentionable,
            tags: role.tags.clone(),
            created_at,
            updated_at,
        }
    }
}
