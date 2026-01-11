//! Discord command executor implementation.

use super::channels::Channels;
use super::members::Members;
use super::messages::Messages;
use super::reactions::Reactions;
use super::roles::Roles;
use super::threads::Threads;
use super::{events, forum, misc, moderation, server};
use botticelli_error::{BotCommandError, BotCommandErrorKind};
use async_trait::async_trait;
use botticelli_interface::BotCommandExecutor;
use serde_json::Value as JsonValue;
use serenity::all::Http;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Discord command executor.
#[derive(Debug, Clone)]
pub struct DiscordCommandExecutor {
    http: Arc<Http>,
}

impl DiscordCommandExecutor {
    /// Create new Discord command executor with HTTP client.
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

#[async_trait]
impl BotCommandExecutor for DiscordCommandExecutor {
    type Error = BotCommandError;

    fn platform(&self) -> &str {
        "discord"
    }

    #[instrument(
        skip(self, args),
        fields(
            platform = "discord",
            command,
            arg_count = args.len(),
            result_size,
            duration_ms
        )
    )]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        info!("Executing Discord bot command");

        let start = std::time::Instant::now();

        let result = match command {
            // Server commands
            "server.get_stats" => server::get_stats(&self.http, args).await?,

            // Misc commands
            "emojis.list" => misc::emojis_list(&self.http, args).await?,
            "stickers.list" => misc::stickers_list(&self.http, args).await?,
            "invites.list" => misc::invites_list(&self.http, args).await?,
            "webhooks.list" => misc::webhooks_list(&self.http, args).await?,
            "integrations.list" => misc::integrations_list(&self.http, args).await?,
            "voice_regions.list" => misc::voice_regions_list(&self.http, args).await?,

            // Moderation commands
            "bans.list" => moderation::list(&self.http, args).await?,
            "members.ban" => moderation::ban(&self.http, args).await?,
            "members.unban" => moderation::unban(&self.http, args).await?,
            "members.kick" => moderation::kick(&self.http, args).await?,

            // Events commands
            "events.list" => events::list(&self.http, args).await?,
            "events.get" => events::get(&self.http, args).await?,
            "events.create" => events::create(&self.http, args).await?,
            "events.edit" => events::edit(&self.http, args).await?,
            "events.delete" => events::delete(&self.http, args).await?,

            // Forum commands
            "forum.create_post" => forum::create_post(&self.http, args).await?,
            "forum.list_posts" => forum::list_posts(&self.http, args).await?,
            "forum.get_post" => forum::get_post(&self.http, args).await?,

            // Reaction commands
            "reactions.add" => Reactions::add(&self.http, args).await?,
            "reactions.remove" => Reactions::remove(&self.http, args).await?,
            "reactions.list" => Reactions::list(&self.http, args).await?,
            "reactions.clear" => Reactions::clear(&self.http, args).await?,
            "reactions.clear_emoji" => Reactions::clear_emoji(&self.http, args).await?,

            // Role commands
            "roles.list" => Roles::list(&self.http, args).await?,
            "roles.get" => Roles::get(&self.http, args).await?,
            "roles.create" => Roles::create(&self.http, args).await?,
            "roles.edit" => Roles::edit(&self.http, args).await?,
            "roles.delete" => Roles::delete(&self.http, args).await?,
            "roles.assign" => Roles::assign(&self.http, args).await?,
            "roles.remove" => Roles::remove(&self.http, args).await?,

            // Member commands (non-moderation)
            "members.list" => Members::list(&self.http, args).await?,
            "members.get" => Members::get(&self.http, args).await?,
            "members.edit" => Members::edit(&self.http, args).await?,
            "members.timeout" => Members::timeout(&self.http, args).await?,
            "members.remove_timeout" => Members::remove_timeout(&self.http, args).await?,

            // Channel commands
            "channels.list" => Channels::list(&self.http, args).await?,
            "channels.get" => Channels::get(&self.http, args).await?,
            "channels.create" => Channels::create(&self.http, args).await?,
            "channels.edit" => Channels::edit(&self.http, args).await?,
            "channels.delete" => Channels::delete(&self.http, args).await?,
            "channels.get_or_create" => Channels::get_or_create(&self.http, args).await?,
            "channels.create_invite" => Channels::create_invite(&self.http, args).await?,
            "channels.typing" => Channels::typing(&self.http, args).await?,

            // Message commands
            "messages.send" => Messages::send(&self.http, args).await?,
            "messages.get" => Messages::get(&self.http, args).await?,
            "messages.list" => Messages::list(&self.http, args).await?,
            "messages.edit" => Messages::edit(&self.http, args).await?,
            "messages.delete" => Messages::delete(&self.http, args).await?,
            "messages.pin" => Messages::pin(&self.http, args).await?,
            "messages.unpin" => Messages::unpin(&self.http, args).await?,
            "messages.bulk_delete" => Messages::bulk_delete(&self.http, args).await?,
            "messages.clear" => Messages::clear(&self.http, args).await?,

            // Thread commands
            "threads.create" => Threads::create(&self.http, args).await?,
            "threads.list" => Threads::list(&self.http, args).await?,
            "threads.get" => Threads::get(&self.http, args).await?,
            "threads.edit" => Threads::edit(&self.http, args).await?,
            "threads.delete" => Threads::delete(&self.http, args).await?,
            "threads.join" => Threads::join(&self.http, args).await?,
            "threads.leave" => Threads::leave(&self.http, args).await?,
            "threads.add_member" => Threads::add_member(&self.http, args).await?,
            "threads.remove_member" => Threads::remove_member(&self.http, args).await?,
            
            _ => {
                error!(
                    command,
                    "Command not found or not yet migrated"
                );
                return Err(BotCommandErrorKind::CommandNotFound(command.to_string()).into());
            }
        };

        let duration_ms = start.elapsed().as_millis();
        let result_size = serde_json::to_string(&result).map(|s| s.len()).unwrap_or(0);

        tracing::Span::current().record("duration_ms", duration_ms);
        tracing::Span::current().record("result_size", result_size);
        info!(
            duration_ms,
            result_size, "Discord command executed successfully"
        );

        Ok(result)
    }

    #[instrument(skip(self, command))]
    fn supports_command(&self, command: &str) -> bool {
        matches!(
            command,
            // Server
            "server.get_stats"
            // Misc
            | "emojis.list"
            | "stickers.list"
            | "invites.list"
            | "webhooks.list"
            | "integrations.list"
            | "voice_regions.list"
            // Moderation
            | "bans.list"
            | "members.ban"
            | "members.unban"
            | "members.kick"
            // Events
            | "events.list"
            | "events.get"
            | "events.create"
            | "events.edit"
            | "events.delete"
            // Forum
            | "forum.create_post"
            | "forum.list_posts"
            | "forum.get_post"
            // Reactions
            | "reactions.add"
            | "reactions.remove"
            | "reactions.list"
            | "reactions.clear"
            | "reactions.clear_emoji"
            // Roles
            | "roles.list"
            | "roles.get"
            | "roles.create"
            | "roles.edit"
            | "roles.delete"
            | "roles.assign"
            | "roles.remove"
            // Members (non-moderation)
            | "members.list"
            | "members.get"
            | "members.edit"
            | "members.timeout"
            | "members.remove_timeout"
            // Channels
            | "channels.list"
            | "channels.get"
            | "channels.create"
            | "channels.edit"
            | "channels.delete"
            | "channels.get_or_create"
            | "channels.create_invite"
            | "channels.typing"
            // Messages
            | "messages.send"
            | "messages.get"
            | "messages.list"
            | "messages.edit"
            | "messages.delete"
            | "messages.pin"
            | "messages.unpin"
            | "messages.bulk_delete"
            | "messages.clear"
            // Threads
            | "threads.create"
            | "threads.list"
            | "threads.get"
            | "threads.edit"
            | "threads.delete"
            | "threads.join"
            | "threads.leave"
            | "threads.add_member"
            | "threads.remove_member"
        )
    }

    #[instrument(skip(self))]
    fn supported_commands(&self) -> Vec<String> {
        vec![
            // Server
            "server.get_stats".to_string(),
            // Misc
            "emojis.list".to_string(),
            "stickers.list".to_string(),
            "invites.list".to_string(),
            "webhooks.list".to_string(),
            "integrations.list".to_string(),
            "voice_regions.list".to_string(),
            // Moderation
            "bans.list".to_string(),
            "members.ban".to_string(),
            "members.unban".to_string(),
            "members.kick".to_string(),
            // Events
            "events.list".to_string(),
            "events.get".to_string(),
            "events.create".to_string(),
            "events.edit".to_string(),
            "events.delete".to_string(),
            // Forum
            "forum.create_post".to_string(),
            "forum.list_posts".to_string(),
            "forum.get_post".to_string(),
            // Reactions
            "reactions.add".to_string(),
            "reactions.remove".to_string(),
            "reactions.list".to_string(),
            "reactions.clear".to_string(),
            "reactions.clear_emoji".to_string(),
            // Roles
            "roles.list".to_string(),
            "roles.get".to_string(),
            "roles.create".to_string(),
            "roles.edit".to_string(),
            "roles.delete".to_string(),
            "roles.assign".to_string(),
            "roles.remove".to_string(),
            // Members (non-moderation)
            "members.list".to_string(),
            "members.get".to_string(),
            "members.edit".to_string(),
            "members.timeout".to_string(),
            "members.remove_timeout".to_string(),
            // Channels
            "channels.list".to_string(),
            "channels.get".to_string(),
            "channels.create".to_string(),
            "channels.edit".to_string(),
            "channels.delete".to_string(),
            "channels.get_or_create".to_string(),
            "channels.create_invite".to_string(),
            "channels.typing".to_string(),
            // Messages
            "messages.send".to_string(),
            "messages.get".to_string(),
            "messages.list".to_string(),
            "messages.edit".to_string(),
            "messages.delete".to_string(),
            "messages.pin".to_string(),
            "messages.unpin".to_string(),
            "messages.bulk_delete".to_string(),
            "messages.clear".to_string(),
            // Threads
            "threads.create".to_string(),
            "threads.list".to_string(),
            "threads.get".to_string(),
            "threads.edit".to_string(),
            "threads.delete".to_string(),
            "threads.join".to_string(),
            "threads.leave".to_string(),
            "threads.add_member".to_string(),
            "threads.remove_member".to_string(),
        ]
    }

    #[instrument(skip(self, command))]
    fn command_help(&self, command: &str) -> Option<String> {
        let help: &'static str = match command {
            "server.get_stats" => 
                "Get server statistics (member count, channels, etc.)\n\
                 Required arguments: guild_id",
            "emojis.list" => "List custom emojis\nRequired arguments: guild_id",
            "stickers.list" => "List custom stickers\nRequired arguments: guild_id",
            "invites.list" => "List active invites\nRequired arguments: guild_id",
            "webhooks.list" => "List webhooks\nRequired arguments: guild_id",
            "integrations.list" => "List integrations\nRequired arguments: guild_id",
            "voice_regions.list" => "List voice regions\nRequired arguments: guild_id",
            "bans.list" => "List banned users\nRequired arguments: guild_id\nOptional: limit",
            "members.ban" => "Ban a member\nRequired arguments: guild_id, user_id\nOptional: delete_message_days",
            "members.unban" => "Unban a member\nRequired arguments: guild_id, user_id",
            "members.kick" => "Kick a member\nRequired arguments: guild_id, user_id\nOptional: reason",
            "events.list" => "List scheduled events\nRequired arguments: guild_id",
            "events.get" => "Get a scheduled event\nRequired arguments: guild_id, event_id",
            "events.create" => "Create a scheduled event\nRequired arguments: guild_id, name, start_time, entity_type\nOptional: description, end_time, location, channel_id",
            "events.edit" => "Edit a scheduled event\nRequired arguments: guild_id, event_id\nOptional: name, description, start_time, end_time, status",
            "events.delete" => "Delete a scheduled event\nRequired arguments: guild_id, event_id",
            "forum.create_post" => "Create a forum post\nRequired arguments: channel_id, name, content\nOptional: auto_archive_duration",
            "forum.list_posts" => "List forum posts\nRequired arguments: channel_id",
            "forum.get_post" => "Get forum post details\nRequired arguments: thread_id",
            "reactions.add" => "Add reaction to message\nRequired arguments: channel_id, message_id, emoji",
            "reactions.remove" => "Remove reaction from message\nRequired arguments: channel_id, message_id, emoji, user_id",
            "reactions.list" => "List users who reacted\nRequired arguments: channel_id, message_id, emoji\nOptional: limit",
            "reactions.clear" => "Clear all reactions\nRequired arguments: channel_id, message_id",
            "reactions.clear_emoji" => "Clear specific emoji reactions\nRequired arguments: channel_id, message_id, emoji",
            "roles.list" => "List all roles in guild\nRequired arguments: guild_id",
            "roles.get" => "Get role details\nRequired arguments: guild_id, role_id",
            "roles.create" => "Create new role\nRequired arguments: guild_id, name\nOptional: color, hoist, mentionable",
            "roles.edit" => "Edit role properties\nRequired arguments: guild_id, role_id\nOptional: name, color, hoist, mentionable",
            "roles.delete" => "Delete role\nRequired arguments: guild_id, role_id",
            "roles.assign" => "Assign role to member\nRequired arguments: guild_id, user_id, role_id",
            "roles.remove" => "Remove role from member\nRequired arguments: guild_id, user_id, role_id",
            "members.list" => "List guild members\nRequired arguments: guild_id\nOptional: limit (max 1000)",
            "members.get" => "Get member details\nRequired arguments: guild_id, user_id",
            "members.edit" => "Edit member properties\nRequired arguments: guild_id, user_id\nOptional: nickname, mute, deafen, roles",
            "members.timeout" => "Timeout member\nRequired arguments: guild_id, user_id, duration_seconds (max 28 days)",
            "members.remove_timeout" => "Remove member timeout\nRequired arguments: guild_id, user_id",
            "channels.list" => "List all channels\nRequired arguments: guild_id",
            "channels.get" => "Get channel details\nRequired arguments: guild_id, channel_id",
            "channels.create" => "Create new channel\nRequired arguments: guild_id, name, kind\nOptional: topic, position, nsfw",
            "channels.edit" => "Edit channel properties\nRequired arguments: channel_id\nOptional: name, topic, nsfw, position, bitrate, user_limit",
            "channels.delete" => "Delete channel\nRequired arguments: guild_id, channel_id",
            "channels.get_or_create" => "Get or create channel\nRequired arguments: guild_id, name\nOptional: channel_type, topic, position, nsfw",
            "channels.create_invite" => "Create invite link\nRequired arguments: channel_id\nOptional: max_age, max_uses, temporary",
            "channels.typing" => "Trigger typing indicator\nRequired arguments: channel_id",
            "messages.send" => "Send message\nRequired arguments: channel_id, content\nOptional: tts",
            "messages.get" => "Get message details\nRequired arguments: channel_id, message_id",
            "messages.list" => "List messages\nRequired arguments: channel_id\nOptional: limit (max 100)",
            "messages.edit" => "Edit message\nRequired arguments: channel_id, message_id, content",
            "messages.delete" => "Delete message\nRequired arguments: channel_id, message_id\nOptional: reason",
            "messages.pin" => "Pin message\nRequired arguments: channel_id, message_id",
            "messages.unpin" => "Unpin message\nRequired arguments: channel_id, message_id",
            "messages.bulk_delete" => "Bulk delete messages\nRequired arguments: channel_id, message_ids (array, max 100)",
            "messages.clear" => "Clear messages from channel\nRequired arguments: channel_id\nOptional: limit (max 100)",
            "threads.create" => "Create thread\nRequired arguments: channel_id, name\nOptional: message_id, kind, auto_archive_duration, invitable",
            "threads.list" => "List active threads\nRequired arguments: guild_id",
            "threads.get" => "Get thread details\nRequired arguments: channel_id",
            "threads.edit" => "Edit thread properties\nRequired arguments: channel_id\nOptional: name, archived, auto_archive_duration, locked, invitable",
            "threads.delete" => "Delete thread\nRequired arguments: channel_id",
            "threads.join" => "Join thread\nRequired arguments: channel_id",
            "threads.leave" => "Leave thread\nRequired arguments: channel_id",
            "threads.add_member" => "Add member to thread\nRequired arguments: channel_id, user_id",
            "threads.remove_member" => "Remove member from thread\nRequired arguments: channel_id, user_id",
            _ => return None,
        };
        Some(help.to_string())
    }
}
