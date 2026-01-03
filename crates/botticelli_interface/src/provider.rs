//! LLM provider abstraction.
//!
//! This module defines the [`LlmProvider`] trait that all provider clients
//! (Anthropic, OpenAI, Gemini, etc.) implement for unified access.

use async_trait::async_trait;

/// Unified interface for LLM providers.
///
/// All provider clients (Anthropic, OpenAI, Gemini, etc.) implement this trait,
/// allowing them to be used interchangeably.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Request type for generation.
    type Request: Send + Sync;

    /// Response type from generation.
    type Response: Send + Sync;

    /// Error type for this provider.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generate a response from the provider.
    async fn generate(&self, request: &Self::Request) -> Result<Self::Response, Self::Error>;

    /// Get the provider name for logging/debugging.
    fn provider_name(&self) -> &str;

    /// Get the default model name.
    fn default_model(&self) -> &str;

    /// Check if this provider supports tool calling.
    fn supports_tools(&self) -> bool {
        true // Most modern providers do
    }
}
