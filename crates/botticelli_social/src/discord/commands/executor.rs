//! Discord command executor implementation.

use super::{channels, events, forum, members, messages, misc, moderation, reactions, roles, server, threads};
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
            "reactions.add" => reactions::add(&self.http, args).await?,
            "reactions.remove" => reactions::remove(&self.http, args).await?,
            "reactions.list" => reactions::list(&self.http, args).await?,
            "reactions.clear" => reactions::clear(&self.http, args).await?,
            "reactions.clear_emoji" => reactions::clear_emoji(&self.http, args).await?,

            // Role commands
            "roles.list" => roles::list(&self.http, args).await?,
            "roles.get" => roles::get(&self.http, args).await?,
            "roles.create" => roles::create(&self.http, args).await?,
            "roles.edit" => roles::edit(&self.http, args).await?,
            "roles.delete" => roles::delete(&self.http, args).await?,
            "roles.assign" => roles::assign(&self.http, args).await?,
            "roles.remove" => roles::remove(&self.http, args).await?,

            // Member commands (non-moderation)
            "members.list" => members::list(&self.http, args).await?,
            "members.get" => members::get(&self.http, args).await?,
            "members.edit" => members::edit(&self.http, args).await?,
            "members.timeout" => members::timeout(&self.http, args).await?,
            "members.remove_timeout" => members::remove_timeout(&self.http, args).await?,

            // Channel commands
            "channels.list" => channels::list(&self.http, args).await?,
            "channels.get" => channels::get(&self.http, args).await?,
            "channels.create" => channels::create(&self.http, args).await?,
            "channels.edit" => channels::edit(&self.http, args).await?,
            "channels.delete" => channels::delete(&self.http, args).await?,
            "channels.get_or_create" => channels::get_or_create(&self.http, args).await?,
            "channels.create_invite" => channels::create_invite(&self.http, args).await?,
            "channels.typing" => channels::typing(&self.http, args).await?,

            // Message commands
            "messages.send" => messages::send(&self.http, args).await?,
            "messages.get" => messages::get(&self.http, args).await?,
            "messages.list" => messages::list(&self.http, args).await?,
            "messages.edit" => messages::edit(&self.http, args).await?,
            "messages.delete" => messages::delete(&self.http, args).await?,
            "messages.pin" => messages::pin(&self.http, args).await?,
            "messages.unpin" => messages::unpin(&self.http, args).await?,
            "messages.bulk_delete" => messages::bulk_delete(&self.http, args).await?,
            "messages.clear" => messages::clear(&self.http, args).await?,

            // Thread commands
            "threads.create" => threads::create(&self.http, args).await?,
            "threads.list" => threads::list(&self.http, args).await?,
            "threads.get" => threads::get(&self.http, args).await?,
            "threads.edit" => threads::edit(&self.http, args).await?,
            "threads.delete" => threads::delete(&self.http, args).await?,
            "threads.join" => threads::join(&self.http, args).await?,
            "threads.leave" => threads::leave(&self.http, args).await?,
            "threads.add_member" => threads::add_member(&self.http, args).await?,
            "threads.remove_member" => threads::remove_member(&self.http, args).await?,
            
            _ => {
                error!(
                    command,
                    "Command not found or not yet migrated"
                );
                return Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                    command.to_string(),
                )));
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

    fn command_help(&self, command: &str) -> Option<String> {
        match command {
            "server.get_stats" => Some(
                "Get server statistics (member count, channels, etc.)\n\
                 Required arguments: guild_id"
                    .to_string(),
            ),
            "emojis.list" => Some("List custom emojis\nRequired arguments: guild_id".to_string()),
            "stickers.list" => Some("List custom stickers\nRequired arguments: guild_id".to_string()),
            "invites.list" => Some("List active invites\nRequired arguments: guild_id".to_string()),
            "webhooks.list" => Some("List webhooks\nRequired arguments: guild_id".to_string()),
            "integrations.list" => Some("List integrations\nRequired arguments: guild_id".to_string()),
            "voice_regions.list" => Some("List voice regions\nRequired arguments: guild_id".to_string()),
            "bans.list" => Some("List banned users\nRequired arguments: guild_id\nOptional: limit".to_string()),
            "members.ban" => Some("Ban a member\nRequired arguments: guild_id, user_id\nOptional: delete_message_days".to_string()),
            "members.unban" => Some("Unban a member\nRequired arguments: guild_id, user_id".to_string()),
            "members.kick" => Some("Kick a member\nRequired arguments: guild_id, user_id\nOptional: reason".to_string()),
            "events.list" => Some("List scheduled events\nRequired arguments: guild_id".to_string()),
            "events.get" => Some("Get a scheduled event\nRequired arguments: guild_id, event_id".to_string()),
            "events.create" => Some("Create a scheduled event\nRequired arguments: guild_id, name, start_time, entity_type\nOptional: description, end_time, location, channel_id".to_string()),
            "events.edit" => Some("Edit a scheduled event\nRequired arguments: guild_id, event_id\nOptional: name, description, start_time, end_time, status".to_string()),
            "events.delete" => Some("Delete a scheduled event\nRequired arguments: guild_id, event_id".to_string()),
            "forum.create_post" => Some("Create a forum post\nRequired arguments: channel_id, name, content\nOptional: auto_archive_duration".to_string()),
            "forum.list_posts" => Some("List forum posts\nRequired arguments: channel_id".to_string()),
            "forum.get_post" => Some("Get forum post details\nRequired arguments: thread_id".to_string()),
            "reactions.add" => Some("Add reaction to message\nRequired arguments: channel_id, message_id, emoji".to_string()),
            "reactions.remove" => Some("Remove reaction from message\nRequired arguments: channel_id, message_id, emoji, user_id".to_string()),
            "reactions.list" => Some("List users who reacted\nRequired arguments: channel_id, message_id, emoji\nOptional: limit".to_string()),
            "reactions.clear" => Some("Clear all reactions\nRequired arguments: channel_id, message_id".to_string()),
            "reactions.clear_emoji" => Some("Clear specific emoji reactions\nRequired arguments: channel_id, message_id, emoji".to_string()),
            "roles.list" => Some("List all roles in guild\nRequired arguments: guild_id".to_string()),
            "roles.get" => Some("Get role details\nRequired arguments: guild_id, role_id".to_string()),
            "roles.create" => Some("Create new role\nRequired arguments: guild_id, name\nOptional: color, hoist, mentionable".to_string()),
            "roles.edit" => Some("Edit role properties\nRequired arguments: guild_id, role_id\nOptional: name, color, hoist, mentionable".to_string()),
            "roles.delete" => Some("Delete role\nRequired arguments: guild_id, role_id".to_string()),
            "roles.assign" => Some("Assign role to member\nRequired arguments: guild_id, user_id, role_id".to_string()),
            "roles.remove" => Some("Remove role from member\nRequired arguments: guild_id, user_id, role_id".to_string()),
            "members.list" => Some("List guild members\nRequired arguments: guild_id\nOptional: limit (max 1000)".to_string()),
            "members.get" => Some("Get member details\nRequired arguments: guild_id, user_id".to_string()),
            "members.edit" => Some("Edit member properties\nRequired arguments: guild_id, user_id\nOptional: nickname, mute, deafen, roles".to_string()),
            "members.timeout" => Some("Timeout member\nRequired arguments: guild_id, user_id, duration_seconds (max 28 days)".to_string()),
            "members.remove_timeout" => Some("Remove member timeout\nRequired arguments: guild_id, user_id".to_string()),
            "channels.list" => Some("List all channels\nRequired arguments: guild_id".to_string()),
            "channels.get" => Some("Get channel details\nRequired arguments: guild_id, channel_id".to_string()),
            "channels.create" => Some("Create new channel\nRequired arguments: guild_id, name, kind\nOptional: topic, position, nsfw".to_string()),
            "channels.edit" => Some("Edit channel properties\nRequired arguments: channel_id\nOptional: name, topic, nsfw, position, bitrate, user_limit".to_string()),
            "channels.delete" => Some("Delete channel\nRequired arguments: guild_id, channel_id".to_string()),
            "channels.get_or_create" => Some("Get or create channel\nRequired arguments: guild_id, name\nOptional: channel_type, topic, position, nsfw".to_string()),
            "channels.create_invite" => Some("Create invite link\nRequired arguments: channel_id\nOptional: max_age, max_uses, temporary".to_string()),
            "channels.typing" => Some("Trigger typing indicator\nRequired arguments: channel_id".to_string()),
            "messages.send" => Some("Send message\nRequired arguments: channel_id, content\nOptional: tts".to_string()),
            "messages.get" => Some("Get message details\nRequired arguments: channel_id, message_id".to_string()),
            "messages.list" => Some("List messages\nRequired arguments: channel_id\nOptional: limit (max 100)".to_string()),
            "messages.edit" => Some("Edit message\nRequired arguments: channel_id, message_id, content".to_string()),
            "messages.delete" => Some("Delete message\nRequired arguments: channel_id, message_id\nOptional: reason".to_string()),
            "messages.pin" => Some("Pin message\nRequired arguments: channel_id, message_id".to_string()),
            "messages.unpin" => Some("Unpin message\nRequired arguments: channel_id, message_id".to_string()),
            "messages.bulk_delete" => Some("Bulk delete messages\nRequired arguments: channel_id, message_ids (array, max 100)".to_string()),
            "messages.clear" => Some("Clear messages from channel\nRequired arguments: channel_id\nOptional: limit (max 100)".to_string()),
            "threads.create" => Some("Create thread\nRequired arguments: channel_id, name\nOptional: message_id, kind, auto_archive_duration, invitable".to_string()),
            "threads.list" => Some("List active threads\nRequired arguments: guild_id".to_string()),
            "threads.get" => Some("Get thread details\nRequired arguments: channel_id".to_string()),
            "threads.edit" => Some("Edit thread properties\nRequired arguments: channel_id\nOptional: name, archived, auto_archive_duration, locked, invitable".to_string()),
            "threads.delete" => Some("Delete thread\nRequired arguments: channel_id".to_string()),
            "threads.join" => Some("Join thread\nRequired arguments: channel_id".to_string()),
            "threads.leave" => Some("Leave thread\nRequired arguments: channel_id".to_string()),
            "threads.add_member" => Some("Add member to thread\nRequired arguments: channel_id, user_id".to_string()),
            "threads.remove_member" => Some("Remove member from thread\nRequired arguments: channel_id, user_id".to_string()),
            _ => None,
        }
    }
}
