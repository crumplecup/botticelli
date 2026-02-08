//! Media storage trait definition.

use crate::media_storage_types::{
    DeleteParams, DeleteResult, ExistsParams, ExistsResult, GetUrlParams, GetUrlResult,
    RetrieveParams, RetrieveResult, StoreParams, StoreResult,
};
use async_trait::async_trait;
use rmcp::handler::server::wrapper::Parameters;

/// Trait for pluggable media storage backends.
///
/// Implementations handle the actual storage and retrieval of binary media data,
/// while metadata is managed separately in the database.
///
/// # Tool-Native Design
///
/// This trait uses MCP-compatible signatures with `#[async_trait]` for automatic
/// tool generation via `#[elicit_trait_tools_router]`. Methods use `Parameters<T>`
/// and return `Result<rmcp::Json<R>, rmcp::ErrorData>` for MCP integration.
#[async_trait]
pub trait MediaStorage: Send + Sync {
    /// Error type for storage operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Media metadata type for this storage backend.
    type Metadata: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + schemars::JsonSchema;

    /// Media reference type returned by storage operations.
    type Reference: Send + Sync + Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + schemars::JsonSchema;

    /// Store media and return a reference (tool-native signature).
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing data and metadata
    ///
    /// # Returns
    ///
    /// A reference containing the storage location and metadata
    async fn store(
        &self,
        params: Parameters<StoreParams<Self::Metadata, Vec<u8>>>,
    ) -> Result<rmcp::Json<StoreResult<Self::Reference>>, rmcp::ErrorData>;

    /// Retrieve media by reference (tool-native signature).
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the reference
    ///
    /// # Returns
    ///
    /// The raw binary media data
    async fn retrieve(
        &self,
        params: Parameters<RetrieveParams<Self::Reference>>,
    ) -> Result<rmcp::Json<RetrieveResult>, rmcp::ErrorData>;

    /// Get a temporary URL for direct access (tool-native signature).
    ///
    /// Some storage backends (like S3) can generate presigned URLs that allow
    /// direct access to the media without going through the application.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing reference and expiry duration
    ///
    /// # Returns
    ///
    /// URL if the backend supports direct URLs, None otherwise
    async fn get_url(
        &self,
        params: Parameters<GetUrlParams<Self::Reference>>,
    ) -> Result<rmcp::Json<GetUrlResult>, rmcp::ErrorData>;

    /// Delete media by reference (tool-native signature).
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the reference
    async fn delete(
        &self,
        params: Parameters<DeleteParams<Self::Reference>>,
    ) -> Result<rmcp::Json<DeleteResult>, rmcp::ErrorData>;

    /// Check if media exists (tool-native signature).
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters containing the reference
    ///
    /// # Returns
    ///
    /// Whether the media exists
    async fn exists(
        &self,
        params: Parameters<ExistsParams<Self::Reference>>,
    ) -> Result<rmcp::Json<ExistsResult>, rmcp::ErrorData>;
}
