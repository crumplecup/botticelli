//! Simplified execution driver trait.
//!
//! This trait provides a type-erased interface for LLM drivers that all use
//! the same Request/Response types (GenerateRequest/GenerateResponse).
//!
//! The main `BotticelliDriver` trait has associated types (Error, RateLimitConfig,
//! Capabilities) that differ between implementations, making trait objects impossible.
//! This trait erases those types to enable uniform handling of drivers in execution
//! contexts.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Simplified driver trait for execution contexts.
///
/// This trait wraps `BotticelliDriver` and erases the problematic associated types
/// (Error, RateLimitConfig, Capabilities) that prevent trait object usage.
///
/// All drivers that implement `BotticelliDriver<Request = GenerateRequest, Response = GenerateResponse>`
/// automatically implement this trait via the blanket implementation.
///
/// # Type Parameters
///
/// Generic types are used as aliases to avoid circular dependencies with botticelli_core:
/// - `Req`: Request type (typically botticelli_core::GenerateRequest)
/// - `Resp`: Response type (typically botticelli_core::GenerateResponse)
#[async_trait]
pub trait ExecutionDriver<Req = (), Resp = ()>: Send + Sync
where
    Req: Send + Sync,
    Resp: Send + Sync,
{
    /// Generate model output given a request.
    ///
    /// Errors are boxed to avoid associated type issues with trait objects.
    async fn generate(
        &self,
        req: &Req,
    ) -> Result<Resp, Box<dyn std::error::Error + Send + Sync>>;

    /// Provider name (e.g., "anthropic", "gemini").
    fn provider_name(&self) -> &str;

    /// Model identifier.
    fn model_name(&self) -> &str;

    /// Get rate limit configuration as a Tier trait object.
    ///
    /// Returns a boxed clone of the rate limit configuration that implements the Tier trait.
    /// This allows uniform handling of different concrete rate limit types.
    fn rate_limits(&self) -> Box<dyn crate::Tier>;
}

/// Blanket implementation for any BotticelliDriver with the correct Request/Response types.
///
/// This automatically makes all LLM drivers usable as `Arc<dyn ExecutionDriver<Req, Resp>>`.
#[async_trait]
impl<D, Req, Resp> ExecutionDriver<Req, Resp> for D
where
    D: BotticelliDriver<Request = Req, Response = Resp> + Send + Sync,
    D::RateLimitConfig: 'static,
    Req: Send + Sync,
    Resp: Send + Sync,
{
    async fn generate(
        &self,
        req: &Req,
    ) -> Result<Resp, Box<dyn std::error::Error + Send + Sync>> {
        BotticelliDriver::generate(self, req)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn provider_name(&self) -> &str {
        BotticelliDriver::provider_name(self)
    }

    fn model_name(&self) -> &str {
        BotticelliDriver::model_name(self)
    }

    fn rate_limits(&self) -> Box<dyn crate::Tier> {
        Box::new(BotticelliDriver::rate_limits(self).clone())
    }
}
