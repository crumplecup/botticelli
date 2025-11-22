//! Conversions from JSON models to Diesel database models.
//!
//! This module provides TryFrom implementations to convert LLM-generated
//! JSON models into Diesel models for database insertion. It includes
//! helper functions for parsing timestamps and enums.

use botticelli_error::{BackendError, BotticelliResult};
use chrono::NaiveDateTime;

use super::{
    ChannelType, DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson,
    DiscordMemberRoleJson, DiscordRoleJson, DiscordUserJson, NewChannel, NewGuild, NewGuildMember,
    NewRole, NewUser,
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
        Ok(NewGuild {
            id: *json.id(),
            name: json.name().clone(),
            icon: json.icon().clone(),
            banner: json.banner().clone(),
            splash: None, // Not in JSON model
            owner_id: *json.owner_id(),

            // Guild features
            features: json.features().as_ref().map(|v| convert_features(v)),
            description: json.description().clone(),
            vanity_url_code: None, // Not in JSON model

            // Member counts
            member_count: *json.member_count(),
            approximate_member_count: None,   // Not in JSON model
            approximate_presence_count: None, // Not in JSON model

            // Guild settings
            afk_channel_id: None,            // Not in JSON model
            afk_timeout: None,               // Not in JSON model
            system_channel_id: None,         // Not in JSON model
            rules_channel_id: None,          // Not in JSON model
            public_updates_channel_id: None, // Not in JSON model

            // Verification and content filtering
            verification_level: *json.verification_level(),
            explicit_content_filter: None, // Not in JSON model
            mfa_level: None,               // Not in JSON model

            // Premium features
            premium_tier: *json.premium_tier(),
            premium_subscription_count: None, // Not in JSON model

            // Server boost progress
            max_presences: None,           // Not in JSON model
            max_members: None,             // Not in JSON model
            max_video_channel_users: None, // Not in JSON model

            // Status flags
            large: None,       // Not in JSON model
            unavailable: None, // Not in JSON model

            // Timestamps
            joined_at: None, // Set by database/bot when joining
            left_at: None,   // Not applicable for new guilds

            // Bot-specific metadata
            bot_permissions: None, // Not in JSON model
            bot_active: None,      // Not in JSON model
        })
    }
}

impl TryFrom<DiscordUserJson> for NewUser {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordUserJson) -> BotticelliResult<Self> {
        Ok(NewUser {
            id: *json.id(),
            username: json.username().clone(),
            discriminator: json.discriminator().clone(),
            global_name: json.global_name().clone(),
            avatar: json.avatar().clone(),
            banner: None,       // Not in JSON model
            accent_color: None, // Not in JSON model

            // Account flags
            bot: *json.bot(),
            system: None,      // Not in JSON model
            mfa_enabled: None, // Not in JSON model
            verified: None,    // Not in JSON model

            // Premium status
            premium_type: *json.premium_type(),
            public_flags: None, // Not in JSON model

            // Locale
            locale: json.locale().clone(),
        })
    }
}

impl TryFrom<DiscordChannelJson> for NewChannel {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordChannelJson) -> BotticelliResult<Self> {
        let channel_type = parse_channel_type(json.channel_type())?;

        Ok(NewChannel {
            id: *json.id(),
            guild_id: *json.guild_id(),
            name: json.name().clone(),
            channel_type,
            position: *json.position(),

            // Topic and description
            topic: json.topic().clone(),

            // Channel settings
            nsfw: *json.nsfw(),
            rate_limit_per_user: *json.rate_limit_per_user(),
            bitrate: *json.bitrate(),
            user_limit: *json.user_limit(),

            // Thread-specific
            parent_id: *json.parent_id(),
            owner_id: None,              // Not in JSON model
            message_count: None,         // Not in JSON model
            member_count: None,          // Not in JSON model
            archived: None,              // Not in JSON model
            auto_archive_duration: None, // Not in JSON model
            archive_timestamp: None,     // Not in JSON model
            locked: None,                // Not in JSON model
            invitable: None,             // Not in JSON model

            // Forum-specific
            available_tags: None,            // Not in JSON model
            default_reaction_emoji: None,    // Not in JSON model
            default_thread_rate_limit: None, // Not in JSON model
            default_sort_order: None,        // Not in JSON model
            default_forum_layout: None,      // Not in JSON model

            // Timestamps
            last_message_at: None, // Not in JSON model

            // Bot tracking
            last_read_message_id: None, // Not in JSON model
            bot_has_access: None,       // Not in JSON model
        })
    }
}

impl TryFrom<DiscordRoleJson> for NewRole {
    type Error = botticelli_error::BotticelliError;

    fn try_from(json: DiscordRoleJson) -> BotticelliResult<Self> {
        Ok(NewRole {
            id: *json.id(),
            guild_id: *json.guild_id(),
            name: json.name().clone(),
            color: json.color().unwrap_or(0), // Default to no color
            hoist: *json.hoist(),
            icon: json.icon().clone(),
            unicode_emoji: json.unicode_emoji().clone(),
            position: *json.position(),
            permissions: *json.permissions(),
            managed: *json.managed(),
            mentionable: *json.mentionable(),

            // Role tags
            tags: None, // Not in JSON model
        })
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

        Ok(NewGuildMember {
            guild_id: *json.guild_id(),
            user_id: *json.user_id(),

            // Member-specific data
            nick: json.nick().clone(),
            avatar: json.avatar().clone(),

            // Timestamps
            joined_at,
            premium_since,
            communication_disabled_until: None, // Not in JSON model

            // Flags
            deaf: *json.deaf(),
            mute: *json.mute(),
            pending: *json.pending(),

            // left_at is None for new members
            left_at: None,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::json_models::{
        DiscordChannelJsonBuilder, DiscordGuildJsonBuilder, DiscordGuildMemberJsonBuilder,
        DiscordMemberRoleJsonBuilder, DiscordRoleJsonBuilder, DiscordUserJsonBuilder,
    };
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_iso_timestamp_with_z() {
        let result = parse_iso_timestamp("2024-01-15T14:30:00Z");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_iso_timestamp_with_fractional() {
        let result = parse_iso_timestamp("2024-01-15T14:30:00.123Z");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_iso_timestamp_without_z() {
        let result = parse_iso_timestamp("2024-01-15T14:30:00");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_iso_timestamp_invalid() {
        let result = parse_iso_timestamp("not a timestamp");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_channel_type_guild_text() {
        let result = parse_channel_type("guild_text");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ChannelType::GuildText);
    }

    #[test]
    fn test_parse_channel_type_guild_voice() {
        let result = parse_channel_type("guild_voice");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ChannelType::GuildVoice);
    }

    #[test]
    fn test_parse_channel_type_invalid() {
        let result = parse_channel_type("invalid_type");
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_features() {
        let input = vec!["COMMUNITY".to_string(), "DISCOVERABLE".to_string()];
        let output = convert_features(&input);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], Some("COMMUNITY".to_string()));
        assert_eq!(output[1], Some("DISCOVERABLE".to_string()));
    }

    #[test]
    fn test_guild_json_to_new_guild() {
        let json = DiscordGuildJsonBuilder::default()
            .id(123456789)
            .name("Test Guild".to_string())
            .owner_id(987654321)
            .icon(Some("icon_hash".to_string()))
            .banner(None)
            .description(Some("A test guild".to_string()))
            .member_count(Some(100))
            .verification_level(Some(2))
            .premium_tier(Some(1))
            .features(Some(vec!["COMMUNITY".to_string()]))
            .build()
            .unwrap();

        let result: Result<NewGuild, _> = json.try_into();
        assert!(result.is_ok());

        let guild = result.unwrap();
        assert_eq!(guild.id, 123456789);
        assert_eq!(guild.name, "Test Guild");
        assert_eq!(guild.owner_id, 987654321);
        assert_eq!(guild.description, Some("A test guild".to_string()));
        assert_eq!(guild.verification_level, Some(2));
    }

    #[test]
    fn test_user_json_to_new_user() {
        let json = DiscordUserJsonBuilder::default()
            .id(222222222)
            .username("testuser".to_string())
            .discriminator(Some("1234".to_string()))
            .global_name(Some("Test User".to_string()))
            .avatar(Some("avatar_hash".to_string()))
            .bot(Some(false))
            .premium_type(Some(2))
            .locale(Some("en-US".to_string()))
            .build()
            .unwrap();

        let result: Result<NewUser, _> = json.try_into();
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.id, 222222222);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.global_name, Some("Test User".to_string()));
    }

    #[test]
    fn test_channel_json_to_new_channel() {
        let json = DiscordChannelJsonBuilder::default()
            .id(111111111)
            .channel_type("guild_text".to_string())
            .guild_id(Some(123456789))
            .name(Some("general".to_string()))
            .topic(Some("General chat".to_string()))
            .position(Some(0))
            .parent_id(None)
            .nsfw(Some(false))
            .rate_limit_per_user(None)
            .bitrate(None)
            .user_limit(None)
            .build()
            .unwrap();

        let result: Result<NewChannel, _> = json.try_into();
        assert!(result.is_ok());

        let channel = result.unwrap();
        assert_eq!(channel.id, 111111111);
        assert_eq!(channel.channel_type, ChannelType::GuildText);
        assert_eq!(channel.guild_id, Some(123456789));
        assert_eq!(channel.name, Some("general".to_string()));
    }

    #[test]
    fn test_role_json_to_new_role() {
        let json = DiscordRoleJsonBuilder::default()
            .id(333333333)
            .guild_id(123456789)
            .name("Moderator".to_string())
            .position(5)
            .permissions(8)
            .color(Some(3447003))
            .hoist(Some(true))
            .icon(None)
            .unicode_emoji(None)
            .managed(Some(false))
            .mentionable(Some(true))
            .build()
            .unwrap();

        let result: Result<NewRole, _> = json.try_into();
        assert!(result.is_ok());

        let role = result.unwrap();
        assert_eq!(role.id, 333333333);
        assert_eq!(role.guild_id, 123456789);
        assert_eq!(role.name, "Moderator");
        assert_eq!(role.permissions, 8);
        assert_eq!(role.color, 3447003);
    }

    #[test]
    fn test_guild_member_json_to_new_guild_member() {
        let json = DiscordGuildMemberJsonBuilder::default()
            .guild_id(123456789)
            .user_id(222222222)
            .joined_at("2024-01-15T14:30:00Z".to_string())
            .nick(Some("TestNick".to_string()))
            .avatar(None)
            .premium_since(Some("2024-02-01T10:00:00Z".to_string()))
            .deaf(Some(false))
            .mute(Some(false))
            .pending(Some(false))
            .build()
            .unwrap();

        let result: Result<NewGuildMember, _> = json.try_into();
        assert!(result.is_ok());

        let member = result.unwrap();
        assert_eq!(member.guild_id, 123456789);
        assert_eq!(member.user_id, 222222222);
        assert_eq!(member.nick, Some("TestNick".to_string()));
        assert!(member.premium_since.is_some());
    }

    #[test]
    fn test_member_role_json_to_new_member_role() {
        let json = DiscordMemberRoleJsonBuilder::default()
            .guild_id(123456789)
            .user_id(222222222)
            .role_id(333333333)
            .assigned_at("2024-01-20T10:00:00Z".to_string())
            .assigned_by(Some(987654321))
            .build()
            .unwrap();

        let result: Result<NewMemberRole, _> = json.try_into();
        assert!(result.is_ok());

        let member_role = result.unwrap();
        assert_eq!(member_role.guild_id, 123456789);
        assert_eq!(member_role.user_id, 222222222);
        assert_eq!(member_role.role_id, 333333333);
        assert_eq!(member_role.assigned_by, Some(987654321));
    }
}
