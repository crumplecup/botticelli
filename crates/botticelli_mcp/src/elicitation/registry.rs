//! Registry operations trait for elicitation data types.

// Re-export from botticelli_interface when available
#[cfg(any(feature = "gemini", feature = "anthropic", feature = "ollama", feature = "huggingface", feature = "groq"))]
pub use botticelli_interface::RegistryOperations;
