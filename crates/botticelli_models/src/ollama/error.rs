//! Error types for Ollama client.

use derive_more::{Display, Error};

// Re-export the shared OllamaErrorKind from botticelli_error
pub use botticelli_error::OllamaErrorKind;

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

/// Conversion from GenerateResponseBuilderError to OllamaError.
impl From<botticelli_core::GenerateResponseBuilderError> for OllamaError {
    #[track_caller]
    fn from(err: botticelli_core::GenerateResponseBuilderError) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind: OllamaErrorKind::Builder(err.to_string()),
            line: loc.line(),
            file: loc.file(),
        }
    }
}

/// Conversion from OllamaError to BotticelliError.
impl From<OllamaError> for botticelli_error::BotticelliError {
    fn from(err: OllamaError) -> Self {
        botticelli_error::BotticelliError::from(botticelli_error::ModelsError::new(
            botticelli_error::ModelsErrorKind::Ollama(err.kind),
        ))
    }
}
