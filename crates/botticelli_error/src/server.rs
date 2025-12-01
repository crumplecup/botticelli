//! Error types for the local inference server.

/// Error kinds for server operations.
#[derive(Debug, Clone, derive_more::Display)]
pub enum ServerErrorKind {
    /// HTTP request failed: {0}
    #[display("HTTP request failed: {}", _0)]
    Http(String),

    /// API error: {0}
    #[display("API error: {}", _0)]
    Api(String),

    /// Failed to deserialize response: {0}
    #[display("Failed to deserialize response: {}", _0)]
    Deserialization(String),

    /// Stream error: {0}
    #[display("Stream error: {}", _0)]
    Stream(String),

    /// Configuration error: {0}
    #[display("Configuration error: {}", _0)]
    Configuration(String),

    /// Failed to start server: {0}
    #[display("Failed to start server: {}", _0)]
    ServerStartFailed(String),

    /// Failed to stop server: {0}
    #[display("Failed to stop server: {}", _0)]
    ServerStopFailed(String),

    /// Model download failed: {0}
    #[display("Model download failed: {}", _0)]
    ModelDownloadFailed(String),

    /// Models error: {0}
    #[cfg(feature = "models")]
    #[display("Models error: {}", _0)]
    Models(crate::ModelsError),
}

#[cfg(feature = "models")]
impl From<crate::ModelsError> for ServerErrorKind {
    fn from(error: crate::ModelsError) -> Self {
        Self::Models(error)
    }
}

/// Error wrapper with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Server Error: {} at line {} in {}", kind, line, file)]
pub struct ServerError {
    /// The error kind
    pub kind: ServerErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: &'static str,
}

impl ServerError {
    /// Create a new ServerError with automatic location tracking.
    #[track_caller]
    pub fn new(kind: ServerErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}
