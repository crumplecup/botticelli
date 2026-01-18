//! Bot server types.

use elicitation::{Prompt, Select};
use std::time::Duration;

/// State of a bot actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, elicitation::Elicit)]
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
