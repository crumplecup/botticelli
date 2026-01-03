//! Tool/function calling capability.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Trait for models that support function/tool calling.
#[async_trait]
pub trait ToolCalling: BotticelliDriver {
    /// Tool definition type.
    type ToolDefinition: Send + Sync;

    /// Tool result type.
    type ToolResult: Send + Sync;

    /// Generate with available tools.
    async fn generate_with_tools(
        &self,
        req: &Self::Request,
        tools: &[Self::ToolDefinition],
    ) -> Result<Self::Response, Self::Error>;

    /// Maximum number of tools per request.
    fn max_tools_per_request(&self) -> usize {
        128
    }
}
