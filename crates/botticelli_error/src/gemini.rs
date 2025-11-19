//! Gemini-specific error types and retry logic.

/// Gemini-specific error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub enum GeminiErrorKind {
    /// API key not found in environment
    #[display("GEMINI_API_KEY environment variable not set")]
    MissingApiKey,
    /// Failed to create Gemini client
    #[display("Failed to create Gemini client: {}", _0)]
    ClientCreation(String),
    /// API request failed
    #[display("Gemini API request failed: {}", _0)]
    ApiRequest(String),
    /// HTTP error with status code and message
    #[display("HTTP {} error: {}", status_code, message)]
    HttpError {
        /// HTTP status code
        status_code: u16,
        /// Error message
        message: String,
    },
    /// Multimodal inputs not yet supported
    #[display("Multimodal inputs not yet supported in simple Gemini wrapper")]
    MultimodalNotSupported,
    /// URL media sources not yet supported
    #[display("URL media sources not yet supported for Gemini")]
    UrlMediaNotSupported,
    /// Base64 decoding failed
    #[display("Base64 decode error: {}", _0)]
    Base64Decode(String),
    /// WebSocket connection failed
    #[display("WebSocket connection failed: {}", _0)]
    WebSocketConnection(String),
    /// WebSocket handshake failed (setup phase)
    #[display("WebSocket handshake failed: {}", _0)]
    WebSocketHandshake(String),
    /// Invalid message received from server
    #[display("Invalid server message: {}", _0)]
    InvalidServerMessage(String),
    /// Server sent goAway message
    #[display("Server disconnected: {}", _0)]
    ServerDisconnect(String),
    /// Stream was interrupted
    #[display("Stream interrupted: {}", _0)]
    StreamInterrupted(String),
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
    ///
    /// Returns `(initial_backoff_ms, max_retries, max_delay_secs)`.
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
///
/// # Examples
///
/// ```
/// use botticelli_error::{GeminiError, GeminiErrorKind};
///
/// let err = GeminiError::new(GeminiErrorKind::MissingApiKey);
/// assert!(format!("{}", err).contains("GEMINI_API_KEY"));
/// ```
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Gemini Error: {} at line {} in {}", kind, line, file)]
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

/// Trait for errors that support retry logic.
///
/// This trait allows error types to specify whether they should trigger a retry
/// and what retry strategy parameters to use.
///
/// # Examples
///
/// ```
/// use botticelli_error::{GeminiError, GeminiErrorKind, RetryableError};
///
/// let err = GeminiError::new(GeminiErrorKind::HttpError {
///     status_code: 503,
///     message: "Service unavailable".to_string(),
/// });
///
/// assert!(err.is_retryable());
/// let (backoff, retries, max_delay) = err.retry_strategy_params();
/// assert_eq!(backoff, 2000);  // 2 second initial backoff
/// assert_eq!(retries, 5);     // 5 retry attempts
/// ```
pub trait RetryableError {
    /// Returns true if this error should trigger a retry.
    ///
    /// Transient errors like 503 (service unavailable), 429 (rate limit),
    /// or network timeouts should return true. Permanent errors like 401
    /// (unauthorized) or 400 (bad request) should return false.
    fn is_retryable(&self) -> bool;

    /// Get retry strategy parameters for this error.
    ///
    /// Returns `(initial_backoff_ms, max_retries, max_delay_secs)`.
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
