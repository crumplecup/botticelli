//! Bot command integration documentation.
//!
//! Bot command integration is implemented via the Discord social tools
//! in the `social` module. This module serves as documentation for how
//! narratives can query Discord data during execution.
//!
//! # Available Bot Commands
//!
//! The following commands are available through the MCP Discord tools:
//!
//! - `discord/get_guild_info` - Get server statistics and information
//! - `discord/get_channels` - List channels in a guild
//! - `discord/get_messages` - Retrieve messages from a channel
//! - `discord/post_message` - Post a message to a channel
//!
//! # Usage in Narratives
//!
//! Narratives can use these commands to:
//! - Query server stats for content generation
//! - Retrieve messages for analysis
//! - Post generated content to Discord
//! - Build data-driven narratives based on Discord state
//!
//! # Example
//!
//! ```toml
//! [acts.discord_query]
//! system = "You are analyzing Discord data"
//! user = "Get statistics for guild {guild_id}"
//! # Response includes server stats that can be used in subsequent acts
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request to execute a bot command during narrative execution.
///
/// This is a documentation struct showing the expected format for
/// bot command requests. Actual execution happens through Discord tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BotCommandRequest {
    /// Command to execute (e.g., "discord/get_guild_info")
    pub command: String,

    /// Command arguments as JSON
    #[serde(default)]
    pub args: Value,
}

/// Response from bot command execution.
///
/// This is a documentation struct showing the expected format for
/// bot command responses. Actual responses come from Discord tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BotCommandResponse {
    /// Command that was executed
    pub command: String,

    /// Response data from command
    pub data: Value,

    /// Whether command succeeded
    pub success: bool,

    /// Error message if command failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_bot_command_request_serialization() {
        let request = BotCommandRequest {
            command: "server.get_stats".to_string(),
            args: json!({"guild_id": "123456"}),
        };

        let json = serde_json::to_string(&request).expect("Serialize");
        assert!(json.contains("server.get_stats"));
        assert!(json.contains("guild_id"));
    }

    #[test]
    fn test_bot_command_response_serialization() {
        let response = BotCommandResponse {
            command: "server.get_stats".to_string(),
            data: json!({"member_count": 100}),
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&response).expect("Serialize");
        assert!(json.contains("server.get_stats"));
        assert!(json.contains("member_count"));
    }
}
