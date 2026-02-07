//! Model provider errors.

use rmcp::tool;

use std::sync::Arc;

use crate::GeminiErrorKind;

/// Ollama-specific error conditions (re-exported when ollama feature is enabled).
#[cfg(feature = "ollama")]
#[derive(Debug, Clone, derive_more::Display)]
pub enum OllamaErrorKind {
    /// Ollama server not running at the specified address
    #[display("Ollama server not running at {}", _0)]
    ServerNotRunning(String),

    /// Requested model not found in Ollama
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    /// Failed to pull model from Ollama registry
    #[display("Failed to pull model: {}", _0)]
    ModelPullFailed(Arc<ollama_rs::error::OllamaError>),

    /// Ollama API returned an error
    #[display("API error: {}", _0)]
    ApiError(Arc<ollama_rs::error::OllamaError>),

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

/// Ollama error with location tracking.
#[cfg(feature = "ollama")]
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Ollama Error: {} at {}:{}", kind, file, line)]
pub struct OllamaError {
    /// The specific error condition
    kind: OllamaErrorKind,
    /// Line number where error occurred
    line: u32,
    /// Source file where error occurred
    file: String,
}

#[cfg(feature = "ollama")]
impl OllamaError {
    /// Create a new Ollama error.
    #[tool]
    #[track_caller]
    pub fn new(kind: OllamaErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }

    #[tool]
    /// Get the error kind.
    pub fn kind(&self) -> &OllamaErrorKind {
        &self.kind
    }
}

/// Result type for Ollama operations.
#[cfg(feature = "ollama")]
pub type OllamaResult<T> = Result<T, OllamaError>;

/// Anthropic-specific error conditions (re-exported when anthropic feature is enabled).
#[cfg(feature = "anthropic")]
#[derive(Debug, Clone, derive_more::Display)]
pub enum AnthropicErrorKind {
    /// HTTP error (connection, timeout, etc.)
    #[display("HTTP error: {}", _0)]
    Http(String),

    /// Reqwest error (network, connection, etc.)
    #[display("Reqwest error: {}", _0)]
    Reqwest(Arc<reqwest::Error>),

    /// Anthropic API returned an error
    #[display("API error (status {}): {message}", status)]
    /// API error with HTTP status and message
    ApiError {
        /// HTTP status code
        status: u16,
        /// Error message from API
        message: String,
    },

    /// Failed to parse response
    #[display("Parse error: {}", _0)]
    Parse(String),

    /// Invalid API key
    #[display("Invalid API key")]
    InvalidApiKey,

    /// Rate limit exceeded
    #[display("Rate limit exceeded: {}", _0)]
    RateLimitExceeded(String),

    /// Model not found
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    /// Environment variable error
    #[display("Environment variable error: {}", _0)]
    EnvVar(Arc<std::env::VarError>),

    /// Error converting between Anthropic and Botticelli types
    #[display("Conversion error: {}", _0)]
    ConversionError(String),

    /// Builder error when constructing responses
    #[display("Builder error: {}", _0)]
    Builder(String),

    /// Feature not supported
    #[display("Unsupported: {}", _0)]
    Unsupported(String),

    /// Invalid role for message
    #[display("Invalid role: {}", _0)]
    InvalidRole(String),
}

/// Anthropic error with location tracking.
#[cfg(feature = "anthropic")]
#[derive(Debug, Clone, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Anthropic Error: {} at line {} in {}", kind, line, file)]
pub struct AnthropicError {
    /// The kind of error that occurred
    kind: AnthropicErrorKind,
    /// Line number where error was created
    line: u32,
    /// File where error was created
    file: String,
}

#[cfg(feature = "anthropic")]
impl AnthropicError {
    /// Creates a new Anthropic error with location tracking.
    #[tool]
    #[track_caller]
    pub fn new(kind: AnthropicErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }
}

/// Model provider-specific error conditions.
#[derive(Debug, Clone, derive_more::Display)]
pub enum ModelsErrorKind {
    /// Gemini-specific error
    #[display("Gemini: {}", _0)]
    Gemini(GeminiErrorKind),

    /// Builder error (derive_builder failures)
    #[display("Builder error: {}", _0)]
    Builder(String),

    /// Gemini client creation error
    #[display("Gemini client error: {}", _0)]
    #[cfg(feature = "models")]
    GeminiClient(std::sync::Arc<gemini_rust::client::Error>),

    /// Ollama-specific error (will be populated when ollama feature is enabled)
    #[cfg(feature = "ollama")]
    #[display("Ollama: {}", _0)]
    Ollama(OllamaErrorKind),

    /// Anthropic-specific error (will be populated when anthropic feature is enabled)
    #[cfg(feature = "anthropic")]
    #[display("Anthropic: {}", _0)]
    Anthropic(AnthropicErrorKind),

    /// OpenAI-compatible API error
    #[display("OpenAI Compatible: {}", _0)]
    OpenAI(crate::OpenAIErrorKind),

    /// Invalid role for message
    #[display("Invalid role: {}", _0)]
    InvalidRole(String),

    /// Token counting failed
    #[display("Token counting failed: {}", _0)]
    TokenCountingFailed(String),

    /// Tiktoken decode error
    #[display("Tiktoken decode error: {}", _0)]
    TiktokenDecode(String),

    /// Tiktoken decode key error (invalid token)
    #[display("Tiktoken decode key error: token {}", _0)]
    TiktokenDecodeKey(u32),

    /// Tiktoken initialization failed
    #[display("Tiktoken initialization failed: {}", _0)]
    Tiktoken(std::sync::Arc<anyhow::Error>),

    /// Serialization error (JSON encoding/decoding failures)
    #[display("Serialization error: {}", _0)]
    Serialization(std::sync::Arc<serde_json::Error>),
}

/// Model provider error with location tracking.
#[derive(Debug, derive_more::Display, derive_more::Error, derive_getters::Getters)]
#[display("Models Error: {} at {}:{}", kind, file, line)]
pub struct ModelsError {
    /// The specific error kind
    kind: ModelsErrorKind,
    /// Line number where error occurred
    line: u32,
    /// Source file where error occurred
    file: String,
}

impl ModelsError {
    /// Create a new models error.
    #[tool]
    #[track_caller]
    pub fn new(kind: ModelsErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file().to_string(),
        }
    }
}

// Gemini error conversions
#[cfg(feature = "models")]
crate::impl_chained_error_bridge!(crate::GeminiErrorKind => crate::GeminiError => ModelsErrorKind => ModelsError);

#[cfg(feature = "models")]
crate::chain_error_kind!(crate::GeminiErrorKind => ModelsErrorKind, Gemini);

#[cfg(feature = "models")]
impl From<crate::GeminiError> for ModelsError {
    #[track_caller]
    fn from(err: crate::GeminiError) -> Self {
        ModelsError::new(ModelsErrorKind::Gemini(err.kind.clone()))
    }
}

#[cfg(feature = "ollama")]
crate::impl_error_from_kind!(OllamaErrorKind => OllamaError);

#[cfg(feature = "ollama")]
crate::impl_chained_error_bridge!(OllamaErrorKind => OllamaError => ModelsErrorKind => ModelsError);

#[cfg(feature = "ollama")]
crate::chain_error_kind!(OllamaErrorKind => ModelsErrorKind, Ollama);

#[cfg(feature = "ollama")]
impl From<OllamaError> for ModelsError {
    #[track_caller]
    fn from(err: OllamaError) -> Self {
        ModelsError::new(ModelsErrorKind::Ollama(err.kind().clone()))
    }
}

#[cfg(feature = "ollama")]
impl From<OllamaError> for crate::BotticelliErrorKind {
    #[track_caller]
    fn from(err: OllamaError) -> Self {
        crate::BotticelliErrorKind::Models(ModelsError::from(err))
    }
}

#[cfg(feature = "anthropic")]
crate::impl_error_from_kind!(AnthropicErrorKind => AnthropicError);

#[cfg(feature = "anthropic")]
crate::impl_chained_error_bridge!(crate::AnthropicErrorKind => crate::AnthropicError => ModelsErrorKind => ModelsError);

#[cfg(feature = "anthropic")]
crate::chain_error_kind!(crate::AnthropicErrorKind => ModelsErrorKind, Anthropic);

#[cfg(feature = "anthropic")]
impl From<AnthropicError> for ModelsError {
    #[track_caller]
    fn from(err: AnthropicError) -> Self {
        ModelsError::new(ModelsErrorKind::Anthropic(err.kind().clone()))
    }
}

#[cfg(feature = "anthropic")]
impl From<AnthropicError> for crate::BotticelliErrorKind {
    fn from(err: AnthropicError) -> Self {
        crate::BotticelliErrorKind::Models(err.into())
    }
}

// OpenAI error bridge
impl From<crate::OpenAIErrorKind> for ModelsError {
    #[track_caller]
    fn from(kind: crate::OpenAIErrorKind) -> Self {
        ModelsError::new(ModelsErrorKind::OpenAI(kind))
    }
}

crate::impl_error_from_kind!(ModelsErrorKind => ModelsError);

/// Result type for model operations.
pub type ModelsResult<T> = Result<T, ModelsError>;
