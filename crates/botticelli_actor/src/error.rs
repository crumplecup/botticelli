//! Error types for actor operations.

use std::path::PathBuf;

/// Result type for actor operations.
pub type ActorResult<T> = Result<T, ActorError>;

/// Error kinds for actor operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ActorErrorKind {
    // Recoverable errors (retry, skip, continue)
    /// Platform temporary failure.
    #[display("Platform temporary failure: {}", _0)]
    PlatformTemporary(String),

    /// Rate limit exceeded.
    #[display("Rate limit exceeded: retry after {}s", _0)]
    RateLimitExceeded(u64),

    /// Content validation failed.
    #[display("Content validation failed: {}", _0)]
    ValidationFailed(String),

    /// Resource temporarily unavailable.
    #[display("Resource temporarily unavailable: {}", _0)]
    ResourceUnavailable(String),

    // Unrecoverable errors (stop execution)
    /// Authentication failed.
    #[display("Authentication failed: {}", _0)]
    AuthenticationFailed(String),

    /// Configuration invalid.
    #[display("Configuration invalid: {}", _0)]
    InvalidConfiguration(String),

    /// Platform permanently failed.
    #[display("Platform permanently failed: {}", _0)]
    PlatformPermanent(String),

    /// Database connection lost.
    #[display("Database connection lost: {}", _0)]
    DatabaseFailed(String),

    /// Skill not found in registry.
    #[display("Skill not found: {}", _0)]
    SkillNotFound(String),

    /// Knowledge table not found.
    #[display("Knowledge table not found: {}", _0)]
    KnowledgeTableNotFound(String),

    /// File I/O error.
    #[display("File I/O error: {} ({})", path.display(), message)]
    FileIo {
        /// Path that caused the error.
        path: PathBuf,
        /// Error message.
        message: String,
    },

    /// TOML parsing error.
    #[display("TOML parsing error: {}", _0)]
    TomlParse(String),

    /// JSON serialization error.
    #[display("JSON error: {}", _0)]
    JsonError(String),
}

impl ActorErrorKind {
    /// Check if error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PlatformTemporary(_)
                | Self::RateLimitExceeded(_)
                | Self::ValidationFailed(_)
                | Self::ResourceUnavailable(_)
        )
    }
}

/// Actor error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Actor error: {} at {}:{}", kind, file, line)]
pub struct ActorError {
    /// Error kind.
    pub kind: ActorErrorKind,
    /// Line number where error occurred.
    pub line: u32,
    /// File where error occurred.
    pub file: &'static str,
}

impl ActorError {
    /// Create a new actor error.
    #[track_caller]
    pub fn new(kind: ActorErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }

    /// Check if error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        self.kind.is_recoverable()
    }
}

impl From<std::io::Error> for ActorError {
    #[track_caller]
    fn from(e: std::io::Error) -> Self {
        Self::new(ActorErrorKind::FileIo {
            path: PathBuf::from("unknown"),
            message: e.to_string(),
        })
    }
}

impl From<toml::de::Error> for ActorError {
    #[track_caller]
    fn from(e: toml::de::Error) -> Self {
        Self::new(ActorErrorKind::TomlParse(e.to_string()))
    }
}

impl From<serde_json::Error> for ActorError {
    #[track_caller]
    fn from(e: serde_json::Error) -> Self {
        Self::new(ActorErrorKind::JsonError(e.to_string()))
    }
}
