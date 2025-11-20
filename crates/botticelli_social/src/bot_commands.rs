//! Bot command execution infrastructure.
//!
//! This module provides the core abstractions for executing bot commands across
//! different social media platforms. Each platform (Discord, Slack, etc.) implements
//! the `BotCommandExecutor` trait to provide platform-specific command handling.
//!
//! # Architecture
//!
//! - `BotCommandExecutor` - Trait for platform-specific command execution
//! - `BotCommandRegistry` - Registry for managing multiple platform executors
//! - `BotCommandError` - Error types for command execution failures
//!
//! # Example
//!
//! ```rust,ignore
//! use botticelli_social::{BotCommandRegistry, DiscordCommandExecutor};
//!
//! // Create platform-specific executor
//! let discord = DiscordCommandExecutor::new("DISCORD_TOKEN");
//!
//! // Register with registry
//! let mut registry = BotCommandRegistry::new();
//! registry.register(discord);
//!
//! // Execute command
//! let result = registry.execute("discord", "server.get_stats", &args).await?;
//! ```

use async_trait::async_trait;
use derive_more::{Display, Error};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Result type for bot command operations.
pub type BotCommandResult<T> = Result<T, BotCommandError>;

/// Specific bot command error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum BotCommandErrorKind {
    /// Command not found or not supported.
    #[display("Command not found: {}", _0)]
    CommandNotFound(String),

    /// Platform not found (no executor registered).
    #[display("Platform not found: {}", _0)]
    PlatformNotFound(String),

    /// Missing required argument.
    #[display("Missing required argument '{}' for command '{}'", arg_name, command)]
    MissingArgument {
        command: String,
        arg_name: String,
    },

    /// Invalid argument type or value.
    #[display("Invalid argument '{}' for command '{}': {}", arg_name, command, reason)]
    InvalidArgument {
        command: String,
        arg_name: String,
        reason: String,
    },

    /// API call failed.
    #[display("API call failed for '{}': {}", command, reason)]
    ApiError { command: String, reason: String },

    /// Authentication failed.
    #[display("Authentication failed for platform '{}': {}", platform, reason)]
    AuthenticationError { platform: String, reason: String },

    /// Rate limit exceeded.
    #[display("Rate limit exceeded for '{}': retry after {} seconds", command, retry_after)]
    RateLimitExceeded { command: String, retry_after: u64 },

    /// Permission denied.
    #[display("Permission denied for '{}': {}", command, reason)]
    PermissionDenied { command: String, reason: String },

    /// Resource not found (guild, channel, user, etc.).
    #[display("Resource not found for '{}': {}", command, resource_type)]
    ResourceNotFound {
        command: String,
        resource_type: String,
    },

    /// Serialization/deserialization error.
    #[display("Serialization error for '{}': {}", command, reason)]
    SerializationError { command: String, reason: String },
}

/// Bot command error with location tracking.
#[derive(Debug, Clone, Display, Error)]
#[display("Bot Command Error: {} at line {} in {}", kind, line, file)]
pub struct BotCommandError {
    pub kind: BotCommandErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl BotCommandError {
    #[track_caller]
    pub fn new(kind: BotCommandErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

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
/// pub struct DiscordCommandExecutor {
///     http: Arc<Http>,
/// }
///
/// #[async_trait]
/// impl BotCommandExecutor for DiscordCommandExecutor {
///     fn platform(&self) -> &str {
///         "discord"
///     }
///     
///     #[instrument(skip(self, args), fields(platform = "discord", command, arg_count = args.len()))]
///     async fn execute(&self, command: &str, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
///         match command {
///             "server.get_stats" => self.server_get_stats(args).await,
///             _ => Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(command.to_string()))),
///         }
///     }
///     
///     fn supports_command(&self, command: &str) -> bool {
///         matches!(command, "server.get_stats" | "channels.list")
///     }
/// }
/// ```
#[async_trait]
pub trait BotCommandExecutor: Send + Sync {
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
    ) -> BotCommandResult<JsonValue>;

    /// Check if this executor supports a command.
    fn supports_command(&self, command: &str) -> bool;

    /// List all supported commands.
    fn supported_commands(&self) -> Vec<String>;

    /// Get command documentation.
    fn command_help(&self, command: &str) -> Option<String>;
}

/// Registry of bot command executors for multiple platforms.
///
/// Manages platform-specific executors and routes commands to the appropriate
/// platform handler.
///
/// # Example
///
/// ```rust,ignore
/// let mut registry = BotCommandRegistry::new();
/// registry.register(DiscordCommandExecutor::new("TOKEN"));
/// registry.register(SlackCommandExecutor::new("TOKEN"));
///
/// let result = registry.execute("discord", "server.get_stats", &args).await?;
/// ```
pub struct BotCommandRegistry {
    executors: HashMap<String, Arc<dyn BotCommandExecutor>>,
}

impl BotCommandRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        tracing::debug!("Creating new BotCommandRegistry");
        Self {
            executors: HashMap::new(),
        }
    }

    /// Register an executor for a platform.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut registry = BotCommandRegistry::new();
    /// registry.register(DiscordCommandExecutor::new("TOKEN"));
    /// ```
    pub fn register<E: BotCommandExecutor + 'static>(&mut self, executor: E) -> &mut Self {
        let platform = executor.platform().to_string();
        let commands = executor.supported_commands();
        tracing::info!(
            platform = %platform,
            commands = commands.len(),
            "Registering bot command executor"
        );
        self.executors.insert(platform, Arc::new(executor));
        self
    }

    /// Get executor for a platform.
    pub fn get(&self, platform: &str) -> Option<&Arc<dyn BotCommandExecutor>> {
        self.executors.get(platform)
    }

    /// Execute a command on a platform.
    ///
    /// # Arguments
    ///
    /// * `platform` - Platform name (e.g., "discord", "slack")
    /// * `command` - Command to execute (e.g., "server.get_stats")
    /// * `args` - Command arguments
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Platform not found in registry
    /// - Command execution fails
    #[tracing::instrument(
        skip(self, args),
        fields(
            platform,
            command,
            arg_count = args.len()
        )
    )]
    pub async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        tracing::info!("Executing bot command via registry");

        let executor = self.get(platform).ok_or_else(|| {
            tracing::error!(
                platform,
                available_platforms = ?self.platforms(),
                "Platform not found in registry"
            );
            BotCommandError::new(BotCommandErrorKind::PlatformNotFound(
                platform.to_string(),
            ))
        })?;

        executor.execute(command, args).await
    }

    /// List all registered platforms.
    pub fn platforms(&self) -> Vec<String> {
        self.executors.keys().cloned().collect()
    }

    /// Check if a platform is registered.
    pub fn has_platform(&self, platform: &str) -> bool {
        self.executors.contains_key(platform)
    }
}

impl Default for BotCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the narrative trait to avoid circular dependencies
#[async_trait]
impl botticelli_narrative::BotCommandRegistry for BotCommandRegistry {
    async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
        self.execute(platform, command, args)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock executor for testing
    struct MockBotCommandExecutor {
        platform_name: String,
        responses: HashMap<String, JsonValue>,
    }

    impl MockBotCommandExecutor {
        fn new(platform: &str) -> Self {
            let mut responses = HashMap::new();

            // Mock server.get_stats response
            responses.insert(
                "server.get_stats".to_string(),
                serde_json::json!({
                    "guild_id": "1234567890",
                    "name": "Test Server",
                    "member_count": 100,
                    "channel_count": 10
                }),
            );

            Self {
                platform_name: platform.to_string(),
                responses,
            }
        }
    }

    #[async_trait]
    impl BotCommandExecutor for MockBotCommandExecutor {
        fn platform(&self) -> &str {
            &self.platform_name
        }

        async fn execute(
            &self,
            command: &str,
            _args: &HashMap<String, JsonValue>,
        ) -> BotCommandResult<JsonValue> {
            self.responses
                .get(command)
                .cloned()
                .ok_or_else(|| {
                    BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                        command.to_string(),
                    ))
                })
        }

        fn supports_command(&self, command: &str) -> bool {
            self.responses.contains_key(command)
        }

        fn supported_commands(&self) -> Vec<String> {
            self.responses.keys().cloned().collect()
        }

        fn command_help(&self, _command: &str) -> Option<String> {
            None
        }
    }

    #[tokio::test]
    async fn test_bot_command_execution() {
        let executor = MockBotCommandExecutor::new("mock");
        let mut args = HashMap::new();
        args.insert("guild_id".to_string(), serde_json::json!("1234567890"));

        let result = executor.execute("server.get_stats", &args).await.unwrap();

        assert_eq!(result["member_count"], 100);
        assert_eq!(result["channel_count"], 10);
    }

    #[tokio::test]
    async fn test_bot_command_registry() {
        let mut registry = BotCommandRegistry::new();
        registry.register(MockBotCommandExecutor::new("mock"));

        let mut args = HashMap::new();
        args.insert("guild_id".to_string(), serde_json::json!("1234567890"));

        let result = registry
            .execute("mock", "server.get_stats", &args)
            .await
            .unwrap();

        assert_eq!(result["member_count"], 100);
    }

    #[tokio::test]
    async fn test_unknown_platform() {
        let registry = BotCommandRegistry::new();
        let args = HashMap::new();

        let result = registry.execute("unknown", "test", &args).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            BotCommandErrorKind::PlatformNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_unknown_command() {
        let executor = MockBotCommandExecutor::new("mock");
        let args = HashMap::new();

        let result = executor.execute("unknown.command", &args).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            BotCommandErrorKind::CommandNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_registry_platforms() {
        let mut registry = BotCommandRegistry::new();
        registry.register(MockBotCommandExecutor::new("discord"));
        registry.register(MockBotCommandExecutor::new("slack"));

        let platforms = registry.platforms();
        assert_eq!(platforms.len(), 2);
        assert!(platforms.contains(&"discord".to_string()));
        assert!(platforms.contains(&"slack".to_string()));
    }

    #[tokio::test]
    async fn test_registry_has_platform() {
        let mut registry = BotCommandRegistry::new();
        registry.register(MockBotCommandExecutor::new("discord"));

        assert!(registry.has_platform("discord"));
        assert!(!registry.has_platform("slack"));
    }
}
