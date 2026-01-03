//! BotticelliDriver trait implementation for GeminiClient.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::BotticelliResult;
use botticelli_interface::BotticelliDriver;

use super::core::GeminiClient;

#[async_trait]
impl BotticelliDriver for GeminiClient {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        self.generate_internal(req).await.map_err(Into::into)
    }
}
