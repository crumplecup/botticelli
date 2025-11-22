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
//! let mut registry = BotCommandRegistryImpl::new();
//! registry.register(discord);
//!
//! // Execute command
//! let result = registry.execute("discord", "server.get_stats", &args).await?;
//! ```

use async_trait::async_trait;
use botticelli_cache::CommandCache;
use derive_getters::Getters;
use derive_more::{Display, Error};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
        /// Command that was missing an argument
        command: String,
        /// Name of the missing argument
        arg_name: String,
    },

    /// Invalid argument type or value.
    #[display("Invalid argument '{}' for command '{}': {}", arg_name, command, reason)]
    InvalidArgument {
        /// Command that received invalid argument
        command: String,
        /// Name of the invalid argument
        arg_name: String,
        /// Reason why the argument is invalid
        reason: String,
    },

    /// API call failed.
    #[display("API call failed for '{}': {}", command, reason)]
    ApiError {
        /// Command that failed
        command: String,
        /// Reason for failure
        reason: String,
    },

    /// Authentication failed.
    #[display("Authentication failed for platform '{}': {}", platform, reason)]
    AuthenticationError {
        /// Platform that failed authentication
        platform: String,
        /// Reason for authentication failure
        reason: String,
    },

    /// Rate limit exceeded.
    #[display("Rate limit exceeded for '{}': retry after {} seconds", command, retry_after)]
    RateLimitExceeded {
        /// Command that was rate limited
        command: String,
        /// Seconds to wait before retrying
        retry_after: u64,
    },

    /// Permission denied.
    #[display("Permission denied for '{}': {}", command, reason)]
    PermissionDenied {
        /// Command that was denied
        command: String,
        /// Reason for denial
        reason: String,
    },

    /// Security policy violation.
    #[display("Security error for '{}': {}", command, reason)]
    SecurityError {
        /// Command that triggered security error
        command: String,
        /// Reason for security violation
        reason: String,
    },

    /// Content filtered by security policy.
    #[display("Content filtered for '{}': {}", command, reason)]
    ContentFiltered {
        /// Command that had content filtered
        command: String,
        /// Reason for filtering
        reason: String,
    },

    /// Resource not found (guild, channel, user, etc.).
    #[display("Resource not found for '{}': {}", command, resource_type)]
    ResourceNotFound {
        /// Command that couldn't find resource
        command: String,
        /// Type of resource that wasn't found
        resource_type: String,
    },

    /// Serialization/deserialization error.
    #[display("Serialization error for '{}': {}", command, reason)]
    SerializationError {
        /// Command that had serialization error
        command: String,
        /// Reason for serialization failure
        reason: String,
    },
}

/// Bot command error with location tracking.
#[derive(Debug, Clone, Display, Error, Getters)]
#[display("Bot Command Error: {} at line {} in {}", kind, line, file)]
pub struct BotCommandError {
    /// The specific error kind
    kind: BotCommandErrorKind,
    /// Line number where error occurred
    line: u32,
    /// File where error occurred
    file: &'static str,
}

impl BotCommandError {
    /// Create a new bot command error with location tracking.
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

    // Message bulk operations
    /// Bulk delete messages from a channel.
    async fn messages_bulk_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    // Thread operations
    /// Create a new thread.
    async fn threads_create(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// List threads in a guild or channel.
    async fn threads_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Get thread information.
    async fn threads_get(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Edit a thread.
    async fn threads_edit(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Delete a thread.
    async fn threads_delete(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Join a thread.
    async fn threads_join(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Leave a thread.
    async fn threads_leave(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Add a member to a thread.
    async fn threads_add_member(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Remove a member from a thread.
    async fn threads_remove_member(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    // Reaction operations
    /// List users who reacted with an emoji.
    async fn reactions_list(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Clear all reactions from a message.
    async fn reactions_clear(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Clear all reactions of a specific emoji from a message.
    async fn reactions_clear_emoji(
        &self,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue>;

    /// Get command documentation.
    fn command_help(&self, command: &str) -> Option<String>;
}

/// Registry of bot command executors for multiple platforms.
///
/// Manages platform-specific executors and routes commands to the appropriate
/// platform handler. Includes result caching with TTL support.
///
/// # Example
///
/// ```rust,ignore
/// let mut registry = BotCommandRegistryImpl::new();
/// registry.register(DiscordCommandExecutor::new("TOKEN"));
/// registry.register(SlackCommandExecutor::new("TOKEN"));
///
/// let result = registry.execute("discord", "server.get_stats", &args).await?;
/// ```
#[derive(Getters)]
pub struct BotCommandRegistryImpl {
    executors: HashMap<String, Arc<dyn BotCommandExecutor>>,
    cache: Arc<Mutex<CommandCache>>,
}

impl BotCommandRegistryImpl {
    /// Create a new empty registry with default cache.
    pub fn new() -> Self {
        tracing::debug!("Creating new BotCommandRegistryImpl");
        Self {
            executors: HashMap::new(),
            cache: Arc::new(Mutex::new(CommandCache::default())),
        }
    }

    /// Create a new registry with custom cache.
    pub fn with_cache(cache: CommandCache) -> Self {
        tracing::debug!("Creating new BotCommandRegistry with custom cache");
        Self {
            executors: HashMap::new(),
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Register an executor for a platform.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut registry = BotCommandRegistryImpl::new();
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

    /// Execute a command on a platform with caching support.
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
    ///
    /// # Caching
    ///
    /// Results are cached with TTL based on `cache_duration` argument.
    /// If `cache_duration` is present in args, it overrides the default TTL.
    #[tracing::instrument(
        skip(self, args),
        fields(
            platform,
            command,
            arg_count = args.len(),
            cache_hit = false
        )
    )]
    pub async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<JsonValue> {
        tracing::info!("Executing bot command via registry");

        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(platform, command, args) {
                tracing::Span::current().record("cache_hit", true);
                tracing::info!(
                    time_remaining = ?entry.time_remaining(),
                    "Cache hit, returning cached result"
                );
                return Ok(entry.value().clone());
            }
        }

        tracing::Span::current().record("cache_hit", false);

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

        let result = executor.execute(command, args).await?;

        // Cache the result
        let cache_duration = args
            .get("cache_duration")
            .and_then(|v| v.as_u64());

        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(platform, command, args, result.clone(), cache_duration);
        }

        Ok(result)
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

impl Default for BotCommandRegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the narrative trait to avoid circular dependencies
#[async_trait]
impl botticelli_narrative::BotCommandRegistry for BotCommandRegistryImpl {
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

        async fn messages_bulk_delete(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"deleted": 5}))
        }

        async fn threads_create(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"thread_id": "123456"}))
        }

        async fn threads_list(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"threads": []}))
        }

        async fn threads_get(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"thread_id": "123456"}))
        }

        async fn threads_edit(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn threads_delete(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn threads_join(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn threads_leave(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn threads_add_member(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn threads_remove_member(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn reactions_list(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"reactions": []}))
        }

        async fn reactions_clear(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
        }

        async fn reactions_clear_emoji(&self, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"success": true}))
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
        let mut registry = BotCommandRegistryImpl::new();
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
        let registry = BotCommandRegistryImpl::new();
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
        let mut registry = BotCommandRegistryImpl::new();
        registry.register(MockBotCommandExecutor::new("discord"));
        registry.register(MockBotCommandExecutor::new("slack"));

        let platforms = registry.platforms();
        assert_eq!(platforms.len(), 2);
        assert!(platforms.contains(&"discord".to_string()));
        assert!(platforms.contains(&"slack".to_string()));
    }

    #[tokio::test]
    async fn test_registry_has_platform() {
        let mut registry = BotCommandRegistryImpl::new();
        registry.register(MockBotCommandExecutor::new("discord"));

        assert!(registry.has_platform("discord"));
        assert!(!registry.has_platform("slack"));
    }
}
