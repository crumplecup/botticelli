//! ActProcessor implementations for Discord data types.
//!
//! This module provides processors that extract Discord data from LLM responses
//! and insert them into the database using the DiscordRepository.

use botticelli_error::BotticelliResult;
use botticelli_narrative::{ActProcessor, ProcessorContext};
use botticelli_narrative::extraction::{extract_json, parse_json};

use crate::{
    DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson, DiscordMemberRoleJson,
    DiscordRepository, DiscordRoleJson, DiscordUserJson, NewChannel, NewGuild, NewGuildMember,
    NewMemberRole, NewRole, NewUser,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Processor for Discord guild (server) data.
///
/// Extracts guild JSON from LLM responses and stores in the database.
/// Handles both single guild objects and arrays.
pub struct DiscordGuildProcessor {
    repository: Arc<DiscordRepository>,
}

impl DiscordGuildProcessor {
    /// Create a new guild processor.
    #[instrument(skip(repository))]
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordGuildProcessor {
    #[instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let json_str = extract_json(&context.execution.response)?;

        // Try parsing as array first, then single object
        let guilds: Vec<DiscordGuildJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(
            act = %context.execution.act_name,
            count = guilds.len(),
            "Processing Discord guilds"
        );

        for guild_json in guilds {
            tracing::debug!(
                guild_id = guild_json.id,
                guild_name = %guild_json.name,
                "Storing Discord guild"
            );

            let new_guild: NewGuild = guild_json.try_into()?;
            self.repository.store_guild(&new_guild).await?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            "Discord guilds stored successfully"
        );
        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        // Process if act name suggests guild/server data
        let name_lower = context.execution.act_name.to_lowercase();
        let name_match = name_lower.contains("guild") || name_lower.contains("server");

        // Or if response contains owner_id field (unique to guilds)
        let content_match = context.execution.response.contains("\"owner_id\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordGuildProcessor"
    }
}

/// Processor for Discord user data.
///
/// Extracts user JSON from LLM responses and stores in the database.
/// Handles both single users and arrays.
pub struct DiscordUserProcessor {
    repository: Arc<DiscordRepository>,
}

impl DiscordUserProcessor {
    /// Create a new user processor.
    #[instrument(skip(repository))]
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordUserProcessor {
    #[instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let json_str = extract_json(&context.execution.response)?;

        let users: Vec<DiscordUserJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(
            act = %context.execution.act_name,
            count = users.len(),
            "Processing Discord users"
        );

        for user_json in users {
            tracing::debug!(
                user_id = user_json.id,
                username = %user_json.username,
                "Storing Discord user"
            );

            let new_user: NewUser = user_json.try_into()?;
            self.repository.store_user(&new_user).await?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            "Discord users stored successfully"
        );
        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        let name_lower = context.execution.act_name.to_lowercase();
        let name_match = name_lower.contains("user") || name_lower.contains("member");

        // Users have username field, members have user_id
        let content_match = context.execution.response.contains("\"username\"")
            && !context.execution.response.contains("\"user_id\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordUserProcessor"
    }
}

/// Processor for Discord channel data.
///
/// Extracts channel JSON from LLM responses and stores in the database.
/// Handles both single channels and arrays.
pub struct DiscordChannelProcessor {
    repository: Arc<DiscordRepository>,
}

impl DiscordChannelProcessor {
    /// Create a new channel processor.
    #[instrument(skip(repository))]
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordChannelProcessor {
    #[instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let json_str = extract_json(&context.execution.response)?;

        let channels: Vec<DiscordChannelJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(
            act = %context.execution.act_name,
            count = channels.len(),
            "Processing Discord channels"
        );

        for channel_json in channels {
            tracing::debug!(
                channel_id = channel_json.id,
                channel_type = %channel_json.channel_type,
                channel_name = ?channel_json.name,
                "Storing Discord channel"
            );

            let new_channel: NewChannel = channel_json.try_into()?;
            self.repository.store_channel(&new_channel).await?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            "Discord channels stored successfully"
        );
        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        let name_lower = context.execution.act_name.to_lowercase();
        let name_match = name_lower.contains("channel");

        // Channels have channel_type field
        let content_match = context.execution.response.contains("\"channel_type\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordChannelProcessor"
    }
}

/// Processor for Discord role data.
///
/// Extracts role JSON from LLM responses and stores in the database.
/// Handles both single roles and arrays.
pub struct DiscordRoleProcessor {
    repository: Arc<DiscordRepository>,
}

impl DiscordRoleProcessor {
    /// Create a new role processor.
    #[instrument(skip(repository))]
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordRoleProcessor {
    #[instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let json_str = extract_json(&context.execution.response)?;

        let roles: Vec<DiscordRoleJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(
            act = %context.execution.act_name,
            count = roles.len(),
            "Processing Discord roles"
        );

        for role_json in roles {
            tracing::debug!(
                role_id = role_json.id,
                role_name = %role_json.name,
                guild_id = role_json.guild_id,
                "Storing Discord role"
            );

            let new_role: NewRole = role_json.try_into()?;
            self.repository.store_role(&new_role).await?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            "Discord roles stored successfully"
        );
        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        let name_lower = context.execution.act_name.to_lowercase();
        let name_match = name_lower.contains("role");

        // Roles have permissions and position fields
        let content_match = context.execution.response.contains("\"permissions\"")
            && context.execution.response.contains("\"position\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordRoleProcessor"
    }
}

/// Processor for Discord guild member data.
///
/// Extracts guild member JSON from LLM responses and stores in the database.
/// Handles both single members and arrays.
pub struct DiscordGuildMemberProcessor {
    repository: Arc<DiscordRepository>,
}

impl DiscordGuildMemberProcessor {
    /// Create a new guild member processor.
    #[instrument(skip(repository))]
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordGuildMemberProcessor {
    #[instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let json_str = extract_json(&context.execution.response)?;

        let members: Vec<DiscordGuildMemberJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(
            act = %context.execution.act_name,
            count = members.len(),
            "Processing Discord guild members"
        );

        for member_json in members {
            tracing::debug!(
                guild_id = member_json.guild_id,
                user_id = member_json.user_id,
                nick = ?member_json.nick,
                "Storing Discord guild member"
            );

            let new_member: NewGuildMember = member_json.try_into()?;
            self.repository.store_guild_member(&new_member).await?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            "Discord guild members stored successfully"
        );
        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        let name_lower = context.execution.act_name.to_lowercase();
        let name_match = name_lower.contains("member") && !name_lower.contains("role");

        // Guild members have both guild_id and user_id, plus joined_at
        let content_match = context.execution.response.contains("\"guild_id\"")
            && context.execution.response.contains("\"user_id\"")
            && context.execution.response.contains("\"joined_at\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordGuildMemberProcessor"
    }
}

/// Processor for Discord member role assignments.
///
/// Extracts member role JSON from LLM responses and stores in the database.
/// Handles both single role assignments and arrays.
pub struct DiscordMemberRoleProcessor {
    repository: Arc<DiscordRepository>,
}

impl DiscordMemberRoleProcessor {
    /// Create a new member role processor.
    #[instrument(skip(repository))]
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordMemberRoleProcessor {
    #[instrument(skip(self, context), fields(act = %context.execution.act_name))]
    async fn process(&self, context: &ProcessorContext<'_>) -> BotticelliResult<()> {
        let json_str = extract_json(&context.execution.response)?;

        let member_roles: Vec<DiscordMemberRoleJson> = if json_str.trim().starts_with('[') {
            parse_json(&json_str)?
        } else {
            vec![parse_json(&json_str)?]
        };

        tracing::info!(
            act = %context.execution.act_name,
            count = member_roles.len(),
            "Processing Discord member role assignments"
        );

        for member_role_json in member_roles {
            tracing::debug!(
                guild_id = member_role_json.guild_id,
                user_id = member_role_json.user_id,
                role_id = member_role_json.role_id,
                "Storing Discord member role assignment"
            );

            let new_member_role: NewMemberRole = member_role_json.try_into()?;
            self.repository.store_member_role(&new_member_role).await?;
        }

        tracing::info!(
            act = %context.execution.act_name,
            "Discord member role assignments stored successfully"
        );
        Ok(())
    }

    fn should_process(&self, context: &ProcessorContext<'_>) -> bool {
        let name_lower = context.execution.act_name.to_lowercase();
        let name_match = name_lower.contains("member") && name_lower.contains("role");

        // Member roles have guild_id, user_id, role_id, and assigned_at
        let content_match = context.execution.response.contains("\"guild_id\"")
            && context.execution.response.contains("\"user_id\"")
            && context.execution.response.contains("\"role_id\"")
            && context.execution.response.contains("\"assigned_at\"");

        name_match || content_match
    }

    fn name(&self) -> &str {
        "DiscordMemberRoleProcessor"
    }
}

