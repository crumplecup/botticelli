//! Model provider errors.

use std::sync::Arc;

use crate::GeminiErrorKind;

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

    /// Invalid Anthropic client configuration
    #[display("Invalid configuration: {}", _0)]
    InvalidConfiguration(String),

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
    file: &'static str,
}

#[cfg(feature = "anthropic")]
impl AnthropicError {
    /// Creates a new Anthropic error with location tracking.
    #[track_caller]
    pub fn new(kind: AnthropicErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

/// HuggingFace-specific error conditions.
#[cfg(feature = "huggingface")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum HuggingFaceErrorKind {
    /// API error from HuggingFace
    #[display("API error: {}", _0)]
    Api(String),

    /// Rate limit exceeded
    #[display("Rate limit exceeded")]
    RateLimit,

    /// Model not found
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    /// Invalid request
    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),

    /// Request conversion failed
    #[display("Request conversion failed: {}", _0)]
    RequestConversion(String),

    /// Response conversion failed
    #[display("Response conversion failed: {}", _0)]
    ResponseConversion(String),
}

/// Errors specific to Groq models.
#[cfg(feature = "groq")]
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum GroqErrorKind {
    /// API error from Groq
    #[display("API error: {}", _0)]
    Api(String),

    /// Rate limit exceeded
    #[display("Rate limit exceeded")]
    RateLimit,

    /// Model not found
    #[display("Model not found: {}", _0)]
    ModelNotFound(String),

    /// Invalid request
    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),

    /// Request conversion failed
    #[display("Request conversion failed: {}", _0)]
    RequestConversion(String),

    /// Response conversion failed
    #[display("Response conversion failed: {}", _0)]
    ResponseConversion(String),

    /// Environment variable error
    #[display("Environment variable error: {}", _0)]
    EnvVar(std::env::VarError),
}

/// Groq error with location tracking.
#[cfg(feature = "groq")]
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Groq: {} at {}:{}", kind, file, line)]
pub struct GroqError {
    /// The specific error condition
    pub kind: GroqErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// Source file where error occurred
    pub file: &'static str,
}

#[cfg(feature = "groq")]
impl GroqError {
    /// Create a new Groq error.
    #[track_caller]
    pub fn new(kind: GroqErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}

/// Model provider-specific error conditions.
#[derive(Debug, Clone, derive_more::Display, derive_more::From)]
pub enum ModelsErrorKind {
    /// Gemini-specific error
    #[display("Gemini: {}", _0)]
    Gemini(GeminiErrorKind),

    /// Builder error (derive_builder failures)
    #[display("Builder error: {}", _0)]
    #[from(ignore)]
    Builder(String),

    /// Gemini client creation error
    #[display("Gemini client error: {}", _0)]
    #[from(ignore)]
    #[cfg(feature = "models")]
    GeminiClient(std::sync::Arc<gemini_rust::client::Error>),

    /// Ollama-specific error (will be populated when ollama feature is enabled)
    #[cfg(feature = "ollama")]
    #[display("Ollama: {}", _0)]
    Ollama(OllamaErrorKind),

    /// Anthropic-specific error (will be populated when anthropic feature is enabled)
    #[cfg(feature = "anthropic")]
    #[display("Anthropic: {}", _0)]
    #[from(AnthropicErrorKind)]
    Anthropic(AnthropicErrorKind),

    /// HuggingFace-specific error
    #[cfg(feature = "huggingface")]
    #[display("HuggingFace: {}", _0)]
    #[from(HuggingFaceErrorKind)]
    HuggingFace(HuggingFaceErrorKind),

    /// Groq-specific error
    #[cfg(feature = "groq")]
    #[from(GroqErrorKind)]
    Groq(GroqErrorKind),

    /// Invalid role for message
    #[display("Invalid role: {}", _0)]
    #[from(ignore)]
    InvalidRole(String),

    /// Token counting failed
    #[display("Token counting failed: {}", _0)]
    #[from(ignore)]
    TokenCountingFailed(String),

    /// Tiktoken decode error
    #[display("Tiktoken decode error: {}", _0)]
    #[from(ignore)]
    TiktokenDecode(String),

    /// Tiktoken decode key error (invalid token)
    #[display("Tiktoken decode key error: token {}", _0)]
    #[from(ignore)]
    TiktokenDecodeKey(u32),

    /// Tiktoken initialization failed
    #[display("Tiktoken initialization failed: {}", _0)]
    #[from(ignore)]
    Tiktoken(std::sync::Arc<anyhow::Error>),

    /// Serialization error (JSON encoding/decoding failures)
    #[display("Serialization error: {}", _0)]
    #[from(ignore)]
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
    file: &'static str,
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

#[cfg(feature = "anthropic")]
crate::impl_error_from_kind!(AnthropicErrorKind => AnthropicError);

#[cfg(feature = "anthropic")]
impl From<AnthropicErrorKind> for ModelsError {
    fn from(kind: AnthropicErrorKind) -> Self {
        ModelsError::new(ModelsErrorKind::Anthropic(kind))
    }
}

crate::impl_error_from_kind!(ModelsErrorKind => ModelsError);

#[cfg(feature = "groq")]
crate::impl_error_from_kind!(GroqErrorKind => GroqError);

#[cfg(feature = "groq")]
impl From<GroqErrorKind> for ModelsError {
    fn from(kind: GroqErrorKind) -> Self {
        ModelsError::new(ModelsErrorKind::Groq(kind))
    }
}

/// Result type for model operations.
pub type ModelsResult<T> = Result<T, ModelsError>;
