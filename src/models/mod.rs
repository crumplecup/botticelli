//! Model implementations for various LLM providers.

#[cfg(feature = "gemini")]
mod gemini;

#[cfg(feature = "gemini")]
pub use gemini::{GeminiClient, GeminiError, GeminiErrorKind};
