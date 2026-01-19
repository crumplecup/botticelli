//! MCP resource trait.
//!
//! Resources are data sources that LLMs can read. They follow URI patterns like:
//! - `content://approved_discord_posts/123` - Content by ID
//! - `narrative://curate_content` - Narrative TOML file

use async_trait::async_trait;

/// MCP resource that LLMs can read.
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
/// use botticelli_interface::McpResource;
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
///     async fn read(&self, uri: &str) -> Result<String, Self::Error> {
///         // Implementation
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

    /// Read resource content
    async fn read(&self, uri: &str) -> Result<String, Self::Error>;

    /// List available resources (optional)
    async fn list(&self) -> Result<Vec<Self::ResourceInfo>, Self::Error> {
        // Default implementation returns empty vec
        // Must be overridden by implementer to return actual list
        unimplemented!("list() not implemented for this resource")
    }
}
