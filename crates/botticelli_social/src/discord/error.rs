//! Discord-specific error types.
//!
//! This module provides error handling for Discord integration, including
//! Serenity API errors, connection issues, and Discord-specific validation errors.

use derive_getters::Getters;

/// Discord error variants.
///
/// Represents different error conditions that can occur during Discord operations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub enum DiscordErrorKind {
    /// Serenity API error (e.g., HTTP error, gateway error, rate limit).
    #[display("Serenity API error: {_0}")]
    SerenityError(String),

    /// Database operation failed.
    #[display("Database error: {_0}")]
    DatabaseError(String),

    /// Guild (server) not found by ID.
    #[display("Guild not found: {_0}")]
    GuildNotFound(i64),

    /// Channel not found by ID.
    #[display("Channel not found: {_0}")]
    ChannelNotFound(i64),

    /// User not found by ID.
    #[display("User not found: {_0}")]
    UserNotFound(i64),

    /// Role not found by ID.
    #[display("Role not found: {_0}")]
    RoleNotFound(i64),

    /// Bot lacks required permissions for an operation.
    #[display("Insufficient permissions: {_0}")]
    InsufficientPermissions(String),

    /// Invalid Discord snowflake ID format.
    #[display("Invalid ID: {_0}")]
    InvalidId(String),

    /// Connection to Discord gateway failed.
    #[display("Connection failed: {_0}")]
    ConnectionFailed(String),

    /// Bot token is invalid or expired.
    #[display("Invalid or expired bot token")]
    InvalidToken,

    /// Message failed to send.
    #[display("Message send failed: {_0}")]
    MessageSendFailed(String),

    /// Interaction (slash command, button) failed.
    #[display("Interaction failed: {_0}")]
    InteractionFailed(String),

    /// Configuration error (missing env vars, invalid settings).
    #[display("Configuration error: {_0}")]
    ConfigurationError(String),
}

/// Discord error with source location tracking.
///
/// Captures the error kind along with the file and line where the error occurred.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, Getters)]
#[display("Discord Error: {} at line {} in {}", kind, line, file)]
pub struct DiscordError {
    kind: DiscordErrorKind,
    line: u32,
    file: &'static str,
}

impl DiscordError {
    /// Create a new DiscordError with automatic location tracking.
    ///
    /// # Example
    /// ```
    /// use botticelli_social::{DiscordError, DiscordErrorKind};
    ///
    /// let err = DiscordError::new(DiscordErrorKind::InvalidToken);
    /// ```
    #[track_caller]
    pub fn new(kind: DiscordErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

/// Result type for Discord operations.
pub type DiscordResult<T> = Result<T, DiscordError>;

// Convenience From implementations for external error types
#[cfg(feature = "discord")]
impl From<serenity::Error> for DiscordError {
    #[track_caller]
    fn from(err: serenity::Error) -> Self {
        DiscordError::new(DiscordErrorKind::SerenityError(err.to_string()))
    }
}
