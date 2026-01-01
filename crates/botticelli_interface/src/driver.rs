//! Core driver trait definition.

use async_trait::async_trait;

/// Core trait that all LLM backends must implement.
///
/// This provides the minimal interface for synchronous text generation.
/// Additional capabilities are exposed through optional traits.
#[async_trait]
pub trait BotticelliDriver: Send + Sync {
    /// Request type for generation.
    type Request: Send + Sync;
    
    /// Response type from generation.
    type Response: Send + Sync;
    
    /// Error type for this driver.
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Rate limit configuration type.
    type RateLimitConfig: Send + Sync;
    
    /// Capabilities type for runtime feature discovery.
    type Capabilities: Send + Sync;

    /// Generate model output given a multimodal request.
    async fn generate(&self, req: &Self::Request) -> Result<Self::Response, Self::Error>;

    /// Provider name (e.g., "anthropic", "openai", "gemini").
    fn provider_name(&self) -> &'static str;

    /// Model identifier (e.g., "claude-3-5-sonnet-20241022").
    fn model_name(&self) -> &str;

    /// Rate limits for this driver.
    ///
    /// Returns the rate limit configuration for carousel budget tracking.
    fn rate_limits(&self) -> &Self::RateLimitConfig;

    /// Query provider capabilities.
    ///
    /// Returns capability flags indicating which optional features
    /// this provider supports (streaming, tools, vision, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_interface::BotticelliDriver;
    ///
    /// fn check_capabilities<D: BotticelliDriver>(driver: &D) {
    ///     let caps = driver.capabilities();
    ///     // Check capabilities specific to the driver's type
    /// }
    /// ```
    fn capabilities(&self) -> Self::Capabilities;
}

// Blanket implementation for Arc<T>
#[async_trait]
impl<T: BotticelliDriver + ?Sized> BotticelliDriver for std::sync::Arc<T> {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type RateLimitConfig = T::RateLimitConfig;
    type Capabilities = T::Capabilities;

    async fn generate(&self, req: &Self::Request) -> Result<Self::Response, Self::Error> {
        (**self).generate(req).await
    }

    fn provider_name(&self) -> &'static str {
        (**self).provider_name()
    }

    fn model_name(&self) -> &str {
        (**self).model_name()
    }

    fn rate_limits(&self) -> &Self::RateLimitConfig {
        (**self).rate_limits()
    }
    
    fn capabilities(&self) -> Self::Capabilities {
        (**self).capabilities()
    }
}
