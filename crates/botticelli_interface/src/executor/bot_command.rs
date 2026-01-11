//! Bot command executor trait.
//!
//! This module defines the platform-specific bot command executor interface.
//! Each platform (Discord, Slack, etc.) implements this trait to provide
//! command execution capabilities.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Executes bot commands for a specific platform.
///
/// Implementations handle platform-specific API calls and return structured
/// JSON results that can be converted to text for LLM consumption.
///
/// # Tracing
///
/// All implementations MUST instrument the `execute` method with:
/// - `#[instrument]` macro
/// - Span fields: platform, command, arg_count
/// - Debug events for key operations
/// - Error events with context
///
/// # Example Implementation
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use botticelli_interface::BotCommandExecutor;
/// use std::collections::HashMap;
/// use serde_json::Value as JsonValue;
///
/// pub struct DiscordCommandExecutor {
///     http: Arc<Http>,
/// }
///
/// #[async_trait]
/// impl BotCommandExecutor for DiscordCommandExecutor {
///     type Error = BotCommandError;
///     
///     fn platform(&self) -> &str {
///         "discord"
///     }
///     
///     #[instrument(skip(self, args), fields(platform = "discord", command, arg_count = args.len()))]
///     async fn execute(&self, command: &str, args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error> {
///         match command {
///             "server.get_stats" => self.server_get_stats(args).await,
///             _ => Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(command.to_string()))),
///         }
///     }
///     
///     fn supports_command(&self, command: &str) -> bool {
///         matches!(command, "server.get_stats" | "channels.list")
///     }
///     
///     fn supported_commands(&self) -> Vec<String> {
///         vec!["server.get_stats".to_string(), "channels.list".to_string()]
///     }
///     
///     fn command_help(&self, command: &str) -> Option<String> {
///         match command {
///             "server.get_stats" => Some("Get server statistics".to_string()),
///             _ => None,
///         }
///     }
///     
///     // ... implement remaining trait methods
/// }
/// ```
#[async_trait]
pub trait BotCommandExecutor: Send + Sync {
    /// Error type returned by this executor.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Returns the platform this executor handles (e.g., "discord", "slack").
    fn platform(&self) -> &str;

    /// Execute a command and return JSON result.
    ///
    /// # Arguments
    ///
    /// * `command` - Command string (e.g., "server.get_stats", "channels.list")
    /// * `args` - Command arguments as JSON values
    ///
    /// # Returns
    ///
    /// JSON value representing the command result
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Command is not supported
    /// - API call fails
    /// - Authentication fails
    /// - Rate limit exceeded
    ///
    /// # Tracing
    ///
    /// Must emit:
    /// - info! at start with command name
    /// - debug! for validation steps
    /// - error! if execution fails with full context
    /// - Record result_size in span
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// Check if this executor supports a command.
    fn supports_command(&self, command: &str) -> bool;

    /// List all supported commands.
    fn supported_commands(&self) -> Vec<String>;

    // Message bulk operations
    /// Bulk delete messages from a channel.
    async fn messages_bulk_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    // Thread operations
    /// Create a new thread.
    async fn threads_create(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// List threads in a guild or channel.
    async fn threads_list(&self, args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error>;

    /// Get thread information.
    async fn threads_get(&self, args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error>;

    /// Edit a thread.
    async fn threads_edit(&self, args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error>;

    /// Delete a thread.
    async fn threads_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// Join a thread.
    async fn threads_join(&self, args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error>;

    /// Leave a thread.
    async fn threads_leave(&self, args: &HashMap<String, JsonValue>)
    -> Result<JsonValue, Self::Error>;

    /// Add a member to a thread.
    async fn threads_add_member(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// Remove a member from a thread.
    async fn threads_remove_member(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    // Reaction operations
    /// List users who reacted with an emoji.
    async fn reactions_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// Clear all reactions from a message.
    async fn reactions_clear(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// Clear all reactions of a specific emoji from a message.
    async fn reactions_clear_emoji(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error>;

    /// Get command documentation.
    fn command_help(&self, command: &str) -> Option<String>;
}
