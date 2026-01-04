//! Streaming trait implementation.

use crate::GeminiClient;
use crate::gemini::GeminiResult;
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, StreamChunk};
use botticelli_interface::Streaming;

#[async_trait]
impl Streaming for GeminiClient {
    type StreamChunk = StreamChunk;

    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> GeminiResult<
        std::pin::Pin<
            Box<dyn futures_util::stream::Stream<Item = GeminiResult<Self::StreamChunk>> + Send>,
        >,
    > {
        // Delegate to internal implementation
        self.generate_stream_internal(req).await
    }
}
