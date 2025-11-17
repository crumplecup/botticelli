//! Error types for the Botticelli library.
//!
//! This crate provides the foundation error types used throughout the Botticelli ecosystem.

/// HTTP error wrapping reqwest errors with source location.
#[derive(Debug)]
pub struct HttpError {
    /// The underlying error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl HttpError {
    /// Create a new HttpError with the given message at the current location.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HTTP Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for HttpError {}

/// JSON serialization/deserialization error with source location.
#[derive(Debug)]
pub struct JsonError {
    /// The underlying error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl JsonError {
    /// Create a new JsonError with the given message at the current location.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JSON Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for JsonError {}

/// Configuration error with source location.
#[derive(Debug)]
pub struct ConfigError {
    /// Error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl ConfigError {
    /// Create a new ConfigError with the given message at the current location.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Configuration Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for ConfigError {}

/// Not implemented error with source location.
#[derive(Debug)]
pub struct NotImplementedError {
    /// Description of what is not implemented
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl NotImplementedError {
    /// Create a new NotImplementedError with the given message at the current location.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for NotImplementedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Not Implemented: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for NotImplementedError {}

/// Backend error with source location.
#[derive(Debug)]
pub struct BackendError {
    /// Error message
    pub message: String,
    /// Line number where the error occurred
    pub line: u32,
    /// File where the error occurred
    pub file: &'static str,
}

impl BackendError {
    /// Create a new BackendError with the given message at the current location.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        Self {
            message: message.into(),
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backend Error: {} at line {} in {}",
            self.message, self.line, self.file
        )
    }
}

impl std::error::Error for BackendError {}

/// Kinds of storage errors.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageErrorKind {
    /// Media not found at the specified location
    NotFound(String),
    /// Permission denied when accessing storage
    PermissionDenied(String),
    /// I/O error during storage operation
    Io(String),
    /// Invalid storage configuration
    InvalidConfig(String),
    /// Storage backend is unavailable
    Unavailable(String),
    /// Content hash mismatch (corruption detected)
    HashMismatch { expected: String, actual: String },
    /// Generic storage error with message
    Other(String),
}

impl std::fmt::Display for StorageErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageErrorKind::NotFound(path) => write!(f, "Media not found: {}", path),
            StorageErrorKind::PermissionDenied(msg) => {
                write!(f, "Permission denied: {}", msg)
            }
            StorageErrorKind::Io(msg) => write!(f, "I/O error: {}", msg),
            StorageErrorKind::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            StorageErrorKind::Unavailable(msg) => write!(f, "Storage unavailable: {}", msg),
            StorageErrorKind::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "Content hash mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            StorageErrorKind::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// Storage error with location tracking.
#[derive(Debug, Clone)]
pub struct StorageError {
    /// The kind of error that occurred
    pub kind: StorageErrorKind,
    /// Line number where error was created
    pub line: u32,
    /// File where error was created
    pub file: &'static str,
}

impl StorageError {
    /// Create a new storage error with automatic location tracking.
    #[track_caller]
    pub fn new(kind: StorageErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Storage Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for StorageError {}

/// Gemini-specific error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeminiErrorKind {
    /// API key not found in environment
    MissingApiKey,
    /// Failed to create Gemini client
    ClientCreation(String),
    /// API request failed
    ApiRequest(String),
    /// HTTP error with status code and message
    HttpError { status_code: u16, message: String },
    /// Multimodal inputs not yet supported
    MultimodalNotSupported,
    /// URL media sources not yet supported
    UrlMediaNotSupported,
    /// Base64 decoding failed
    Base64Decode(String),
    /// WebSocket connection failed
    WebSocketConnection(String),
    /// WebSocket handshake failed (setup phase)
    WebSocketHandshake(String),
    /// Invalid message received from server
    InvalidServerMessage(String),
    /// Server sent goAway message
    ServerDisconnect(String),
    /// Stream was interrupted
    StreamInterrupted(String),
}

impl std::fmt::Display for GeminiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeminiErrorKind::MissingApiKey => {
                write!(f, "GEMINI_API_KEY environment variable not set")
            }
            GeminiErrorKind::ClientCreation(msg) => {
                write!(f, "Failed to create Gemini client: {}", msg)
            }
            GeminiErrorKind::ApiRequest(msg) => write!(f, "Gemini API request failed: {}", msg),
            GeminiErrorKind::HttpError {
                status_code,
                message,
            } => write!(f, "HTTP {} error: {}", status_code, message),
            GeminiErrorKind::MultimodalNotSupported => write!(
                f,
                "Multimodal inputs not yet supported in simple Gemini wrapper"
            ),
            GeminiErrorKind::UrlMediaNotSupported => {
                write!(f, "URL media sources not yet supported for Gemini")
            }
            GeminiErrorKind::Base64Decode(msg) => write!(f, "Base64 decode error: {}", msg),
            GeminiErrorKind::WebSocketConnection(msg) => {
                write!(f, "WebSocket connection failed: {}", msg)
            }
            GeminiErrorKind::WebSocketHandshake(msg) => {
                write!(f, "WebSocket handshake failed: {}", msg)
            }
            GeminiErrorKind::InvalidServerMessage(msg) => {
                write!(f, "Invalid server message: {}", msg)
            }
            GeminiErrorKind::ServerDisconnect(msg) => {
                write!(f, "Server disconnected: {}", msg)
            }
            GeminiErrorKind::StreamInterrupted(msg) => {
                write!(f, "Stream interrupted: {}", msg)
            }
        }
    }
}

impl GeminiErrorKind {
    /// Check if this error type should be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            GeminiErrorKind::HttpError { status_code, .. } => {
                matches!(*status_code, 408 | 429 | 500 | 502 | 503 | 504)
            }
            GeminiErrorKind::WebSocketConnection(_) => true,
            GeminiErrorKind::WebSocketHandshake(_) => true,
            GeminiErrorKind::StreamInterrupted(_) => true,
            _ => false,
        }
    }

    /// Get retry strategy parameters for this error type.
    pub fn retry_strategy_params(&self) -> (u64, usize, u64) {
        match self {
            GeminiErrorKind::HttpError { status_code, .. } => match *status_code {
                429 => (5000, 3, 40),
                503 => (2000, 5, 60),
                500 | 502 | 504 => (1000, 3, 8),
                408 => (2000, 4, 30),
                _ => (2000, 5, 60),
            },
            GeminiErrorKind::WebSocketConnection(_) => (2000, 5, 60),
            GeminiErrorKind::WebSocketHandshake(_) => (2000, 5, 60),
            GeminiErrorKind::StreamInterrupted(_) => (1000, 3, 10),
            _ => (2000, 5, 60),
        }
    }
}

/// Gemini error with source location tracking.
#[derive(Debug, Clone)]
pub struct GeminiError {
    /// The kind of error that occurred
    pub kind: GeminiErrorKind,
    /// Line number where error was created
    pub line: u32,
    /// File where error was created
    pub file: &'static str,
}

impl GeminiError {
    /// Create a new GeminiError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: GeminiErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for GeminiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gemini Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for GeminiError {}

/// Trait for errors that support retry logic.
///
/// This trait allows error types to specify whether they should trigger a retry
/// and what retry strategy parameters to use.
pub trait RetryableError {
    /// Returns true if this error should trigger a retry.
    ///
    /// Transient errors like 503 (service unavailable), 429 (rate limit),
    /// or network timeouts should return true. Permanent errors like 401
    /// (unauthorized) or 400 (bad request) should return false.
    fn is_retryable(&self) -> bool;

    /// Get retry strategy parameters for this error.
    ///
    /// Returns (initial_backoff_ms, max_retries, max_delay_secs).
    /// Default implementation returns standard parameters.
    ///
    /// Override this to provide error-specific retry strategies:
    /// - Rate limit errors (429): Longer delays, fewer retries
    /// - Server overload (503): Standard delays, more patient
    /// - Server errors (500): Quick retries, fail fast
    fn retry_strategy_params(&self) -> (u64, usize, u64) {
        (2000, 5, 60) // Default: 2s initial, 5 retries, 60s cap
    }
}

impl RetryableError for GeminiError {
    fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }

    fn retry_strategy_params(&self) -> (u64, usize, u64) {
        self.kind.retry_strategy_params()
    }
}

/// Database error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DatabaseErrorKind {
    /// Connection failed
    Connection(String),
    /// Query execution failed
    Query(String),
    /// Serialization/deserialization error
    Serialization(String),
    /// Migration error
    Migration(String),
    /// Record not found
    NotFound,
    /// Table not found
    TableNotFound(String),
    /// Schema inference error
    SchemaInference(String),
}

impl std::fmt::Display for DatabaseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseErrorKind::Connection(msg) => write!(f, "Database connection error: {}", msg),
            DatabaseErrorKind::Query(msg) => write!(f, "Database query error: {}", msg),
            DatabaseErrorKind::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            DatabaseErrorKind::Migration(msg) => write!(f, "Migration error: {}", msg),
            DatabaseErrorKind::NotFound => write!(f, "Record not found"),
            DatabaseErrorKind::TableNotFound(table) => {
                write!(f, "Table '{}' not found in database", table)
            }
            DatabaseErrorKind::SchemaInference(msg) => {
                write!(f, "Schema inference error: {}", msg)
            }
        }
    }
}

/// Database error with source location tracking.
#[derive(Debug, Clone)]
pub struct DatabaseError {
    /// The kind of error that occurred
    pub kind: DatabaseErrorKind,
    /// Line number where error was created
    pub line: u32,
    /// File where error was created
    pub file: &'static str,
}

impl DatabaseError {
    /// Create a new DatabaseError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: DatabaseErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Database Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for DatabaseError {}

/// Specific error conditions for narrative operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NarrativeErrorKind {
    /// Failed to read narrative file
    FileRead(String),
    /// Failed to parse TOML content
    TomlParse(String),
    /// Table of contents is empty
    EmptyToc,
    /// Act referenced in table of contents does not exist in acts map
    MissingAct(String),
    /// Act prompt is empty or contains only whitespace
    EmptyPrompt(String),
    /// Template field required but not set
    MissingTemplate,
    /// Failed to assemble prompt with schema injection
    PromptAssembly { act: String, message: String },
}

impl std::fmt::Display for NarrativeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NarrativeErrorKind::FileRead(msg) => write!(f, "Failed to read narrative file: {}", msg),
            NarrativeErrorKind::TomlParse(msg) => write!(f, "Failed to parse TOML: {}", msg),
            NarrativeErrorKind::EmptyToc => write!(f, "Table of contents (toc.order) cannot be empty"),
            NarrativeErrorKind::MissingAct(act) => write!(f, "Act '{}' referenced in toc.order does not exist in acts map", act),
            NarrativeErrorKind::EmptyPrompt(act) => write!(f, "Act '{}' has an empty prompt", act),
            NarrativeErrorKind::MissingTemplate => write!(f, "Template field is required for prompt assembly"),
            NarrativeErrorKind::PromptAssembly { act, message } => write!(f, "Failed to assemble prompt for act '{}': {}", act, message),
        }
    }
}

/// Error type for narrative operations.
#[derive(Debug, Clone)]
pub struct NarrativeError {
    /// The specific error condition
    pub kind: NarrativeErrorKind,
    /// Line number where the error occurred
    pub line: u32,
    /// Source file where the error occurred
    pub file: &'static str,
}

impl NarrativeError {
    /// Create a new NarrativeError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: NarrativeErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for NarrativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Narrative Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for NarrativeError {}

/// Crate-level error variants.
///
/// This is the foundation error enum. Additional variants will be added
/// by other botticelli crates during the workspace migration.
#[derive(Debug, derive_more::From)]
pub enum BotticelliErrorKind {
    /// HTTP error
    Http(HttpError),
    /// JSON serialization/deserialization error
    Json(JsonError),
    /// Generic backend error
    Backend(BackendError),
    /// Configuration error
    Config(ConfigError),
    /// Feature not yet implemented
    NotImplemented(NotImplementedError),
    /// Storage error (Phase 3)
    Storage(StorageError),
    /// Gemini error (Phase 4)
    Gemini(GeminiError),
    /// Database error (Phase 3.5)
    Database(DatabaseError),
    /// Narrative error (Phase 3.5)
    Narrative(NarrativeError),
}

impl std::fmt::Display for BotticelliErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotticelliErrorKind::Http(e) => write!(f, "{}", e),
            BotticelliErrorKind::Json(e) => write!(f, "{}", e),
            BotticelliErrorKind::Backend(e) => write!(f, "{}", e),
            BotticelliErrorKind::Config(e) => write!(f, "{}", e),
            BotticelliErrorKind::NotImplemented(e) => write!(f, "{}", e),
            BotticelliErrorKind::Storage(e) => write!(f, "{}", e),
            BotticelliErrorKind::Gemini(e) => write!(f, "{}", e),
            BotticelliErrorKind::Database(e) => write!(f, "{}", e),
            BotticelliErrorKind::Narrative(e) => write!(f, "{}", e),
        }
    }
}

/// Botticelli error with kind discrimination.
#[derive(Debug)]
pub struct BotticelliError(Box<BotticelliErrorKind>);

impl BotticelliError {
    /// Create a new error from a kind.
    pub fn new(kind: BotticelliErrorKind) -> Self {
        Self(Box::new(kind))
    }

    /// Get the error kind.
    pub fn kind(&self) -> &BotticelliErrorKind {
        &self.0
    }
}

impl std::fmt::Display for BotticelliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Botticelli Error: {}", self.0)
    }
}

impl std::error::Error for BotticelliError {}

// Generic From implementation for any type that converts to BotticelliErrorKind
impl<T> From<T> for BotticelliError
where
    T: Into<BotticelliErrorKind>,
{
    fn from(err: T) -> Self {
        Self::new(err.into())
    }
}

/// Result type for Botticelli operations.
pub type BotticelliResult<T> = std::result::Result<T, BotticelliError>;
