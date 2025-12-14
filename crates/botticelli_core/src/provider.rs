//! LLM provider abstraction.
//!
//! This module defines the [`LlmProvider`] trait that all provider clients
//! (Anthropic, OpenAI, Gemini, etc.) implement for unified access.

use crate::{GenerateRequest, GenerateResponse};
use async_trait::async_trait;

/// Unified interface for LLM providers.
///
/// All provider clients (Anthropic, OpenAI, Gemini, etc.) implement this trait,
/// allowing them to be used interchangeably.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a response from the provider.
    async fn generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, ProviderError>;

    /// Get the provider name for logging/debugging.
    fn provider_name(&self) -> &str;

    /// Get the default model name.
    fn default_model(&self) -> &str;

    /// Check if this provider supports tool calling.
    fn supports_tools(&self) -> bool {
        true // Most modern providers do
    }
}

/// Errors from provider operations.
#[derive(Debug, Clone, derive_more::Display, derive_more::Error)]
#[display("Provider {}: {} at {}:{}", provider, kind, file, line)]
pub struct ProviderError {
    /// Provider name
    pub provider: String,
    /// Error kind
    pub kind: ProviderErrorKind,
    /// Line number where error occurred
    pub line: u32,
    /// File where error occurred
    pub file: &'static str,
}

/// Types of provider errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ProviderErrorKind {
    /// API error from the provider.
    #[display("API error: {}", _0)]
    ApiError(String),

    /// Authentication failed.
    #[display("Authentication failed")]
    AuthenticationFailed,

    /// Rate limit exceeded.
    #[display("Rate limit exceeded")]
    RateLimitExceeded,

    /// Invalid request.
    #[display("Invalid request: {}", _0)]
    InvalidRequest(String),

    /// Response parsing failed.
    #[display("Response parsing failed: {}", _0)]
    ParsingError(String),
}

impl ProviderError {
    /// Create a new provider error with location tracking.
    #[track_caller]
    pub fn new(provider: impl Into<String>, kind: ProviderErrorKind) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            provider: provider.into(),
            kind,
            line: loc.line(),
            file: loc.file(),
        }
    }
}
