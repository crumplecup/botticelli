//! Model provider errors.

use crate::GeminiErrorKind;

#[cfg(feature = "models")]
use botticelli_core::GenerateResponseBuilderError;

/// Ollama-specific error conditions (re-exported when ollama feature is enabled).
#[cfg(feature = "ollama")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum OllamaErrorKind {
    /// Ollama server not running at the specified address
    #[display("Ollama server not running at {}", _0)]
    ServerNotRunning(String),

    /// Requested model not found in Ollama
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    /// Failed to pull model from Ollama registry
    #[display("Failed to pull model: {}", _0)]
    ModelPullFailed(String),

    /// Ollama API returned an error
    #[display("API error: {}", _0)]
    ApiError(String),

    /// Invalid Ollama client configuration
    #[display("Invalid configuration: {}", _0)]
    InvalidConfiguration(String),

    /// Error converting between Ollama and Botticelli types
    #[display("Conversion error: {}", _0)]
    ConversionError(String),

    /// Builder error when constructing responses
    #[display("Builder error: {}", _0)]
    Builder(String),
}

/// Model provider-specific error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum ModelsErrorKind {
    /// Gemini-specific error
    #[display("Gemini: {}", _0)]
    Gemini(GeminiErrorKind),

    /// Builder error (derive_builder failures)
    #[display("Builder error: {}", _0)]
    Builder(String),

    /// Ollama-specific error (will be populated when ollama feature is enabled)
    #[cfg(feature = "ollama")]
    #[display("Ollama: {}", _0)]
    Ollama(OllamaErrorKind),
}

/// Model provider error with location tracking.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Models Error: {} at {}:{}", kind, file, line)]
pub struct ModelsError {
    /// The specific error kind
    pub kind: ModelsErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// Source file where error occurred
    pub file: &'static str,
}

impl ModelsError {
    /// Create a new models error.
    #[track_caller]
    pub fn new(kind: ModelsErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

#[cfg(feature = "models")]
impl From<GenerateResponseBuilderError> for ModelsError {
    #[track_caller]
    fn from(err: GenerateResponseBuilderError) -> Self {
        let kind = ModelsErrorKind::Builder(err.to_string());
        Self::new(kind)
    }
}

/// Result type for model operations.
pub type ModelsResult<T> = Result<T, ModelsError>;
