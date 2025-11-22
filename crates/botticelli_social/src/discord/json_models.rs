//! JSON deserialization models for Discord data.
//!
//! These models match the JSON schemas defined in DISCORD_NARRATIVE.md
//! and are used to parse LLM-generated responses before inserting into
//! the database.
//!
//! These models are separate from the Diesel models in the `models` module
//! because they represent the JSON format from LLM responses, while Diesel
//! models represent the database schema.

use serde::{Deserialize, Serialize};

/// JSON model for Discord guild data.
///
/// Matches the schema defined in DISCORD_NARRATIVE.md for guild generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, derive_getters::Getters, derive_builder::Builder)]
pub struct DiscordGuildJson {
    /// Discord snowflake ID (required)
    id: i64,
    /// Guild name (required, max 100 characters)
    name: String,
    /// User ID of guild owner (required)
    owner_id: i64,

    /// Icon hash (optional)
    #[serde(default)]
    icon: Option<String>,
    /// Banner hash (optional)
    #[serde(default)]
    banner: Option<String>,
    /// Guild description (optional)
    #[serde(default)]
    description: Option<String>,
    /// Total member count (optional)
    #[serde(default)]
    member_count: Option<i32>,
    /// Verification level 0-4 (optional)
    #[serde(default)]
    verification_level: Option<i16>,
    /// Premium tier 0-3 (optional)
    #[serde(default)]
    premium_tier: Option<i16>,
    /// Array of feature flags (optional)
    #[serde(default)]
    features: Option<Vec<String>>,
}

/// JSON model for Discord channel data.
///
/// Matches the schema defined in DISCORD_NARRATIVE.md for channel generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, derive_getters::Getters, derive_builder::Builder)]
pub struct DiscordChannelJson {
    /// Discord snowflake ID (required)
    id: i64,
    /// Channel type (required)
    channel_type: String,

    /// Parent guild ID (optional, required for guild channels)
    #[serde(default)]
    guild_id: Option<i64>,
    /// Channel name (optional)
    #[serde(default)]
    name: Option<String>,
    /// Channel topic/description (optional)
    #[serde(default)]
    topic: Option<String>,
    /// Sort position (optional)
    #[serde(default)]
    position: Option<i32>,
    /// Parent category or channel ID (optional)
    #[serde(default)]
    parent_id: Option<i64>,
    /// Age-restricted content (optional)
    #[serde(default)]
    nsfw: Option<bool>,
    /// Slowmode in seconds (optional)
    #[serde(default)]
    rate_limit_per_user: Option<i32>,
    /// Voice channel bitrate (optional)
    #[serde(default)]
    bitrate: Option<i32>,
    /// Voice channel user limit (optional)
    #[serde(default)]
    user_limit: Option<i32>,
}

/// JSON model for Discord user data.
///
/// Matches the schema defined in DISCORD_NARRATIVE.md for user generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, derive_getters::Getters, derive_builder::Builder)]
pub struct DiscordUserJson {
    /// Discord snowflake ID (required)
    id: i64,
    /// Username without @ (required, max 32 characters)
    username: String,

    /// Legacy 4-digit discriminator (optional)
    #[serde(default)]
    discriminator: Option<String>,
    /// Display name (optional)
    #[serde(default)]
    global_name: Option<String>,
    /// Avatar hash (optional)
    #[serde(default)]
    avatar: Option<String>,
    /// True if bot account (optional)
    #[serde(default)]
    bot: Option<bool>,
    /// Nitro subscription 0-3 (optional)
    #[serde(default)]
    premium_type: Option<i16>,
    /// Language code (optional)
    #[serde(default)]
    locale: Option<String>,
}

/// JSON model for Discord role data.
///
/// Matches the schema defined in DISCORD_NARRATIVE.md for role generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, derive_getters::Getters, derive_builder::Builder)]
pub struct DiscordRoleJson {
    /// Discord snowflake ID (required)
    id: i64,
    /// Parent guild ID (required)
    guild_id: i64,
    /// Role name (required, max 100 characters)
    name: String,
    /// Role hierarchy position (required, 0-based)
    position: i32,
    /// Permission bitfield (required)
    permissions: i64,

    /// RGB color as decimal integer (optional, 0 for no color)
    #[serde(default)]
    color: Option<i32>,
    /// Display separately in member list (optional)
    #[serde(default)]
    hoist: Option<bool>,
    /// Role icon hash (optional)
    #[serde(default)]
    icon: Option<String>,
    /// Unicode emoji for role (optional)
    #[serde(default)]
    unicode_emoji: Option<String>,
    /// Managed by integration (optional)
    #[serde(default)]
    managed: Option<bool>,
    /// Can be @mentioned (optional)
    #[serde(default)]
    mentionable: Option<bool>,
}

/// JSON model for Discord guild member data.
///
/// Matches the schema defined in DISCORD_NARRATIVE.md for guild member generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, derive_getters::Getters, derive_builder::Builder)]
pub struct DiscordGuildMemberJson {
    /// Guild ID (required)
    guild_id: i64,
    /// User ID (required)
    user_id: i64,
    /// ISO 8601 timestamp when user joined (required)
    joined_at: String,

    /// Guild-specific nickname (optional)
    #[serde(default)]
    nick: Option<String>,
    /// Guild-specific avatar hash (optional)
    #[serde(default)]
    avatar: Option<String>,
    /// ISO 8601 timestamp when user started boosting (optional)
    #[serde(default)]
    premium_since: Option<String>,
    /// Server deafened status (optional)
    #[serde(default)]
    deaf: Option<bool>,
    /// Server muted status (optional)
    #[serde(default)]
    mute: Option<bool>,
    /// Pending membership screening (optional)
    #[serde(default)]
    pending: Option<bool>,
}

/// JSON model for Discord member role assignment data.
///
/// Matches the schema defined in DISCORD_NARRATIVE.md for member role generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, derive_getters::Getters, derive_builder::Builder)]
pub struct DiscordMemberRoleJson {
    /// Guild ID (required)
    guild_id: i64,
    /// User ID (required)
    user_id: i64,
    /// Role ID (required)
    role_id: i64,
    /// ISO 8601 timestamp when role was assigned (required)
    assigned_at: String,

    /// User ID who assigned the role (optional)
    #[serde(default)]
    assigned_by: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_guild_minimal() {
        let json = r#"{
            "id": 123456789,
            "name": "Test Guild",
            "owner_id": 987654321
        }"#;

        let guild: DiscordGuildJson = serde_json::from_str(json).unwrap();
        assert_eq!(guild.id, 123456789);
        assert_eq!(guild.name, "Test Guild");
        assert_eq!(guild.owner_id, 987654321);
        assert_eq!(guild.description, None);
    }

    #[test]
    fn test_deserialize_guild_full() {
        let json = r#"{
            "id": 123456789,
            "name": "Test Guild",
            "owner_id": 987654321,
            "description": "A test guild",
            "member_count": 100,
            "verification_level": 2,
            "premium_tier": 1,
            "features": ["COMMUNITY", "DISCOVERABLE"]
        }"#;

        let guild: DiscordGuildJson = serde_json::from_str(json).unwrap();
        assert_eq!(guild.description, Some("A test guild".to_string()));
        assert_eq!(guild.member_count, Some(100));
        assert_eq!(guild.verification_level, Some(2));
        assert_eq!(guild.features.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_deserialize_channel() {
        let json = r#"{
            "id": 111111111,
            "channel_type": "guild_text",
            "guild_id": 123456789,
            "name": "general",
            "topic": "General chat",
            "position": 0
        }"#;

        let channel: DiscordChannelJson = serde_json::from_str(json).unwrap();
        assert_eq!(channel.id, 111111111);
        assert_eq!(channel.channel_type, "guild_text");
        assert_eq!(channel.guild_id, Some(123456789));
        assert_eq!(channel.name, Some("general".to_string()));
    }

    #[test]
    fn test_deserialize_user() {
        let json = r#"{
            "id": 222222222,
            "username": "testuser",
            "global_name": "Test User",
            "bot": false
        }"#;

        let user: DiscordUserJson = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 222222222);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.global_name, Some("Test User".to_string()));
        assert_eq!(user.bot, Some(false));
    }

    #[test]
    fn test_deserialize_role() {
        let json = r#"{
            "id": 333333333,
            "guild_id": 123456789,
            "name": "Moderator",
            "position": 5,
            "permissions": 8,
            "color": 3447003,
            "hoist": true,
            "mentionable": true
        }"#;

        let role: DiscordRoleJson = serde_json::from_str(json).unwrap();
        assert_eq!(role.id, 333333333);
        assert_eq!(role.guild_id, 123456789);
        assert_eq!(role.name, "Moderator");
        assert_eq!(role.permissions, 8);
        assert_eq!(role.color, Some(3447003));
    }

    #[test]
    fn test_deserialize_guild_member() {
        let json = r#"{
            "guild_id": 123456789,
            "user_id": 222222222,
            "joined_at": "2024-01-15T14:30:00Z",
            "nick": "TestNick"
        }"#;

        let member: DiscordGuildMemberJson = serde_json::from_str(json).unwrap();
        assert_eq!(member.guild_id, 123456789);
        assert_eq!(member.user_id, 222222222);
        assert_eq!(member.joined_at, "2024-01-15T14:30:00Z");
        assert_eq!(member.nick, Some("TestNick".to_string()));
    }

    #[test]
    fn test_deserialize_member_role() {
        let json = r#"{
            "guild_id": 123456789,
            "user_id": 222222222,
            "role_id": 333333333,
            "assigned_at": "2024-01-20T10:00:00Z",
            "assigned_by": 987654321
        }"#;

        let member_role: DiscordMemberRoleJson = serde_json::from_str(json).unwrap();
        assert_eq!(member_role.guild_id, 123456789);
        assert_eq!(member_role.user_id, 222222222);
        assert_eq!(member_role.role_id, 333333333);
        assert_eq!(member_role.assigned_by, Some(987654321));
    }

    #[test]
    fn test_serialize_roundtrip() {
        let guild = DiscordGuildJson {
            id: 123,
            name: "Test".to_string(),
            owner_id: 456,
            icon: Some("abc123".to_string()),
            banner: None,
            description: Some("Test guild".to_string()),
            member_count: Some(50),
            verification_level: Some(1),
            premium_tier: Some(0),
            features: Some(vec!["COMMUNITY".to_string()]),
        };

        let json = serde_json::to_string(&guild).unwrap();
        let deserialized: DiscordGuildJson = serde_json::from_str(&json).unwrap();
        assert_eq!(guild, deserialized);
    }

    #[test]
    fn test_deserialize_array_of_channels() {
        let json = r#"[
            {
                "id": 111,
                "channel_type": "guild_text",
                "name": "general"
            },
            {
                "id": 222,
                "channel_type": "guild_voice",
                "name": "Voice"
            }
        ]"#;

        let channels: Vec<DiscordChannelJson> = serde_json::from_str(json).unwrap();
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].id, 111);
        assert_eq!(channels[1].id, 222);
    }
}
