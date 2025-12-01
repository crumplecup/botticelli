//! Ollama LLM client implementation.

mod client;
mod conversion;
mod error;

pub use client::OllamaClient;
pub use error::{OllamaError, OllamaErrorKind, OllamaResult};
