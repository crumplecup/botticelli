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
#[derive(Debug, Clone, Insertable, derive_getters::Getters)]
#[diesel(table_name = botticelli_database::schema::discord_users)]
pub struct NewUser {
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) discriminator: Option<String>,
    pub(crate) global_name: Option<String>,
    pub(crate) avatar: Option<String>,
    pub(crate) banner: Option<String>,
    pub(crate) accent_color: Option<i32>,

    // Account flags
    pub(crate) bot: Option<bool>,
    pub(crate) system: Option<bool>,
    pub(crate) mfa_enabled: Option<bool>,
    pub(crate) verified: Option<bool>,

    // Premium status
    pub(crate) premium_type: Option<i16>,
    pub(crate) public_flags: Option<i32>,

    // Locale
    pub(crate) locale: Option<String>,
}
