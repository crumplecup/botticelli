//! Streaming response capability.

use crate::BotticelliDriver;
use async_trait::async_trait;
use futures_util::stream::Stream;
use std::pin::Pin;

/// Trait for models that support streaming responses.
#[async_trait]
pub trait Streaming: BotticelliDriver {
    /// Stream chunk type.
    type StreamChunk: Send + Sync;
    
    /// Generate a streaming response.
    ///
    /// Returns a stream that yields chunks as they arrive from the API.
    async fn generate_stream(
        &self,
        req: &Self::Request,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Self::StreamChunk, Self::Error>> + Send>>, Self::Error>;
}
