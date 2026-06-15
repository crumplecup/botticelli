//! Discord user models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stored user data.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct UserRow {
    /// User ID
    pub id: i64,
    /// Username
    pub username: String,
    /// Legacy discriminator
    pub discriminator: Option<String>,
    /// Display name
    pub global_name: Option<String>,
    /// Avatar hash
    pub avatar: Option<String>,
    /// Banner hash
    pub banner: Option<String>,
    /// Accent color
    pub accent_color: Option<i32>,

    bot: Option<bool>,
    system: Option<bool>,
    mfa_enabled: Option<bool>,
    verified: Option<bool>,

    premium_type: Option<i16>,
    public_flags: Option<i32>,

    locale: Option<String>,

    pub(crate) first_seen: DateTime<Utc>,
    pub(crate) last_seen: DateTime<Utc>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

/// Input for creating or updating a user record.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct NewUser {
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) discriminator: Option<String>,
    pub(crate) global_name: Option<String>,
    pub(crate) avatar: Option<String>,
    pub(crate) banner: Option<String>,
    pub(crate) accent_color: Option<i32>,

    pub(crate) bot: Option<bool>,
    pub(crate) system: Option<bool>,
    pub(crate) mfa_enabled: Option<bool>,
    pub(crate) verified: Option<bool>,

    pub(crate) premium_type: Option<i16>,
    pub(crate) public_flags: Option<i32>,

    pub(crate) locale: Option<String>,
}

impl UserRow {
    /// Create a `UserRow` from a `NewUser` input, using the provided timestamps.
    pub fn from_new(user: &NewUser, now: DateTime<Utc>) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            discriminator: user.discriminator.clone(),
            global_name: user.global_name.clone(),
            avatar: user.avatar.clone(),
            banner: user.banner.clone(),
            accent_color: user.accent_color,
            bot: user.bot,
            system: user.system,
            mfa_enabled: user.mfa_enabled,
            verified: user.verified,
            premium_type: user.premium_type,
            public_flags: user.public_flags,
            locale: user.locale.clone(),
            first_seen: now,
            last_seen: now,
            created_at: now,
            updated_at: now,
        }
    }
}
