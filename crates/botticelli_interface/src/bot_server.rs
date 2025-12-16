//! Bot server trait definitions.

use async_trait::async_trait;
use std::time::Duration;

/// Result type for bot operations.
pub type BotResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// State of a bot actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BotState {
    /// Bot is starting up.
    Starting,
    /// Bot is running normally.
    Running,
    /// Bot is paused.
    Paused,
    /// Bot is stopping.
    Stopping,
    /// Bot has stopped.
    Stopped,
    /// Bot encountered an error.
    Failed,
}

/// Statistics for bot execution.
#[derive(Debug, Clone, derive_getters::Getters)]
pub struct BotStats {
    /// Number of tasks processed successfully.
    tasks_completed: u64,
    /// Number of tasks that failed.
    tasks_failed: u64,
    /// Total time spent processing tasks.
    total_processing_time: Duration,
    /// Timestamp of last task completion.
    last_task_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Trait for bot actors that run scheduled tasks.
///
/// Generic over `D: crate::BotticelliDriver` to support different LLM backends.
#[async_trait]
pub trait BotActor<D>: Send + Sync
where
    D: crate::BotticelliDriver + Clone + 'static,
{
    /// Start the bot actor with the provided driver.
    async fn start(&mut self, driver: D) -> BotResult<()>;

    /// Stop the bot actor.
    async fn stop(&mut self) -> BotResult<()>;

    /// Pause the bot actor.
    async fn pause(&mut self) -> BotResult<()>;

    /// Resume the bot actor from paused state.
    async fn resume(&mut self) -> BotResult<()>;

    /// Get current state of the bot.
    fn state(&self) -> BotState;

    /// Get statistics for the bot.
    fn stats(&self) -> BotStats;

    /// Get the name of the bot.
    fn name(&self) -> &str;
}

/// Configuration for the bot server.
#[derive(Debug, Clone, derive_getters::Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct BotServerConfig {
    /// Path to the configuration file.
    config_path: String,
    /// Enable graceful shutdown on signals.
    graceful_shutdown: bool,
    /// Shutdown timeout duration.
    shutdown_timeout: Duration,
}

/// Trait for bot server management.
///
/// Generic over `D: crate::BotticelliDriver` to support different LLM backends.
#[async_trait]
pub trait BotServer<D>: Send + Sync
where
    D: crate::BotticelliDriver + Clone + 'static,
{
    /// Start the bot server with all configured bots using the provided driver.
    async fn start(&mut self, driver: D) -> BotResult<()>;

    /// Stop the bot server and all bots.
    async fn stop(&mut self) -> BotResult<()>;

    /// Get the state of all bots.
    fn bot_states(&self) -> Vec<(String, BotState)>;

    /// Get statistics for all bots.
    fn bot_stats(&self) -> Vec<(String, BotStats)>;

    /// Pause a specific bot by name.
    async fn pause_bot(&mut self, name: &str) -> BotResult<()>;

    /// Resume a specific bot by name.
    async fn resume_bot(&mut self, name: &str) -> BotResult<()>;
}
