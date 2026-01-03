//! JSON mode capability.

use crate::BotticelliDriver;
use async_trait::async_trait;

/// Trait for models that support structured JSON output mode.
#[async_trait]
pub trait JsonMode: BotticelliDriver {
    /// Schema type for JSON output.
    type JsonSchema: Send + Sync;

    /// Generate JSON output matching the provided schema.
    async fn generate_json(
        &self,
        req: &Self::Request,
        schema: &Self::JsonSchema,
    ) -> Result<Self::Response, Self::Error>;
}
