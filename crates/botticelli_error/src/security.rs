//! Security error types.

/// Specific security error conditions.
#[cfg(feature = "mcp")]
use crate::tool;

use elicitation::{Prompt, Select};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    elicitation::Elicit,
)]
pub enum SecurityErrorKind {
    /// Permission denied for command
    #[display("Permission denied for command '{}': {}", command, reason)]
    PermissionDenied {
        /// Command that was denied
        command: String,
        /// Reason for denial
        reason: String,
    },

    /// Resource access denied
    #[display("Resource access denied: {} ({})", resource, reason)]
    ResourceAccessDenied {
        /// Resource that was denied
        resource: String,
        /// Reason for denial
        reason: String,
    },

    /// Validation failed
    #[display("Validation failed for '{}': {}", field, reason)]
    ValidationFailed {
        /// Field that failed validation
        field: String,
        /// Reason for failure
        reason: String,
    },

    /// Content filter violation
    #[display("Content filter violation: {}", reason)]
    ContentViolation {
        /// Reason for violation
        reason: String,
    },

    /// Rate limit exceeded
    #[display(
        "Rate limit exceeded for '{}': {} (limit: {} per {}s)",
        operation,
        reason,
        limit,
        window_secs
    )]
    RateLimitExceeded {
        /// Operation that exceeded rate limit
        operation: String,
        /// Reason for rate limit
        reason: String,
        /// Rate limit value
        limit: u32,
        /// Time window in seconds
        window_secs: u64,
    },

    /// Approval required
    #[display("Approval required for '{}': {}", operation, reason)]
    ApprovalRequired {
        /// Operation requiring approval
        operation: String,
        /// Reason approval is required
        reason: String,
        /// ID of pending action
        action_id: Option<String>,
    },

    /// Approval denied
    #[display("Approval denied for action '{}': {}", action_id, reason)]
    ApprovalDenied {
        /// Action ID that was denied
        action_id: String,
        /// Reason for denial
        reason: String,
    },

    /// Configuration error
    #[display("Configuration error: {}", _0)]
    Configuration(String),

    /// Database error
    #[cfg(feature = "database")]
    #[display("Database error: {}", _0)]
    Database(String),
}

/// Security error with location tracking.
#[derive(
    Debug,
    Clone,
    derive_more::Display,
    derive_more::Error,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    elicitation::Elicit,
)]
#[display("Security: {} at {}:{}", kind, file, line)]
pub struct SecurityError {
    /// The specific error kind
    pub kind: SecurityErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: String,
}

impl SecurityError {
    /// Create a new security error with location tracking.
    #[track_caller]
    #[cfg_attr(feature = "mcp", tool)]
    #[tracing::instrument(skip(kind), fields(kind = ?kind))]
    pub fn new(kind: SecurityErrorKind) -> Self {
        let location = std::panic::Location::caller();
        let error = Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        };
        tracing::error!(
            error_kind = ?error.kind,
            line = error.line,
            file = error.file,
            "Security error created"
        );
        error
    }

    /// Get the error kind.
    #[cfg_attr(feature = "mcp", tool)]
    #[tracing::instrument(skip(self))]
    pub fn kind(&self) -> &SecurityErrorKind {
        &self.kind
    }
}

#[cfg(feature = "database")]
impl From<diesel::result::Error> for SecurityError {
    #[track_caller]
    fn from(err: diesel::result::Error) -> Self {
        SecurityError::new(SecurityErrorKind::Database(err.to_string()))
    }
}

crate::impl_error_from_kind!(SecurityErrorKind => SecurityError);

/// Result type for security operations.
pub type SecurityResult<T> = Result<T, SecurityError>;
