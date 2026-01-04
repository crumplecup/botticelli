//! Error types for Ollama client.

// Re-export from botticelli_error
pub use botticelli_error::{OllamaError, OllamaErrorKind};

pub type OllamaResult<T> = Result<T, OllamaError>;

