//! Gemini-specific error types and retry logic.

/// Gemini-specific error conditions.
#[derive(Debug, Clone, derive_more::Display)]
pub enum GeminiErrorKind {
    /// API key not found in environment
    #[display("GEMINI_API_KEY environment variable not set")]
    MissingApiKey,
    /// Failed to create Gemini client (captures gemini-rust error)
    #[display("Failed to create Gemini client: {}", _0)]
    #[cfg(feature = "models")]
    ClientCreation(std::sync::Arc<gemini_rust::client::Error>),
    /// Gemini-rust library error (runtime errors)
    #[display("Gemini-rust error: {}", _0)]
    #[cfg(feature = "models")]
    GeminiRust(std::sync::Arc<gemini_rust::client::Error>),
    /// Live API client not available
    #[display("Live API client not available: {}", _0)]
    LiveClientUnavailable(String),
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
    /// WebSocket connection failed (captures tokio-tungstenite error)
    #[display("WebSocket connection failed: {}", _0)]
    #[cfg(feature = "models")]
    WebSocketConnection(std::sync::Arc<tokio_tungstenite::tungstenite::Error>),
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
    /// Tungstenite websocket error (runtime errors)
    #[display("Tungstenite error: {}", _0)]
    #[cfg(feature = "models")]
    Tungstenite(std::sync::Arc<tokio_tungstenite::tungstenite::Error>),
    /// Builder error (derive_builder failures)
    #[display("Builder error: {}", _0)]
    BuilderError(String),
    /// Mutex was poisoned (panic occurred while holding lock)
    #[display("Mutex poisoned: {}", _0)]
    MutexPoisoned(String),
    /// Invalid model name
    #[display("Invalid model: {}", _0)]
    InvalidModel(String),
    /// JSON serialization/deserialization failed
    #[display("Serialization error: {}", _0)]
    #[cfg(feature = "models")]
    Serialization(std::sync::Arc<serde_json::Error>),
    /// Tiktoken encoding failed
    #[display("Tiktoken error: {}", _0)]
    Tiktoken(String),
}

impl GeminiErrorKind {
    /// Check if this error type should be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            GeminiErrorKind::HttpError { status_code, .. } => {
                matches!(*status_code, 408 | 429 | 500 | 502 | 503 | 504)
            }
            #[cfg(feature = "models")]
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
            #[cfg(feature = "models")]
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
    pub file: String,
}

impl GeminiError {
    /// Create a new GeminiError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: GeminiErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file().to_string(),
        }
    }
}

/// Convert gemini-rust client errors to GeminiError
#[cfg(feature = "models")]
impl From<gemini_rust::client::Error> for GeminiError {
    #[track_caller]
    fn from(e: gemini_rust::client::Error) -> Self {
        Self::new(GeminiErrorKind::ClientCreation(std::sync::Arc::new(e)))
    }
}

impl botticelli_interface::RetryableError for GeminiError {
    fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }

    fn retry_strategy_params(&self) -> (u64, usize, u64) {
        self.kind.retry_strategy_params()
    }
}

// Enable GeminiErrorKind -> GeminiError conversion
crate::impl_error_from_kind!(GeminiErrorKind => GeminiError);

// Enable GeminiErrorKind -> BotticelliErrorKind conversion (via GeminiError)
impl From<GeminiErrorKind> for crate::BotticelliErrorKind {
    #[track_caller]
    fn from(kind: GeminiErrorKind) -> Self {
        GeminiError::from(kind).into()
    }
}
