//! Serenity event handler for Discord bot.
//!
//! This module implements the EventHandler trait to respond to Discord events
//! and persist data to the database.

use crate::{DiscordRepository, NewChannel, NewGuild, NewGuildMember, NewRole, NewUser};
use serenity::all::{GuildId, Ready};
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::{Channel, GuildChannel};
use serenity::model::gateway::GatewayIntents;
use serenity::model::guild::{Guild, Member, Role};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use super::models::ChannelType;

/// Event handler for the Boticelli Discord bot.
///
/// Implements Serenity's EventHandler trait to respond to Discord events
/// and persist data to the database via DiscordRepository.
pub struct BoticelliHandler {
    /// Repository for database operations
    repository: Arc<DiscordRepository>,
}

impl BoticelliHandler {
    /// Create a new BoticelliHandler with the given repository.
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }

    /// Required gateway intents for the bot.
    ///
    /// This specifies what events the bot will receive from Discord.
    pub fn intents() -> GatewayIntents {
        GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
    }

    /// Convert Discord snowflake ID (u64) to database ID (i64).
    ///
    /// Discord IDs are 64-bit unsigned integers, but PostgreSQL uses signed bigints.
    fn to_db_id(id: u64) -> i64 {
        id as i64
    }

    /// Store a Discord guild in the database.
    async fn store_guild(&self, guild: &Guild) {
        let new_guild = NewGuild {
            id: Self::to_db_id(guild.id.get()),
            name: guild.name.clone(),
            icon: guild.icon.map(|i| i.to_string()),
            banner: guild.banner.map(|b| b.to_string()),
            splash: guild.splash.map(|s| s.to_string()),
            owner_id: Self::to_db_id(guild.owner_id.get()),
            features: Some(guild.features.iter().map(|f| Some(f.clone())).collect()),
            description: guild.description.clone(),
            vanity_url_code: guild.vanity_url_code.clone(),
            member_count: Some(guild.member_count as i32),
            approximate_member_count: guild.approximate_member_count.map(|c| c as i32),
            approximate_presence_count: guild.approximate_presence_count.map(|c| c as i32),
            afk_channel_id: guild.afk_channel_id.map(|id| Self::to_db_id(id.get())),
            afk_timeout: Some(guild.afk_timeout as i32),
            system_channel_id: guild.system_channel_id.map(|id| Self::to_db_id(id.get())),
            rules_channel_id: guild.rules_channel_id.map(|id| Self::to_db_id(id.get())),
            public_updates_channel_id: guild
                .public_updates_channel_id
                .map(|id| Self::to_db_id(id.get())),
            verification_level: Some(guild.verification_level as i16),
            explicit_content_filter: Some(guild.explicit_content_filter as i16),
            mfa_level: Some(guild.mfa_level as i16),
            premium_tier: Some(guild.premium_tier as i16),
            premium_subscription_count: guild.premium_subscription_count.map(|c| c as i32),
            max_presences: guild.max_presences.map(|c| c as i32),
            max_members: guild.max_members.map(|c| c as i32),
            max_video_channel_users: guild.max_video_channel_users.map(|c| c as i32),
            large: Some(guild.large),
            unavailable: Some(guild.unavailable),
            joined_at: guild.joined_at.map(|t| t.naive_utc()),
            left_at: None,
            bot_permissions: None,
            bot_active: Some(true),
        };

        match self.repository.store_guild(&new_guild).await {
            Ok(_) => {
                debug!(guild_id = %guild.id, guild_name = %guild.name, "Stored guild");
            }
            Err(e) => {
                error!(guild_id = %guild.id, error = %e, "Failed to store guild");
            }
        }
    }

    /// Store a Discord channel in the database.
    async fn store_channel(&self, guild_id: Option<GuildId>, channel: &Channel) {
        let (id, name, channel_type, position, topic, nsfw, parent_id) = match channel {
            Channel::Guild(gc) => (
                Self::to_db_id(gc.id.get()),
                Some(gc.name.clone()),
                Self::map_channel_type(gc.kind),
                Some(gc.position as i32),
                gc.topic.clone(),
                Some(gc.nsfw),
                gc.parent_id.map(|p| Self::to_db_id(p.get())),
            ),
            Channel::Private(dm) => (
                Self::to_db_id(dm.id.get()),
                None,
                ChannelType::Dm,
                None,
                None,
                None,
                None,
            ),
            _ => {
                warn!("Unsupported channel type for storage");
                return;
            }
        };

        let new_channel = NewChannel {
            id,
            guild_id: guild_id.map(|g| Self::to_db_id(g.get())),
            name,
            channel_type,
            position,
            topic,
            nsfw,
            rate_limit_per_user: None,
            bitrate: None,
            user_limit: None,
            parent_id,
            owner_id: None,
            message_count: None,
            member_count: None,
            archived: None,
            auto_archive_duration: None,
            archive_timestamp: None,
            locked: None,
            invitable: None,
            available_tags: None,
            default_reaction_emoji: None,
            default_thread_rate_limit: None,
            default_sort_order: None,
            default_forum_layout: None,
            last_message_at: None,
            last_read_message_id: None,
            bot_has_access: Some(true),
        };

        match self.repository.store_channel(&new_channel).await {
            Ok(_) => {
                debug!(channel_id = id, "Stored channel");
            }
            Err(e) => {
                error!(channel_id = id, error = %e, "Failed to store channel");
            }
        }
    }

    /// Store a Discord member in the database.
    async fn store_member(&self, guild_id: GuildId, member: &Member) {
        // First store the user
        let new_user = NewUser {
            id: Self::to_db_id(member.user.id.get()),
            username: member.user.name.clone(),
            discriminator: member.user.discriminator.map(|d| d.get().to_string()),
            global_name: member.user.global_name.clone(),
            avatar: member.user.avatar.map(|a| a.to_string()),
            bot: Some(member.user.bot),
            system: Some(member.user.system),
            mfa_enabled: None,
            banner: member.user.banner.map(|b| b.to_string()),
            accent_color: member.user.accent_colour.map(|c| c.0 as i32),
            locale: None,
            premium_type: None,
            public_flags: None,
        };

        if let Err(e) = self.repository.store_user(&new_user).await {
            error!(user_id = %member.user.id, error = %e, "Failed to store user");
            return;
        }

        // Then store the guild member
        let new_member = NewGuildMember {
            guild_id: Self::to_db_id(guild_id.get()),
            user_id: Self::to_db_id(member.user.id.get()),
            nick: member.nick.clone(),
            avatar: member.avatar.map(|a| a.to_string()),
            joined_at: member.joined_at.naive_utc(),
            premium_since: member.premium_since.map(|t| t.naive_utc()),
            deaf: Some(member.deaf),
            mute: Some(member.mute),
            pending: Some(member.pending),
            communication_disabled_until: member
                .communication_disabled_until
                .map(|t| t.naive_utc()),
        };

        match self.repository.store_guild_member(&new_member).await {
            Ok(_) => {
                debug!(
                    guild_id = %guild_id,
                    user_id = %member.user.id,
                    "Stored guild member"
                );
            }
            Err(e) => {
                error!(
                    guild_id = %guild_id,
                    user_id = %member.user.id,
                    error = %e,
                    "Failed to store guild member"
                );
            }
        }
    }

    /// Store a Discord role in the database.
    async fn store_role(&self, guild_id: GuildId, role: &Role) {
        let new_role = NewRole {
            id: Self::to_db_id(role.id.get()),
            guild_id: Self::to_db_id(guild_id.get()),
            name: role.name.clone(),
            color: role.colour.0 as i32,
            position: role.position as i32,
            permissions: role.permissions.bits() as i64,
            hoist: role.hoist,
            managed: Some(role.managed),
            mentionable: role.mentionable,
            icon: role.icon.map(|i| i.to_string()),
            unicode_emoji: role.unicode_emoji.clone(),
            tags: None,
        };

        match self.repository.store_role(&new_role).await {
            Ok(_) => {
                debug!(role_id = %role.id, role_name = %role.name, "Stored role");
            }
            Err(e) => {
                error!(role_id = %role.id, error = %e, "Failed to store role");
            }
        }
    }

    /// Map Serenity ChannelType to our ChannelType enum.
    fn map_channel_type(kind: serenity::model::channel::ChannelType) -> ChannelType {
        use serenity::model::channel::ChannelType as ST;
        match kind {
            ST::Text => ChannelType::GuildText,
            ST::Private => ChannelType::Dm,
            ST::Voice => ChannelType::GuildVoice,
            ST::GroupDm => ChannelType::GroupDm,
            ST::Category => ChannelType::GuildCategory,
            ST::News => ChannelType::GuildAnnouncement,
            ST::NewsThread => ChannelType::AnnouncementThread,
            ST::PublicThread => ChannelType::PublicThread,
            ST::PrivateThread => ChannelType::PrivateThread,
            ST::Stage => ChannelType::GuildStageVoice,
            ST::Directory => ChannelType::GuildDirectory,
            ST::Forum => ChannelType::GuildForum,
            ST::Media => ChannelType::GuildMedia,
            _ => {
                warn!("Unknown channel type: {:?}, defaulting to GuildText", kind);
                ChannelType::GuildText
            }
        }
    }
}

#[async_trait]
impl EventHandler for BoticelliHandler {
    /// Called when the bot successfully connects to Discord.
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!(
            bot_user = %ready.user.name,
            bot_id = %ready.user.id,
            guilds = ready.guilds.len(),
            "Bot connected to Discord"
        );

        // The guilds in Ready are partial, we'll get full data via guild_create events
        for guild in &ready.guilds {
            debug!(guild_id = %guild.id, "Bot is in guild");
        }
    }

    /// Called when a guild becomes available or the bot joins a guild.
    ///
    /// This is where we store the full guild data including channels, roles, and members.
    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
        info!(
            guild_id = %guild.id,
            guild_name = %guild.name,
            members = guild.members.len(),
            channels = guild.channels.len(),
            roles = guild.roles.len(),
            "Guild available"
        );

        // Store the guild
        self.store_guild(&guild).await;

        // Store all channels
        for channel in guild.channels.values() {
            self.store_channel(Some(guild.id), &Channel::Guild(channel.clone()))
                .await;
        }

        // Store all roles
        for role in guild.roles.values() {
            self.store_role(guild.id, role).await;
        }

        // Store all members
        for member in &guild.members {
            self.store_member(guild.id, member).await;
        }

        info!(guild_id = %guild.id, "Finished storing guild data");
    }

    /// Called when the bot leaves a guild or a guild becomes unavailable.
    async fn guild_delete(
        &self,
        _ctx: Context,
        incomplete: serenity::model::guild::UnavailableGuild,
        _full: Option<Guild>,
    ) {
        info!(guild_id = %incomplete.id, "Guild unavailable or left");

        if let Err(e) = self
            .repository
            .mark_guild_left(Self::to_db_id(incomplete.id.get()))
            .await
        {
            error!(guild_id = %incomplete.id, error = %e, "Failed to mark guild as left");
        }
    }

    /// Called when a channel is created.
    async fn channel_create(&self, _ctx: Context, channel: GuildChannel) {
        info!(
            channel_id = %channel.id,
            channel_name = %channel.name,
            "Channel created"
        );
        self.store_channel(channel.guild_id, &Channel::Guild(channel))
            .await;
    }

    /// Called when a new member joins a guild.
    async fn guild_member_addition(&self, _ctx: Context, new_member: Member) {
        info!(
            guild_id = %new_member.guild_id,
            user_id = %new_member.user.id,
            username = %new_member.user.name,
            "Member joined guild"
        );
        self.store_member(new_member.guild_id, &new_member).await;
    }

    /// Called when a member leaves a guild.
    async fn guild_member_removal(
        &self,
        _ctx: Context,
        guild_id: GuildId,
        user: serenity::model::user::User,
        _member_data_if_available: Option<Member>,
    ) {
        info!(
            guild_id = %guild_id,
            user_id = %user.id,
            username = %user.name,
            "Member left guild"
        );

        if let Err(e) = self
            .repository
            .mark_member_left(Self::to_db_id(guild_id.get()), Self::to_db_id(user.id.get()))
            .await
        {
            error!(
                guild_id = %guild_id,
                user_id = %user.id,
                error = %e,
                "Failed to mark member as left"
            );
        }
    }

    /// Called when a role is created.
    async fn guild_role_create(&self, _ctx: Context, new: Role) {
        info!(
            guild_id = %new.guild_id,
            role_id = %new.id,
            role_name = %new.name,
            "Role created"
        );
        self.store_role(new.guild_id, &new).await;
    }
}
