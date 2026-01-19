//! Bot command execution infrastructure.
//!
//! This module provides the core abstractions for executing bot commands across
//! different social media platforms. Each platform (Discord, Slack, etc.) implements
//! the `BotCommandExecutor` trait to provide platform-specific command handling.
//!
//! # Architecture
//!
//! - `BotCommandExecutor` - Trait for platform-specific command execution (from botticelli_interface)
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
use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use botticelli_interface::BotCommandExecutor;
use derive_getters::Getters;
use rmcp::tool;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{Span, debug, error, info, instrument, warn};

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
    executors: HashMap<String, Arc<dyn BotCommandExecutor<Error = BotCommandError>>>,
    cache: Arc<Mutex<CommandCache>>,
}

impl BotCommandRegistryImpl {
    /// Create a new empty registry with default cache.
    #[tool]
    #[instrument]
    pub fn new() -> Self {
        debug!("Creating new BotCommandRegistryImpl");
        Self {
            executors: HashMap::new(),
            cache: Arc::new(Mutex::new(CommandCache::default())),
        }
    }

    /// Create a new registry with custom cache.
    #[tool]
    #[instrument(skip(cache))]
    pub fn with_cache(cache: CommandCache) -> Self {
        debug!("Creating new BotCommandRegistry with custom cache");
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
    #[instrument(skip(self, executor), fields(platform = %executor.platform()))]
    pub fn register<E>(&mut self, executor: E) -> &mut Self
    where
        E: BotCommandExecutor<Error = BotCommandError> + 'static,
    {
        let platform = executor.platform().to_string();
        let commands = executor.supported_commands();
        info!(
            platform = %platform,
            commands = commands.len(),
            "Registering bot command executor"
        );
        self.executors.insert(platform, Arc::new(executor));
        self
    }

    /// Get executor for a platform.
    #[instrument(skip(self))]
    pub fn get(
        &self,
        platform: &str,
    ) -> Option<&Arc<dyn BotCommandExecutor<Error = BotCommandError>>> {
        let result = self.executors.get(platform);
        if result.is_some() {
            debug!(platform, "Found executor");
        } else {
            debug!(platform, "Executor not found");
        }
        result
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
        info!("Executing bot command via registry");

        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(platform, command, args) {
                Span::current().record("cache_hit", true);
                info!(
                    time_remaining = ?entry.time_remaining(),
                    "Cache hit, returning cached result"
                );
                return Ok(entry.value().clone());
            }
        }

        Span::current().record("cache_hit", false);

        let executor = self.get(platform).ok_or_else(|| {
            error!(
                platform,
                available_platforms = ?self.platforms(),
                "Platform not found in registry"
            );
            BotCommandError::new(BotCommandErrorKind::PlatformNotFound(platform.to_string()))
        })?;

        let result = executor.execute(command, args).await?;

        // Cache the result
        let cache_duration = args.get("cache_duration").and_then(|v| v.as_u64());

        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(platform, command, args, result.clone(), cache_duration);
        }

        Ok(result)
    }

    /// List all registered platforms.
    #[tool]
    #[instrument(skip(self))]
    pub fn platforms(&self) -> Vec<String> {
        let platforms = self.executors.keys().cloned().collect::<Vec<_>>();
        debug!(count = platforms.len(), "Listing platforms");
        platforms
    }

    /// Check if a platform is registered.
    #[tool]
    #[instrument(skip(self), fields(platform))]
    pub fn has_platform(&self, platform: &str) -> bool {
        let exists = self.executors.contains_key(platform);
        debug!(platform, exists, "Checking platform registration");
        exists
    }
}

impl Default for BotCommandRegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the narrative trait to avoid circular dependencies
#[async_trait]
impl botticelli_interface::BotCommandRegistry for BotCommandRegistryImpl {
    type Error = BotCommandError;

    async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        self.execute(platform, command, args).await
    }
}
