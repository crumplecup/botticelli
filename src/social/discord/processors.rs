//! ActProcessor implementations for Discord data types.
//!
//! This module provides processors that extract Discord data from LLM responses
//! and insert them into the database using the DiscordRepository.

use crate::{
    ActProcessor, BoticelliResult, DiscordChannelJson, DiscordGuildJson, DiscordGuildMemberJson,
    DiscordMemberRoleJson, DiscordRepository, DiscordRoleJson, DiscordUserJson, NewChannel,
    NewGuild, NewGuildMember, NewMemberRole, NewRole, NewUser, ProcessorContext, extract_json,
    parse_json,
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
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordGuildProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
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
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordUserProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
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
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordChannelProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
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
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordRoleProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
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
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordGuildMemberProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
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
    pub fn new(repository: Arc<DiscordRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ActProcessor for DiscordMemberRoleProcessor {
    async fn process(&self, context: &ProcessorContext<'_>) -> BoticelliResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActExecution, Input, NarrativeMetadata};

    fn create_test_execution(act_name: &str, response: &str) -> ActExecution {
        ActExecution {
            act_name: act_name.to_string(),
            inputs: vec![Input::Text("test input".to_string())],
            model: None,
            temperature: None,
            max_tokens: None,
            response: response.to_string(),
            sequence_number: 0,
        }
    }

    fn create_test_metadata() -> NarrativeMetadata {
        NarrativeMetadata {
            name: "test_narrative".to_string(),
            description: "Test narrative".to_string(),
            template: None,
            skip_content_generation: false,
        }
    }

    fn create_test_context<'a>(
        execution: &'a ActExecution,
        metadata: &'a NarrativeMetadata,
    ) -> ProcessorContext<'a> {
        ProcessorContext {
            execution,
            narrative_metadata: metadata,
            narrative_name: "test_narrative",
        }
    }

    #[test]
    fn test_guild_processor_should_process_by_name() {
        let processor = DiscordGuildProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let exec1 = create_test_execution("create_guild", "");
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let exec2 = create_test_execution("CREATE_SERVER", "");
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(processor.should_process(&ctx2));

        let exec3 = create_test_execution("create_user", "");
        let ctx3 = create_test_context(&exec3, &metadata);
        assert!(!processor.should_process(&ctx3));
    }

    #[test]
    fn test_guild_processor_should_process_by_content() {
        let processor = DiscordGuildProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let response_with_owner = r#"{"id": 123, "name": "Test", "owner_id": 456}"#;
        let exec1 = create_test_execution("unknown_act", response_with_owner);
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let response_without_owner = r#"{"id": 123, "name": "Test"}"#;
        let exec2 = create_test_execution("unknown_act", response_without_owner);
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(!processor.should_process(&ctx2));
    }

    #[test]
    fn test_user_processor_should_process() {
        let processor = DiscordUserProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let exec1 = create_test_execution("create_user", "");
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let exec2 = create_test_execution("generate_members", "");
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(processor.should_process(&ctx2));

        let user_response = r#"{"id": 123, "username": "test"}"#;
        let exec3 = create_test_execution("unknown", user_response);
        let ctx3 = create_test_context(&exec3, &metadata);
        assert!(processor.should_process(&ctx3));

        let member_response = r#"{"user_id": 123, "guild_id": 456}"#;
        let exec4 = create_test_execution("unknown", member_response);
        let ctx4 = create_test_context(&exec4, &metadata);
        assert!(!processor.should_process(&ctx4));
    }

    #[test]
    fn test_channel_processor_should_process() {
        let processor = DiscordChannelProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let exec1 = create_test_execution("create_channels", "");
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let channel_response = r#"{"id": 123, "channel_type": "guild_text"}"#;
        let exec2 = create_test_execution("unknown", channel_response);
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(processor.should_process(&ctx2));
    }

    #[test]
    fn test_role_processor_should_process() {
        let processor = DiscordRoleProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let exec1 = create_test_execution("create_roles", "");
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let role_response = r#"{"id": 123, "permissions": "0", "position": 1}"#;
        let exec2 = create_test_execution("unknown", role_response);
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(processor.should_process(&ctx2));
    }

    #[test]
    fn test_guild_member_processor_should_process() {
        let processor = DiscordGuildMemberProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let exec1 = create_test_execution("create_members", "");
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let exec2 = create_test_execution("create_member_roles", "");
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(!processor.should_process(&ctx2));

        let member_response = r#"{"guild_id": 123, "user_id": 456, "joined_at": "2024-01-01"}"#;
        let exec3 = create_test_execution("unknown", member_response);
        let ctx3 = create_test_context(&exec3, &metadata);
        assert!(processor.should_process(&ctx3));
    }

    #[test]
    fn test_member_role_processor_should_process() {
        let processor = DiscordMemberRoleProcessor {
            repository: Arc::new(DiscordRepository::from_arc(Arc::new(
                tokio::sync::Mutex::new(
                    crate::establish_connection().expect("DB connection failed"),
                ),
            ))),
        };

        let metadata = create_test_metadata();
        let exec1 = create_test_execution("assign_member_roles", "");
        let ctx1 = create_test_context(&exec1, &metadata);
        assert!(processor.should_process(&ctx1));

        let role_response =
            r#"{"guild_id": 123, "user_id": 456, "role_id": 789, "assigned_at": "2024-01-01"}"#;
        let exec2 = create_test_execution("unknown", role_response);
        let ctx2 = create_test_context(&exec2, &metadata);
        assert!(processor.should_process(&ctx2));
    }
}
