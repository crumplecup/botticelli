//! Serenity event handler for Discord bot.
//!
//! This module implements the EventHandler trait to respond to Discord events
//! and persist data to the database.

use crate::{
    ChannelType, DiscordRepository, NewChannelBuilder, NewGuildBuilder,
    NewGuildMemberBuilder, NewRoleBuilder, NewUserBuilder,
};
use botticelli_error::{DiscordError, DiscordErrorKind, DiscordErrorSeverity, DiscordResult};
use botticelli_interface::{DiscordEventProcessor, EventResult};
use chrono::NaiveDateTime;
use serenity::all::{GuildId, Ready};
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::Timestamp;
use serenity::model::channel::{Channel, GuildChannel};
use serenity::model::gateway::GatewayIntents;
use serenity::model::guild::{Guild, Member, Role};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

/// Convert Serenity Timestamp to Chrono NaiveDateTime
fn timestamp_to_naive(ts: &Timestamp) -> Option<NaiveDateTime> {
    chrono::DateTime::from_timestamp(ts.unix_timestamp(), 0).map(|dt| dt.naive_utc())
}

/// Event handler for the Botticelli Discord bot.
///
/// Implements Serenity's EventHandler trait to respond to Discord events
/// and persist data to the database via DiscordRepository.
///
/// Critical errors are sent to an error channel for the bot runtime to handle,
/// allowing graceful shutdown or recovery on database/connection failures.
pub struct BotticelliHandler {
    /// Repository for database operations
    repository: Arc<DiscordRepository>,
    /// Channel for sending critical errors back to bot runtime
    error_tx: mpsc::UnboundedSender<DiscordError>,
}

impl BotticelliHandler {
    /// Create a new BotticelliHandler with the given repository and error channel.
    ///
    /// # Arguments
    /// * `repository` - Database repository for Discord entities
    /// * `error_tx` - Channel sender for critical errors that should abort the bot
    pub fn new(
        repository: Arc<DiscordRepository>,
        error_tx: mpsc::UnboundedSender<DiscordError>,
    ) -> Self {
        Self {
            repository,
            error_tx,
        }
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
    #[instrument(skip(self, guild), fields(guild_id = %guild.id, guild_name = %guild.name))]
    async fn store_guild(&self, guild: &Guild) -> DiscordResult<()> {
        let new_guild = NewGuildBuilder::default()
            .id(Self::to_db_id(guild.id.get()))
            .name(guild.name.clone())
            .icon(guild.icon.as_ref().map(|i| i.to_string()))
            .banner(guild.banner.as_ref().map(|b| b.to_string()))
            .splash(guild.splash.as_ref().map(|s| s.to_string()))
            .owner_id(Self::to_db_id(guild.owner_id.get()))
            .features(Some(
                guild.features.iter().map(|f| Some(f.clone())).collect(),
            ))
            .description(guild.description.clone())
            .vanity_url_code(guild.vanity_url_code.clone())
            .member_count(Some(guild.member_count as i32))
            .approximate_member_count(guild.approximate_member_count.map(|c| c as i32))
            .approximate_presence_count(guild.approximate_presence_count.map(|c| c as i32))
            .afk_channel_id(
                guild
                    .afk_metadata
                    .as_ref()
                    .map(|afk| Self::to_db_id(afk.afk_channel_id.get())),
            )
            .afk_timeout(
                guild
                    .afk_metadata
                    .as_ref()
                    .map(|afk| u16::from(afk.afk_timeout) as i32),
            )
            .system_channel_id(guild.system_channel_id.map(|id| Self::to_db_id(id.get())))
            .rules_channel_id(guild.rules_channel_id.map(|id| Self::to_db_id(id.get())))
            .public_updates_channel_id(
                guild
                    .public_updates_channel_id
                    .map(|id| Self::to_db_id(id.get())),
            )
            .verification_level(Some(u8::from(guild.verification_level) as i16))
            .explicit_content_filter(Some(u8::from(guild.explicit_content_filter) as i16))
            .mfa_level(Some(u8::from(guild.mfa_level) as i16))
            .premium_tier(Some(u8::from(guild.premium_tier) as i16))
            .premium_subscription_count(guild.premium_subscription_count.map(|c| c as i32))
            .max_presences(guild.max_presences.map(|c| c as i32))
            .max_members(guild.max_members.map(|c| c as i32))
            .max_video_channel_users(guild.max_video_channel_users.map(|c| c as i32))
            .large(Some(guild.large))
            .unavailable(Some(guild.unavailable))
            .joined_at(timestamp_to_naive(&guild.joined_at))
            .bot_active(Some(true))
            .build();

        let new_guild = new_guild
            .map_err(|e| DiscordError::new(DiscordErrorKind::BuilderValidationError(e.to_string())))?;

        self.repository.store_guild(&new_guild).await?;
        debug!(guild_id = %guild.id, guild_name = %guild.name, "Stored guild");
        Ok(())
    }

    /// Store a Discord channel in the database.
    #[instrument(skip(self, guild_id, channel), fields(guild_id = ?guild_id))]
    async fn store_channel(
        &self,
        guild_id: Option<GuildId>,
        channel: &Channel,
    ) -> DiscordResult<()> {
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
                return Err(DiscordError::new(
                    DiscordErrorKind::UnsupportedType(
                        "Unsupported channel type for storage".to_string(),
                    ),
                ));
            }
        };

        let new_channel = NewChannelBuilder::default()
            .id(id)
            .guild_id(guild_id.map(|g| Self::to_db_id(g.get())))
            .name(name)
            .channel_type(channel_type)
            .position(position)
            .topic(topic)
            .nsfw(nsfw)
            .rate_limit_per_user(None)
            .bitrate(None)
            .user_limit(None)
            .parent_id(parent_id)
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
            .bot_has_access(Some(true))
            .build();

        let new_channel = match new_channel {
            Ok(channel) => channel,
            Err(e) => {
                error!(channel_id = id, error = %e, "Failed to build NewChannel");
                return Err(DiscordError::new(
                    DiscordErrorKind::BuilderValidationError(e.to_string())
                ));
            }
        };

        self.repository.store_channel(&new_channel).await?;
        debug!(channel_id = id, "Stored channel");
        Ok(())
    }

    /// Store a Discord member in the database.
    #[instrument(skip(self, guild_id, member), fields(guild_id = %guild_id, user_id = %member.user.id))]
    async fn store_member(&self, guild_id: GuildId, member: &Member) -> DiscordResult<()> {
        // First store the user
        let new_user = NewUserBuilder::default()
            .id(Self::to_db_id(member.user.id.get()))
            .username(member.user.name.clone())
            .discriminator(member.user.discriminator.map(|d| d.get().to_string()))
            .global_name(member.user.global_name.clone())
            .avatar(member.user.avatar.map(|a| a.to_string()))
            .bot(Some(member.user.bot))
            .system(Some(member.user.system))
            .mfa_enabled(None)
            .verified(None)
            .banner(member.user.banner.map(|b| b.to_string()))
            .accent_color(member.user.accent_colour.map(|c| c.0 as i32))
            .locale(None)
            .premium_type(None)
            .public_flags(None)
            .build();

        let new_user = match new_user {
            Ok(user) => user,
            Err(e) => {
                return Err(DiscordError::new(
                    DiscordErrorKind::BuilderValidationError(format!(
                        "Failed to build NewUser: {}",
                        e
                    )),
                ));
            }
        };

        self.repository.store_user(&new_user).await?;

        // Then store the guild member
        let new_member = NewGuildMemberBuilder::default()
            .guild_id(Self::to_db_id(guild_id.get()))
            .user_id(Self::to_db_id(member.user.id.get()))
            .nick(member.nick.clone())
            .avatar(member.avatar.map(|a| a.to_string()))
            .joined_at(
                member
                    .joined_at
                    .as_ref()
                    .and_then(timestamp_to_naive)
                    .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
            )
            .premium_since(member.premium_since.as_ref().and_then(timestamp_to_naive))
            .deaf(Some(member.deaf))
            .mute(Some(member.mute))
            .pending(Some(member.pending))
            .left_at(None)
            .communication_disabled_until(
                member
                    .communication_disabled_until
                    .as_ref()
                    .and_then(timestamp_to_naive),
            )
            .build();

        let new_member = new_member.map_err(|e| {
            DiscordError::new(DiscordErrorKind::BuilderValidationError(format!(
                "Failed to build NewGuildMember: {}",
                e
            )))
        })?;

        self.repository.store_guild_member(&new_member).await?;
        debug!(guild_id = %guild_id, user_id = %member.user.id, "Stored guild member");
        Ok(())
    }

    /// Store a Discord role in the database.
    #[instrument(skip(self, guild_id, role), fields(guild_id = %guild_id, role_id = %role.id, role_name = %role.name))]
    async fn store_role(&self, guild_id: GuildId, role: &Role) -> DiscordResult<()> {
        let new_role = NewRoleBuilder::default()
            .id(Self::to_db_id(role.id.get()))
            .guild_id(Self::to_db_id(guild_id.get()))
            .name(role.name.clone())
            .color(role.colour.0 as i32)
            .position(role.position as i32)
            .permissions(role.permissions.bits() as i64)
            .hoist(Some(role.hoist))
            .managed(Some(role.managed))
            .mentionable(Some(role.mentionable))
            .icon(role.icon.map(|i| i.to_string()))
            .unicode_emoji(role.unicode_emoji.clone())
            .tags(None)
            .build();

        let new_role = new_role.map_err(|e| {
            DiscordError::new(DiscordErrorKind::BuilderValidationError(format!(
                "Failed to build NewRole: {}",
                e
            )))
        })?;

        self.repository.store_role(&new_role).await?;
        debug!(role_id = %role.id, role_name = %role.name, "Stored role");
        Ok(())
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
            _ => {
                warn!("Unknown channel type: {:?}, defaulting to GuildText", kind);
                ChannelType::GuildText
            }
        }
    }
}

// Implementation of our internal event processing trait
#[async_trait]
impl DiscordEventProcessor for BotticelliHandler {
    type Error = DiscordError;
    type Severity = DiscordErrorSeverity;
    type Guild = serenity::model::guild::Guild;
    type Channel = serenity::model::channel::GuildChannel;
    type Member = serenity::model::guild::Member;
    type Role = serenity::model::guild::Role;
    type User = serenity::model::user::CurrentUser;

    fn error_severity(&self, error: &Self::Error) -> Self::Severity {
        error.severity()
    }

    fn error_is_retryable(&self, error: &Self::Error) -> bool {
        error.is_retryable()
    }

    fn error_context(&self, error: &Self::Error) -> String {
        error.error_context()
    }

    #[instrument(skip(self, guild), fields(guild_id = %guild.id, guild_name = %guild.name))]
    async fn process_guild_create(
        &self,
        guild: &Self::Guild,
        is_new: Option<bool>,
    ) -> EventResult<(), Self::Error> {
        debug!(is_new = ?is_new, "Processing guild_create event");

        // Critical: Store the guild first
        self.store_guild(guild).await?;

        // Best effort: Store all channels (collect errors)
        let mut channel_errors = Vec::new();
        for channel in guild.channels.values() {
            if let Err(e) = self
                .store_channel(Some(guild.id), &Channel::Guild(channel.clone()))
                .await
            {
                use botticelli_error::DiscordErrorSeverity;
                if self.error_severity(&e) == DiscordErrorSeverity::Critical {
                    return Err(e);
                }
                channel_errors.push((channel.id, e));
            }
        }

        // Best effort: Store all roles
        let mut role_errors = Vec::new();
        for role in guild.roles.values() {
            if let Err(e) = self.store_role(guild.id, role).await {
                use botticelli_error::DiscordErrorSeverity;
                if self.error_severity(&e) == DiscordErrorSeverity::Critical {
                    return Err(e);
                }
                role_errors.push((role.id, e));
            }
        }

        // Best effort: Store all members
        let mut member_errors = Vec::new();
        for member in guild.members.values() {
            if let Err(e) = self.store_member(guild.id, member).await {
                use botticelli_error::DiscordErrorSeverity;
                if self.error_severity(&e) == DiscordErrorSeverity::Critical {
                    return Err(e);
                }
                member_errors.push((member.user.id, e));
            }
        }

        // Log any warnings
        if !channel_errors.is_empty() {
            warn!(
                guild_id = %guild.id,
                failed_count = channel_errors.len(),
                "Some channels failed to store"
            );
        }
        if !role_errors.is_empty() {
            warn!(
                guild_id = %guild.id,
                failed_count = role_errors.len(),
                "Some roles failed to store"
            );
        }
        if !member_errors.is_empty() {
            warn!(
                guild_id = %guild.id,
                failed_count = member_errors.len(),
                "Some members failed to store"
            );
        }

        Ok(())
    }

    #[instrument(skip(self, channel), fields(channel_id = %channel.id, channel_name = %channel.name))]
    async fn process_channel_create(
        &self,
        channel: &Self::Channel,
    ) -> EventResult<(), Self::Error> {
        debug!("Processing channel_create event");
        self.store_channel(Some(channel.guild_id), &Channel::Guild(channel.clone()))
            .await
    }

    #[instrument(skip(self, member), fields(user_id = %member.user.id))]
    async fn process_member_add(&self, member: &Self::Member) -> EventResult<(), Self::Error> {
        debug!("Processing member_add event");
        self.store_member(member.guild_id, member).await
    }

    #[instrument(skip(self, role), fields(role_id = %role.id, role_name = %role.name))]
    async fn process_role_create(&self, role: &Self::Role) -> EventResult<(), Self::Error> {
        debug!("Processing role_create event");
        self.store_role(role.guild_id, role).await
    }

    #[instrument(skip(self, user), fields(user_id = %user.id, username = %user.name))]
    async fn process_ready(
        &self,
        user: &Self::User,
        guild_count: usize,
    ) -> EventResult<(), Self::Error> {
        info!(
            bot_user = %user.name,
            bot_id = %user.id,
            guilds = guild_count,
            "Bot connected to Discord"
        );
        Ok(())
    }
}

#[async_trait]
impl EventHandler for BotticelliHandler {
    /// Called when the bot successfully connects to Discord.
    #[instrument(skip(self, _ctx, ready), fields(user_id = %ready.user.id, username = %ready.user.name, guild_count = ready.guilds.len()))]
    async fn ready(&self, _ctx: Context, ready: Ready) {
        // The guilds in Ready are partial, we'll get full data via guild_create events
        for guild in &ready.guilds {
            debug!(guild_id = %guild.id, "Bot is in guild");
        }

        if let Err(e) = self.process_ready(&ready.user, ready.guilds.len()).await {
            let severity = self.error_severity(&e);
            let retryable = self.error_is_retryable(&e);
            let context = self.error_context(&e);

            error!(
                error = %e,
                context = %context,
                severity = ?severity,
                retryable = retryable,
                "Failed to process ready event"
            );

            // Send critical errors to runtime for handling
            use botticelli_error::DiscordErrorSeverity;
            if severity == DiscordErrorSeverity::Critical {
                if let Err(send_err) = self.error_tx.send(e) {
                    error!(error = %send_err, "Failed to send critical error to runtime");
                }
            }
        }
    }

    /// Called when a guild becomes available or the bot joins a guild.
    #[instrument(skip(self, _ctx, guild), fields(guild_id = %guild.id, guild_name = %guild.name, is_new = ?is_new))]
    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: Option<bool>) {
        info!(
            guild_id = %guild.id,
            guild_name = %guild.name,
            members = guild.members.len(),
            channels = guild.channels.len(),
            roles = guild.roles.len(),
            "Guild available"
        );

        if let Err(e) = self.process_guild_create(&guild, is_new).await {
            let severity = self.error_severity(&e);
            let retryable = self.error_is_retryable(&e);
            let context = self.error_context(&e);

            error!(
                guild_id = %guild.id,
                error = %e,
                context = %context,
                severity = ?severity,
                retryable = retryable,
                "Failed to process guild_create event"
            );

            // Send critical errors to runtime for handling
            use botticelli_error::DiscordErrorSeverity;
            if severity == DiscordErrorSeverity::Critical {
                if let Err(send_err) = self.error_tx.send(e) {
                    error!(error = %send_err, "Failed to send critical error to runtime");
                }
            }
        }
    }

    /// Called when the bot leaves a guild or a guild becomes unavailable.
    #[instrument(skip(self, _ctx, incomplete, _full), fields(guild_id = %incomplete.id))]
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
    #[instrument(skip(self, _ctx, channel), fields(channel_id = %channel.id, channel_name = %channel.name))]
    async fn channel_create(&self, _ctx: Context, channel: GuildChannel) {
        info!(
            channel_id = %channel.id,
            channel_name = %channel.name,
            "Channel created"
        );
        if let Err(e) = self.store_channel(Some(channel.guild_id), &Channel::Guild(channel)).await {
            error!(error = ?e, "Failed to store channel");
        }
    }

    /// Called when a new member joins a guild.
    #[instrument(skip(self, _ctx, new_member), fields(guild_id = %new_member.guild_id, user_id = %new_member.user.id))]
    async fn guild_member_addition(&self, _ctx: Context, new_member: Member) {
        info!(
            guild_id = %new_member.guild_id,
            user_id = %new_member.user.id,
            username = %new_member.user.name,
            "Member joined guild"
        );
        if let Err(e) = self.store_member(new_member.guild_id, &new_member).await {
            error!(error = ?e, "Failed to store member");
        }
    }

    /// Called when a member leaves a guild.
    #[instrument(skip(self, _ctx, user, _member_data_if_available), fields(guild_id = %guild_id, user_id = %user.id))]
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
            .mark_member_left(
                Self::to_db_id(guild_id.get()),
                Self::to_db_id(user.id.get()),
            )
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
    #[instrument(skip(self, _ctx, new), fields(guild_id = %new.guild_id, role_id = %new.id, role_name = %new.name))]
    async fn guild_role_create(&self, _ctx: Context, new: Role) {
        info!(
            guild_id = %new.guild_id,
            role_id = %new.id,
            role_name = %new.name,
            "Role created"
        );
        if let Err(e) = self.store_role(new.guild_id, &new).await {
            error!(error = ?e, "Failed to store role");
        }
    }
}
