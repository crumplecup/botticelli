//! Tool/function calling capability.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Trait for models that support function/tool calling.
#[async_trait]
pub trait ToolCalling: BotticelliDriver {
    /// Error type for tool calling operations.
    type Error: std::error::Error + Send + Sync;

    /// Tool definition type.
    type ToolDefinition: Send + Sync;

    /// Generate with available tools.
    async fn generate_with_tools(
        &self,
        req: &Self::Request,
        tools: &[Self::ToolDefinition],
    ) -> Result<Self::Response, <Self as ToolCalling>::Error>;

    /// Maximum number of tools per request.
    fn max_tools_per_request(&self) -> usize {
        128
    }
}
