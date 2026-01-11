//! Discord-specific error types.
//!
//! This module provides error handling for Discord integration, including
//! Serenity API errors, connection issues, and Discord-specific validation errors.

use derive_getters::Getters;
use std::sync::Arc;

/// Result type for Discord operations.
pub type DiscordErrorResult<T> = Result<T, DiscordError>;

/// Error severity level for Discord event processing.
///
/// Used to determine whether event processing should abort or continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordErrorSeverity {
    /// Critical error - abort event processing immediately
    Critical,
    /// Non-critical - log and continue with other entities
    Warning,
    /// Informational - entity already processed or skipped
    Info,
}

/// Discord error variants.
///
/// Represents different error conditions that can occur during Discord operations.
#[derive(Debug, Clone, derive_more::Display)]
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

    /// Connection to Discord gateway failed with source error preserved.
    #[display("Connection failed: {}", source)]
    ConnectionFailedWithSource {
        /// Original connection error
        source: Arc<dyn std::error::Error + Send + Sync>,
    },

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

    /// Builder validation error (missing required fields, constraint violations).
    ///
    /// This indicates a programming error in builder usage, not a runtime data issue.
    #[display("Builder validation failed: {_0}")]
    BuilderValidationError(String),

    /// Unsupported Discord entity type.
    ///
    /// Encountered a Discord type that cannot be processed (e.g., unsupported channel type).
    #[display("Unsupported type: {_0}")]
    UnsupportedType(String),
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

impl From<DiscordErrorKind> for DiscordError {
    #[track_caller]
    fn from(kind: DiscordErrorKind) -> Self {
        Self::new(kind)
    }
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

    /// Create from a connection error with location tracking.
    #[track_caller]
    pub fn from_connection_error(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::new(DiscordErrorKind::ConnectionFailedWithSource {
            source: Arc::new(error),
        })
    }

    /// Get the severity level for this error.
    ///
    /// Used to determine whether event processing should abort or continue.
    pub fn severity(&self) -> DiscordErrorSeverity {
        use DiscordErrorKind::*;
        use DiscordErrorSeverity::*;

        match self.kind() {
            // Critical errors - can't continue processing
            ConnectionFailed(_) | ConnectionFailedWithSource { .. } => Critical,
            InvalidToken => Critical,
            DatabaseError(_) => Critical,
            BuilderValidationError(_) => Critical,
            ConfigurationError(_) => Critical,

            // Warnings - log and continue
            GuildNotFound(_) => Warning,
            ChannelNotFound(_) => Warning,
            UserNotFound(_) => Warning,
            RoleNotFound(_) => Warning,
            MessageSendFailed(_) => Warning,
            InteractionFailed(_) => Warning,
            InvalidId(_) => Warning,
            UnsupportedType(_) => Warning,

            // Info - expected conditions
            SerenityError(_) => Info,
            InsufficientPermissions(_) => Info,
        }
    }

    /// Check if this error is retryable.
    ///
    /// Used for retry logic and circuit breakers.
    pub fn is_retryable(&self) -> bool {
        use DiscordErrorKind::*;

        match self.kind() {
            // Network/transient errors - retryable
            ConnectionFailed(_) | ConnectionFailedWithSource { .. } => true,
            SerenityError(_) => true, // May be rate limit
            DatabaseError(_) => true, // May be temporary lock

            // Permanent errors - not retryable
            InvalidToken => false,
            BuilderValidationError(_) => false,
            GuildNotFound(_) => false,
            ChannelNotFound(_) => false,
            UserNotFound(_) => false,
            RoleNotFound(_) => false,
            InvalidId(_) => false,
            InsufficientPermissions(_) => false,
            MessageSendFailed(_) => false,
            InteractionFailed(_) => false,
            ConfigurationError(_) => false,
            UnsupportedType(_) => false,
        }
    }

    /// Get human-readable context for logging.
    ///
    /// Returns formatted string with error details and source location.
    pub fn error_context(&self) -> String {
        format!("{} at {}:{}", self.kind, self.file, self.line)
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
