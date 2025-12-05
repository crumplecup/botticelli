//! Model provider errors.

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum AnthropicErrorKind {
    /// HTTP error (connection, timeout, etc.)
    #[display("HTTP error: {}", _0)]
    Http(String),

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
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
    InvalidRole(String),

    /// Token counting failed
    #[display("Token counting failed: {}", _0)]
    TokenCountingFailed(String),
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

/// Result type for model operations.
pub type ModelsResult<T> = Result<T, ModelsError>;
