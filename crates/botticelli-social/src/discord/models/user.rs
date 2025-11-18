//! Discord user models.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for discord_users table.
///
/// Represents a Discord user account with global profile information.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name = botticelli_database::schema::discord_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub discriminator: Option<String>, // Legacy discriminator
    pub global_name: Option<String>,   // Display name
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub accent_color: Option<i32>,

    // Account flags
    pub bot: Option<bool>,
    pub system: Option<bool>,
    pub mfa_enabled: Option<bool>,
    pub verified: Option<bool>,

    // Premium status
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,

    // Locale
    pub locale: Option<String>,

    // Timestamps
    pub first_seen: NaiveDateTime,
    pub last_seen: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Insertable struct for discord_users table.
///
/// Used to create new user records in the database.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = botticelli_database::schema::discord_users)]
pub struct NewUser {
    pub id: i64,
    pub username: String,
    pub discriminator: Option<String>,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub accent_color: Option<i32>,

    // Account flags
    pub bot: Option<bool>,
    pub system: Option<bool>,
    pub mfa_enabled: Option<bool>,
    pub verified: Option<bool>,

    // Premium status
    pub premium_type: Option<i16>,
    pub public_flags: Option<i32>,

    // Locale
    pub locale: Option<String>,
}
