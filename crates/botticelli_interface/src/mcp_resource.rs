//! MCP resource trait.
//!
//! Resources are data sources that LLMs can read. They follow URI patterns like:
//! - `content://approved_discord_posts/123` - Content by ID
//! - `narrative://curate_content` - Narrative TOML file

use async_trait::async_trait;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for reading a resource.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadParams {
    /// URI to read
    pub uri: String,
}

/// Result of reading a resource.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResult {
    /// Resource content
    pub content: String,
}

/// MCP resource that LLMs can read.
///
/// # Tool-Native Design
///
/// This trait uses MCP-compatible signatures to enable automatic tool generation
/// via `#[elicit_trait_tools_router]`. Uses `#[async_trait]` for object safety,
/// allowing `Box<dyn McpResource>` in registries while maintaining MCP tool compatibility.
///
/// # Associated Types
///
/// Implementations specify concrete types for:
/// - Error type for operations
/// - ResourceInfo type (metadata about resources)
///
/// # Examples
///
/// ```ignore
/// use async_trait::async_trait;
/// use botticelli_interface::{McpResource, ReadParams, ReadResult};
/// use rmcp::{Parameters, Json};
///
/// pub struct MyResource;
///
/// #[async_trait]
/// impl McpResource for MyResource {
///     type Error = MyError;
///     type ResourceInfo = MyResourceInfo;
///
///     fn uri_pattern(&self) -> &'static str {
///         "myresource://"
///     }
///
///     fn description(&self) -> &'static str {
///         "My custom resource"
///     }
///
///     async fn read(
///         &self,
///         params: Parameters<ReadParams>,
///     ) -> Result<Json<ReadResult>, rmcp::ErrorData> {
///         Ok(Json(ReadResult {
///             content: "data".to_string(),
///         }))
///     }
/// }
/// ```
#[async_trait]
pub trait McpResource: Send + Sync {
    /// Error type for operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Resource information type
    type ResourceInfo;

    /// URI pattern this resource handles (e.g., "content://", "narrative://")
    fn uri_pattern(&self) -> &'static str;

    /// Resource description for LLM
    fn description(&self) -> &'static str;

    /// Check if this resource handles the given URI
    fn matches(&self, uri: &str) -> bool {
        uri.starts_with(self.uri_pattern())
    }

    /// Read resource content (tool-native signature).
    ///
    /// Uses MCP-compatible parameters and return type for automatic tool generation.
    async fn read(
        &self,
        params: Parameters<ReadParams>,
    ) -> Result<Json<ReadResult>, rmcp::ErrorData>;

    /// List available resources (non-tool method for registry).
    ///
    /// Returns list of available resource instances. Not exposed as MCP tool.
    /// Default implementation returns error.
    async fn list(&self) -> Result<Vec<Self::ResourceInfo>, Self::Error> {
        unimplemented!("list() not implemented for this resource")
    }
}
