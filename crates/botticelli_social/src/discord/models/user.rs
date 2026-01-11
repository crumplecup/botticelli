//! Discord user models.

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// Database row for discord_users table.
///
/// Represents a Discord user account with global profile information.
#[derive(Debug, Clone, Queryable, Identifiable, Selectable, derive_getters::Getters)]
#[diesel(table_name = botticelli_database::schema::discord_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserRow {
    /// User ID
    id: i64,
    /// Username
    username: String,
    /// Legacy discriminator
    discriminator: Option<String>,
    /// Display name
    global_name: Option<String>,
    /// Avatar hash
    avatar: Option<String>,
    /// Banner hash
    banner: Option<String>,
    /// Accent color
    accent_color: Option<i32>,

    // Account flags
    bot: Option<bool>,
    system: Option<bool>,
    mfa_enabled: Option<bool>,
    verified: Option<bool>,

    // Premium status
    premium_type: Option<i16>,
    public_flags: Option<i32>,

    // Locale
    locale: Option<String>,

    // Timestamps
    first_seen: NaiveDateTime,
    last_seen: NaiveDateTime,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

/// Insertable struct for discord_users table.
///
/// Used to create new user records in the database.
#[derive(Debug, Clone, Insertable, derive_getters::Getters, derive_builder::Builder)]
#[diesel(table_name = botticelli_database::schema::discord_users)]
pub struct NewUser {
    id: i64,
    username: String,
    discriminator: Option<String>,
    global_name: Option<String>,
    avatar: Option<String>,
    banner: Option<String>,
    accent_color: Option<i32>,

    // Account flags
    bot: Option<bool>,
    system: Option<bool>,
    mfa_enabled: Option<bool>,
    verified: Option<bool>,

    // Premium status
    premium_type: Option<i16>,
    public_flags: Option<i32>,

    // Locale
    locale: Option<String>,
}
