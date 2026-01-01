//! Bot server trait definitions.

use async_trait::async_trait;

/// Trait for bot actors that run scheduled tasks.
#[async_trait]
pub trait BotActor: Send + Sync {
    /// Driver type for LLM backend.
    type Driver: Send + Sync + Clone + 'static;
    /// State type for bot status.
    type State;
    /// Statistics type for bot metrics.
    type Stats;
    /// Error type for bot operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Start the bot actor with the provided driver.
    async fn start(&mut self, driver: Self::Driver) -> Result<(), Self::Error>;

    /// Stop the bot actor.
    async fn stop(&mut self) -> Result<(), Self::Error>;

    /// Pause the bot actor.
    async fn pause(&mut self) -> Result<(), Self::Error>;

    /// Resume the bot actor from paused state.
    async fn resume(&mut self) -> Result<(), Self::Error>;

    /// Get current state of the bot.
    fn state(&self) -> Self::State;

    /// Get statistics for the bot.
    fn stats(&self) -> Self::Stats;

    /// Get the name of the bot.
    fn name(&self) -> &str;
}

/// Trait for bot server management.
#[async_trait]
pub trait BotServer: Send + Sync {
    /// Driver type for LLM backend.
    type Driver: Send + Sync + Clone + 'static;
    /// State type for bot status.
    type State;
    /// Statistics type for bot metrics.
    type Stats;
    /// Error type for server operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Start the bot server with all configured bots using the provided driver.
    async fn start(&mut self, driver: Self::Driver) -> Result<(), Self::Error>;

    /// Stop the bot server and all bots.
    async fn stop(&mut self) -> Result<(), Self::Error>;

    /// Get the state of all bots.
    fn bot_states(&self) -> Vec<(String, Self::State)>;

    /// Get statistics for all bots.
    fn bot_stats(&self) -> Vec<(String, Self::Stats)>;

    /// Pause a specific bot by name.
    async fn pause_bot(&mut self, name: &str) -> Result<(), Self::Error>;

    /// Resume a specific bot by name.
    async fn resume_bot(&mut self, name: &str) -> Result<(), Self::Error>;
}
