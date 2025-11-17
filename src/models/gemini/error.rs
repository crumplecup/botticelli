//! Error types for Gemini API operations.

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
    ///
    /// Returns true for transient errors that may resolve with retry:
    /// - HTTP 429 (rate limit exceeded)
    /// - HTTP 500, 502, 503, 504 (server errors)
    /// - HTTP 408 (request timeout)
    /// - WebSocket connection/stream errors
    ///
    /// Returns false for permanent errors that won't change with retry:
    /// - HTTP 400, 401, 403, 404 (client errors)
    /// - Missing API key
    /// - Unsupported features
    pub fn is_retryable(&self) -> bool {
        match self {
            GeminiErrorKind::HttpError { status_code, .. } => {
                matches!(
                    *status_code,
                    408 | 429 | 500 | 502 | 503 | 504
                )
            }
            GeminiErrorKind::WebSocketConnection(_) => true,
            GeminiErrorKind::StreamInterrupted(_) => true,
            // Most other errors are permanent
            _ => false,
        }
    }
}

/// Gemini error with source location tracking.
#[derive(Debug, Clone)]
pub struct GeminiError {
    pub kind: GeminiErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl GeminiError {
    /// Create a new GeminiError with the given kind at the current location.
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

impl crate::rate_limit::RetryableError for GeminiError {
    fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }
}

/// Result type for Gemini operations.
pub type GeminiResult<T> = Result<T, GeminiError>;
