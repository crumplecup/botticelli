//! Conversions from JSON models to Diesel database models.
//!
//! This module provides TryFrom implementations to convert LLM-generated
//! JSON models into Diesel models for database insertion. It includes
//! helper functions for parsing timestamps and enums.

use botticelli_error::{BackendError, BotticelliResult};
use chrono::NaiveDateTime;

use crate::{
    ChannelType, DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson,
    DiscordMemberRoleJson, DiscordRoleJson, DiscordUserJson, NewChannel, NewChannelBuilder,
    NewGuild, NewGuildBuilder, NewGuildMember, NewGuildMemberBuilder, NewRole, NewRoleBuilder,
    NewUser, NewUserBuilder,
};

/// Parse an ISO 8601 timestamp string to NaiveDateTime.
///
/// Accepts formats:
/// - RFC 3339: `2024-01-15T14:30:00Z`
/// - RFC 3339 with fractional seconds: `2024-01-15T14:30:00.123Z`
/// - Without timezone: `2024-01-15T14:30:00`
///
/// # Errors
///
/// Returns an error if the timestamp string cannot be parsed.
#[track_caller]
pub fn parse_iso_timestamp(s: &str) -> BotticelliResult<NaiveDateTime> {
    // Try parsing with timezone first (strip the Z and parse as naive)
    if let Some(without_z) = s.strip_suffix('Z') {
        // Try with fractional seconds
        if let Ok(dt) = NaiveDateTime::parse_from_str(without_z, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(dt);
        }
        // Try without fractional seconds
        if let Ok(dt) = NaiveDateTime::parse_from_str(without_z, "%Y-%m-%dT%H:%M:%S") {
            return Ok(dt);
        }
    }

    // Try parsing without timezone marker
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }

    Err(BackendError::new(format!("Invalid ISO 8601 timestamp: {}", s)).into())
}

/// Convert a channel type string to ChannelType enum.
///
/// Accepts Discord API channel type names in snake_case.
///
/// # Errors
///
/// Returns an error if the channel type string is not recognized.
#[track_caller]
pub fn parse_channel_type(s: &str) -> BotticelliResult<ChannelType> {
    match s {
        "guild_text" => Ok(ChannelType::GuildText),
        "dm" => Ok(ChannelType::Dm),
        "guild_voice" => Ok(ChannelType::GuildVoice),
        "group_dm" => Ok(ChannelType::GroupDm),
        "guild_category" => Ok(ChannelType::GuildCategory),
        "guild_announcement" => Ok(ChannelType::GuildAnnouncement),
        "announcement_thread" => Ok(ChannelType::AnnouncementThread),
        "public_thread" => Ok(ChannelType::PublicThread),
        "private_thread" => Ok(ChannelType::PrivateThread),
        "guild_stage_voice" => Ok(ChannelType::GuildStageVoice),
        "guild_directory" => Ok(ChannelType::GuildDirectory),
        "guild_forum" => Ok(ChannelType::GuildForum),
        "guild_media" => Ok(ChannelType::GuildMedia),
        _ => Err(BackendError::new(format!("Unknown channel type: {}", s)).into()),
    }
}

/// Convert feature array from Vec<String> to Vec<Option<String>>.
///
/// The database schema uses `Vec<Option<String>>` for the features field,
/// so we wrap each string in Some().
fn convert_features(features: &[String]) -> Vec<Option<String>> {
    features.iter().map(|s| Some(s.clone())).collect()
}

// ============================================================================
// TryFrom implementations
// ============================================================================

impl TryFrom<DiscordGuildJson> for NewGuild {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordGuildJson) -> BotticelliResult<Self> {
        let mut builder = NewGuildBuilder::default();
        builder.id(*json.id());
        builder.name(json.name().clone());
        builder.owner_id(*json.owner_id());

        if let Some(icon) = json.icon() {
            builder.icon(Some(icon.clone()));
        }

        if let Some(banner) = json.banner() {
            builder.banner(Some(banner.clone()));
        }

        if let Some(features_vec) = json.features().as_ref() {
            builder.features(Some(convert_features(features_vec)));
        }

        if let Some(description) = json.description() {
            builder.description(Some(description.clone()));
        }

        if let Some(member_count) = json.member_count() {
            builder.member_count(Some(*member_count));
        }

        if let Some(verification_level) = json.verification_level() {
            builder.verification_level(Some(*verification_level));
        }

        if let Some(premium_tier) = json.premium_tier() {
            builder.premium_tier(Some(*premium_tier));
        }

        builder
            .build()
            .map_err(|e| BackendError::new(e.to_string()).into())
    }
}

impl TryFrom<DiscordUserJson> for NewUser {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordUserJson) -> BotticelliResult<Self> {
        Ok(NewUserBuilder::default()
            .id(*json.id())
            .username(json.username().clone())
            .discriminator(json.discriminator().clone())
            .global_name(json.global_name().clone())
            .avatar(json.avatar().clone())
            .banner(None)
            .accent_color(None)
            .bot(*json.bot())
            .system(None)
            .mfa_enabled(None)
            .verified(None)
            .premium_type(*json.premium_type())
            .public_flags(None)
            .locale(json.locale().clone())
            .build()
            .expect("Valid user"))
    }
}

impl TryFrom<DiscordChannelJson> for NewChannel {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordChannelJson) -> BotticelliResult<Self> {
        let channel_type = parse_channel_type(json.channel_type())?;

        Ok(NewChannelBuilder::default()
            .id(*json.id())
            .guild_id(*json.guild_id())
            .name(json.name().clone())
            .channel_type(channel_type)
            .position(*json.position())
            .topic(json.topic().clone())
            .nsfw(*json.nsfw())
            .rate_limit_per_user(*json.rate_limit_per_user())
            .bitrate(*json.bitrate())
            .user_limit(*json.user_limit())
            .parent_id(*json.parent_id())
            .owner_id(None)
            .message_count(None)
            .member_count(None)
            .archived(None)
            .auto_archive_duration(None)
            .archive_timestamp(None)
            .locked(None)
            .invitable(None)
            .available_tags(None)
            .default_reaction_emoji(None)
            .default_thread_rate_limit(None)
            .default_sort_order(None)
            .default_forum_layout(None)
            .last_message_at(None)
            .last_read_message_id(None)
            .bot_has_access(None)
            .build()
            .expect("Valid channel"))
    }
}

impl TryFrom<DiscordRoleJson> for NewRole {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordRoleJson) -> BotticelliResult<Self> {
        Ok(NewRoleBuilder::default()
            .id(*json.id())
            .guild_id(*json.guild_id())
            .name(json.name().clone())
            .color(json.color().unwrap_or(0))
            .hoist(*json.hoist())
            .icon(json.icon().clone())
            .unicode_emoji(json.unicode_emoji().clone())
            .position(*json.position())
            .permissions(*json.permissions())
            .managed(*json.managed())
            .mentionable(*json.mentionable())
            .tags(None)
            .build()
            .expect("Valid role"))
    }
}

impl TryFrom<DiscordGuildMemberJson> for NewGuildMember {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordGuildMemberJson) -> BotticelliResult<Self> {
        let joined_at = parse_iso_timestamp(json.joined_at())?;
        let premium_since = json
            .premium_since()
            .as_ref()
            .map(|s| parse_iso_timestamp(s))
            .transpose()?;

        Ok(NewGuildMemberBuilder::default()
            .guild_id(*json.guild_id())
            .user_id(*json.user_id())
            .nick(json.nick().clone())
            .avatar(json.avatar().clone())
            .joined_at(joined_at)
            .premium_since(premium_since)
            .communication_disabled_until(None)
            .deaf(*json.deaf())
            .mute(*json.mute())
            .pending(*json.pending())
            .left_at(None)
            .build()
            .expect("Valid guild member"))
    }
}

/// Insertable struct for discord_member_roles table.
///
/// Used to create role assignment records in the database.
#[derive(Debug, Clone, diesel::Insertable, derive_getters::Getters)]
#[diesel(table_name = botticelli_database::schema::discord_member_roles)]
pub struct NewMemberRole {
    guild_id: i64,
    user_id: i64,
    role_id: i64,
    assigned_at: NaiveDateTime,
    assigned_by: Option<i64>,
}

impl TryFrom<DiscordMemberRoleJson> for NewMemberRole {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordMemberRoleJson) -> BotticelliResult<Self> {
        let assigned_at = parse_iso_timestamp(json.assigned_at())?;

        Ok(NewMemberRole {
            guild_id: *json.guild_id(),
            user_id: *json.user_id(),
            role_id: *json.role_id(),
            assigned_at,
            assigned_by: *json.assigned_by(),
        })
    }
}
