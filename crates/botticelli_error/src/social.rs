//! Social media bot command errors.
//!
//! This module provides error types for bot command execution across
//! different social media platforms (Discord, Slack, etc.).

#[cfg(feature = "mcp")]
use crate::tool;

use derive_getters::Getters;
use derive_more::{Display, Error};
use std::sync::Arc;

/// Result type for bot command operations.
pub type BotCommandResult<T> = Result<T, BotCommandError>;

/// Specific bot command error conditions.
#[derive(Debug, Clone, Display)]
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
    #[display(
        "Invalid argument '{}' for command '{}': {}",
        arg_name,
        command,
        reason
    )]
    InvalidArgument {
        /// Command that received invalid argument
        command: String,
        /// Name of the invalid argument
        arg_name: String,
        /// Reason why the argument is invalid
        reason: String,
    },

    /// Invalid argument with source error preserved.
    #[display("Invalid argument '{}': {}", arg_name, source)]
    InvalidArgumentWithSource {
        /// Name of the invalid argument
        arg_name: String,
        /// Original parse error
        source: Arc<dyn std::error::Error + Send + Sync>,
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
    #[display(
        "Rate limit exceeded for '{}': retry after {} seconds",
        command,
        retry_after
    )]
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

    /// Command execution failed in wrapped executor.
    #[display("Execution failed for '{}': {}", command, source)]
    ExecutionFailed {
        /// Command that failed
        command: String,
        /// Source error from wrapped executor
        source: String,
    },

    /// Security policy violation with source error preserved.
    #[display("Security violation: {}", source)]
    SecurityViolation {
        /// Original security error
        source: Arc<dyn std::error::Error + Send + Sync>,
    },

    /// JSON serialization/deserialization error with source preserved.
    #[display("Serialization failed: {}", source)]
    SerializationFailed {
        /// Original serialization error
        source: Arc<dyn std::error::Error + Send + Sync>,
    },

    /// API error with source preserved.
    #[display("API error in '{}': {}", command, source)]
    ApiErrorWithSource {
        /// Command that failed
        command: String,
        /// Original API error
        source: Arc<dyn std::error::Error + Send + Sync>,
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
    file: String,
}

impl BotCommandError {
    /// Create a new bot command error with location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn new(kind: BotCommandErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }

    /// Create from a security error with location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn from_security_error(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::new(BotCommandErrorKind::SecurityViolation {
            source: Arc::new(error),
        })
    }

    /// Create from a serialization error with location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn from_serialization_error(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::new(BotCommandErrorKind::SerializationFailed {
            source: Arc::new(error),
        })
    }

    /// Create from an API error with location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn from_api_error(
        command: impl Into<String>,
        error: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::new(BotCommandErrorKind::ApiErrorWithSource {
            command: command.into(),
            source: Arc::new(error),
        })
    }

    /// Create from an invalid argument parse error with location tracking.
    #[cfg_attr(feature = "mcp", tool)]
    #[track_caller]
    pub fn from_parse_error(
        arg_name: impl Into<String>,
        error: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::new(BotCommandErrorKind::InvalidArgumentWithSource {
            arg_name: arg_name.into(),
            source: Arc::new(error),
        })
    }
}

impl From<BotCommandErrorKind> for BotCommandError {
    #[track_caller]
    fn from(kind: BotCommandErrorKind) -> Self {
        Self::new(kind)
    }
}
