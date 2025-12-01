//! Error types for Ollama client.

use derive_more::{Display, Error};

/// Ollama-specific error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum OllamaErrorKind {
    #[display("Ollama server not running at {}", _0)]
    ServerNotRunning(String),

    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    #[display("Failed to pull model: {}", _0)]
    ModelPullFailed(String),

    #[display("API error: {}", _0)]
    ApiError(String),

    #[display("Invalid configuration: {}", _0)]
    InvalidConfiguration(String),
}

/// Ollama error with location tracking.
#[derive(Debug, Clone, Display, Error)]
#[display("Ollama Error: {} at {}:{}", kind, file, line)]
pub struct OllamaError {
    pub kind: OllamaErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl OllamaError {
    #[track_caller]
    pub fn new(kind: OllamaErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

pub type OllamaResult<T> = Result<T, OllamaError>;
