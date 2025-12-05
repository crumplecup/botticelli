//! Social media integration tools for MCP.
//!
//! These tools provide access to social media platforms like Discord through
//! bot commands, enabling LLMs to query server stats, post messages, and
//! interact with social media data.

use crate::tools::McpTool;
use crate::{McpError, McpResult};
use async_trait::async_trait;
use botticelli_social::{BotCommandRegistryImpl, DiscordCommandExecutor};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Execute Discord bot command tool.
///
/// This tool provides access to Discord bot commands, allowing LLMs to query
/// Discord servers, channels, and messages through the bot command system.
#[derive(Clone)]
pub struct DiscordBotCommandTool {
    registry: Arc<BotCommandRegistryImpl>,
}

impl DiscordBotCommandTool {
    /// Creates a new Discord bot command tool.
    ///
    /// # Arguments
    ///
    /// * `token` - Discord bot token for authentication
    #[instrument(skip(token))]
    pub fn new(token: String) -> McpResult<Self> {
        debug!("Creating Discord bot command tool");
        
        let executor = DiscordCommandExecutor::new(token);
        
        let mut registry = BotCommandRegistryImpl::new();
        registry.register(executor);
        
        Ok(Self {
            registry: Arc::new(registry),
        })
    }
}

#[async_trait]
impl McpTool for DiscordBotCommandTool {
    fn name(&self) -> &str {
        "discord_bot_command"
    }

    fn description(&self) -> &str {
        "Execute Discord bot commands to query server data, channels, messages, and more. \
         Available commands: server.get_stats, channels.list, messages.list, and others. \
         Each command requires specific arguments (e.g., guild_id, channel_id)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bot command to execute (e.g., 'server.get_stats', 'channels.list')"
                },
                "args": {
                    "type": "object",
                    "description": "Arguments for the command as key-value pairs",
                    "additionalProperties": true
                }
            },
            "required": ["command"]
        })
    }

    #[instrument(skip(self), fields(command))]
    async fn execute(&self, params: Value) -> McpResult<Value> {
        debug!("Executing Discord bot command");

        let command = params
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'command' parameter".to_string()))?;

        let args_obj = params
            .get("args")
            .and_then(|a| a.as_object())
            .cloned()
            .unwrap_or_default();

        // Convert serde_json::Map to HashMap<String, Value>
        let args: HashMap<String, Value> = args_obj.into_iter().collect();

        debug!(command = %command, args = ?args, "Executing bot command");

        let result = self
            .registry
            .execute("discord", command, &args)
            .await
            .map_err(|e| McpError::ToolExecutionFailed(format!("Bot command failed: {}", e)))?;

        Ok(result)
    }
}

/// Post to Discord tool.
///
/// This tool posts messages to Discord channels, enabling LLMs to publish
/// content generated through narratives or other means.
#[derive(Clone)]
pub struct DiscordPostTool {
    registry: Arc<BotCommandRegistryImpl>,
}

impl DiscordPostTool {
    /// Creates a new Discord post tool.
    ///
    /// # Arguments
    ///
    /// * `token` - Discord bot token for authentication
    #[instrument(skip(token))]
    pub fn new(token: String) -> McpResult<Self> {
        debug!("Creating Discord post tool");
        
        let executor = DiscordCommandExecutor::new(token);
        
        let mut registry = BotCommandRegistryImpl::new();
        registry.register(executor);
        
        Ok(Self {
            registry: Arc::new(registry),
        })
    }
}

#[async_trait]
impl McpTool for DiscordPostTool {
    fn name(&self) -> &str {
        "discord_post"
    }

    fn description(&self) -> &str {
        "Post a message to a Discord channel. Requires channel_id and content. \
         Useful for publishing generated content, announcements, or bot responses."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel_id": {
                    "type": "string",
                    "description": "The Discord channel ID to post to"
                },
                "content": {
                    "type": "string",
                    "description": "The message content to post"
                }
            },
            "required": ["channel_id", "content"]
        })
    }

    #[instrument(skip(self), fields(channel_id))]
    async fn execute(&self, params: Value) -> McpResult<Value> {
        debug!("Posting to Discord");

        let channel_id = params
            .get("channel_id")
            .and_then(|c| c.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'channel_id' parameter".to_string()))?;

        let content = params
            .get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| McpError::InvalidInput("Missing 'content' parameter".to_string()))?;

        debug!(channel_id = %channel_id, content_len = content.len(), "Posting message");

        let mut args = HashMap::new();
        args.insert("channel_id".to_string(), json!(channel_id));
        args.insert("content".to_string(), json!(content));

        let result = self
            .registry
            .execute("discord", "messages.send", &args)
            .await
            .map_err(|e| McpError::ToolExecutionFailed(format!("Post failed: {}", e)))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_bot_command_tool_schema() {
        let token = "test_token".to_string();
        let tool = DiscordBotCommandTool::new(token).expect("Tool creation failed");
        
        assert_eq!(tool.name(), "discord_bot_command");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
    }

    #[test]
    fn test_discord_post_tool_schema() {
        let token = "test_token".to_string();
        let tool = DiscordPostTool::new(token).expect("Tool creation failed");
        
        assert_eq!(tool.name(), "discord_post");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
    }
}
