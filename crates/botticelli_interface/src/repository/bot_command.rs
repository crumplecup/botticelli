//! Bot command registry trait.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Trait for executing bot commands (platform-agnostic).
///
/// This trait allows narratives to execute bot commands on various platforms
/// (Discord, social media, etc.) without depending on specific implementations.
#[async_trait]
pub trait BotCommandRegistry: Send + Sync {
    /// Error type for bot command operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute a bot command on a specific platform.
    ///
    /// # Parameters
    ///
    /// * `platform` - The platform identifier (e.g., "discord", "twitter")
    /// * `command` - The command to execute
    /// * `args` - Command arguments as JSON values
    ///
    /// # Errors
    ///
    /// Returns an error if the command execution fails.
    async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;
}
