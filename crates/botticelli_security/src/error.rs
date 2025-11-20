//! Security error types.

/// Specific security error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
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
    #[display("Rate limit exceeded for '{}': {} (limit: {} per {}s)", operation, reason, limit, window_secs)]
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
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Security Error: {} at line {} in {}", kind, line, file)]
pub struct SecurityError {
    /// The specific error kind
    pub kind: SecurityErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: &'static str,
}

impl SecurityError {
    /// Create a new security error with location tracking.
    #[track_caller]
    pub fn new(kind: SecurityErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }

    /// Get the error kind.
    pub fn kind(&self) -> &SecurityErrorKind {
        &self.kind
    }
}

/// Result type for security operations.
pub type SecurityResult<T> = Result<T, SecurityError>;

#[cfg(feature = "database")]
impl From<diesel::result::Error> for SecurityError {
    fn from(err: diesel::result::Error) -> Self {
        SecurityError::new(SecurityErrorKind::Database(err.to_string()))
    }
}
