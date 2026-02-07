//! Storage trait wrapper tools and primitive delegation.
//!
//! This module provides:
//! 1. MCP tool access to MediaStorage trait methods
//! 2. Orchestrator delegation wrappers for storage primitives

use crate::rmcp_server::BotticelliServer;
use botticelli_interface::MediaStorage;
use botticelli_storage::{FileSystemStorage, MediaMetadata, MediaReference, MediaType};
use elicitation::Elicit;
use rmcp::{tool, tool_router};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ErrorCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tracing::instrument;

/// Parameters for storing media.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageStoreParams {
    /// Binary data to store
    pub data: Vec<u8>,
    /// Media type
    pub media_type: MediaType,
}

/// Result from storing media.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageStoreResult {
    /// Storage reference
    pub reference: MediaReference,
}

/// Parameters for retrieving media.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageRetrieveParams {
    /// Storage reference
    pub reference: MediaReference,
}

/// Result from retrieving media.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageRetrieveResult {
    /// Binary data
    pub data: Vec<u8>,
}

/// Parameters for getting media URL.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageGetUrlParams {
    /// Storage reference
    pub reference: MediaReference,
    /// Expiry duration in seconds
    pub expires_in_secs: u64,
}

/// Result from getting media URL.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageGetUrlResult {
    /// Public URL (if supported)
    pub url: Option<String>,
}

/// Parameters for deleting media.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageDeleteParams {
    /// Storage reference
    pub reference: MediaReference,
}

/// Result from deleting media.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageDeleteResult {
    /// Success status
    pub success: bool,
}

/// Parameters for checking media existence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageExistsParams {
    /// Storage reference
    pub reference: MediaReference,
}

/// Result from checking media existence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaStorageExistsResult {
    /// Whether media exists
    pub exists: bool,
}

/// Parameters for creating filesystem storage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageNewParams {
    /// Base path for storage
    pub base_path: PathBuf,
}

/// Parameters for computing hash.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageComputeHashParams {
    /// Data to hash
    pub data: Vec<u8>,
}

/// Result from computing hash.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageComputeHashResult {
    /// SHA-256 hash
    pub hash: String,
}

/// Parameters for getting storage path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageGetPathParams {
    /// Base path for storage
    pub base_path: PathBuf,
    /// Content hash
    pub hash: String,
    /// Media type
    pub media_type: MediaType,
}

/// Result from getting storage path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageGetPathResult {
    /// Full filesystem path
    pub path: PathBuf,
}

/// Parameters for verifying hash.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageVerifyHashParams {
    /// Data to verify
    pub data: Vec<u8>,
    /// Expected hash
    pub expected_hash: String,
}

/// Result from verifying hash.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct StorageVerifyHashResult {
    /// Verification success
    pub success: bool,
}

/// Parameters for media type conversion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaTypeAsStrParams {
    /// Media type to convert
    pub media_type: MediaType,
}

/// Result from media type conversion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct MediaTypeAsStrResult {
    /// String representation
    pub value: String,
}

/// Storage tool implementations for the MCP server.
///
/// Generated tool router function: `storage_tool_router()`
///
/// Note: The #[tool_router] macro generates the `storage_tool_router()` function
/// without documentation attributes. Since we cannot modify the macro-generated
/// code, we use #[allow(missing_docs)] as an explicit exception to the crate's
/// missing_docs lint. This is acceptable for macro-generated code where
/// documentation would need to be added by the macro itself (upstream fix).
#[allow(missing_docs)]
#[tool_router(router = storage_tool_router, vis = "pub")]
impl BotticelliServer {
    /// Store media content.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "media_storage_store"))]
    pub async fn media_storage_store(
        &self,
        Parameters(_params): Parameters<MediaStorageStoreParams>,
    ) -> Result<Json<MediaStorageStoreResult>, rmcp::ErrorData> {
        let metadata = MediaMetadata::new(_params.media_type);
        let storage = FileSystemStorage::new(PathBuf::from("./media"))
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        let reference = storage.store(&_params.data, &metadata).await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        
        tracing::info!(reference = ?reference, data_len = _params.data.len(), "Media stored");
        Ok(Json(MediaStorageStoreResult { reference }))
    }

    /// Retrieve media content.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "media_storage_retrieve"))]
    pub async fn media_storage_retrieve(
        &self,
        Parameters(_params): Parameters<MediaStorageRetrieveParams>,
    ) -> Result<Json<MediaStorageRetrieveResult>, rmcp::ErrorData> {
        let storage = FileSystemStorage::new(PathBuf::from("./media"))
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        let data = storage.retrieve(&_params.reference).await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        
        tracing::info!(data_len = data.len(), "Media retrieved");
        Ok(Json(MediaStorageRetrieveResult { data }))
    }

    /// Get public URL for media.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "media_storage_get_url"))]
    pub async fn media_storage_get_url(
        &self,
        Parameters(_params): Parameters<MediaStorageGetUrlParams>,
    ) -> Result<Json<MediaStorageGetUrlResult>, rmcp::ErrorData> {
        let storage = FileSystemStorage::new(PathBuf::from("./media"))
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        let expires_in = Duration::from_secs(_params.expires_in_secs);
        let url = storage.get_url(&_params.reference, expires_in).await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        
        tracing::info!(has_url = url.is_some(), "URL obtained");
        Ok(Json(MediaStorageGetUrlResult { url }))
    }

    /// Delete media content.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "media_storage_delete"))]
    pub async fn media_storage_delete(
        &self,
        Parameters(_params): Parameters<MediaStorageDeleteParams>,
    ) -> Result<Json<MediaStorageDeleteResult>, rmcp::ErrorData> {
        let storage = FileSystemStorage::new(PathBuf::from("./media"))
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        storage.delete(&_params.reference).await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        
        tracing::info!("Media deleted");
        Ok(Json(MediaStorageDeleteResult { success: true }))
    }

    /// Check if media exists.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "media_storage_exists"))]
    pub async fn media_storage_exists(
        &self,
        Parameters(_params): Parameters<MediaStorageExistsParams>,
    ) -> Result<Json<MediaStorageExistsResult>, rmcp::ErrorData> {
        let storage = FileSystemStorage::new(PathBuf::from("./media"))
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        let exists = storage.exists(&_params.reference).await
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        
        tracing::info!(exists = exists, "Existence checked");
        Ok(Json(MediaStorageExistsResult { exists }))
    }

    /// Compute SHA-256 hash of data.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "storage_compute_hash"))]
    pub fn storage_compute_hash(
        &self,
        Parameters(_params): Parameters<StorageComputeHashParams>,
    ) -> Result<Json<StorageComputeHashResult>, rmcp::ErrorData> {
        tracing::debug!(data_len = _params.data.len(), "Delegating to botticelli_storage::FileSystemStorage::compute_hash");
        
        let hash = FileSystemStorage::compute_hash(&_params.data);
        
        tracing::debug!(hash = %hash, "Hash computed");
        Ok(Json(StorageComputeHashResult { hash }))
    }

    /// Get the filesystem path for a given hash and media type.
    ///
    /// Structure: `{base}/{type}/{hash[0:2]}/{hash[2:4]}/{hash}`
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "storage_get_path"))]
    pub fn storage_get_path(
        &self,
        Parameters(_params): Parameters<StorageGetPathParams>,
    ) -> Result<Json<StorageGetPathResult>, rmcp::ErrorData> {
        tracing::debug!(hash = %_params.hash, "Delegating to botticelli_storage::FileSystemStorage::get_path");
        
        let storage = FileSystemStorage::new(_params.base_path)
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        let path = storage.get_path(&_params.hash, _params.media_type);
        
        tracing::debug!(path = %path.display(), "Path computed");
        Ok(Json(StorageGetPathResult { path }))
    }

    /// Verify content hash matches expected hash.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "storage_verify_hash"))]
    pub fn storage_verify_hash(
        &self,
        Parameters(_params): Parameters<StorageVerifyHashParams>,
    ) -> Result<Json<StorageVerifyHashResult>, rmcp::ErrorData> {
        tracing::debug!(expected = %_params.expected_hash, "Delegating to botticelli_storage::FileSystemStorage::verify_hash");
        
        FileSystemStorage::verify_hash(&_params.data, &_params.expected_hash)
            .map_err(|e| rmcp::ErrorData::new(ErrorCode(-1), e.to_string(), None))?;
        
        tracing::debug!("Hash verification succeeded");
        Ok(Json(StorageVerifyHashResult { success: true }))
    }

    /// Convert MediaType to string representation.
    #[tool]
    #[instrument(skip(self, _params), fields(tool = "media_type_as_str"))]
    pub fn media_type_as_str(
        &self,
        Parameters(_params): Parameters<MediaTypeAsStrParams>,
    ) -> Result<Json<MediaTypeAsStrResult>, rmcp::ErrorData> {
        tracing::debug!("Delegating to botticelli_storage::MediaType::as_str");
        
        let value = _params.media_type.as_str().to_string();
        
        tracing::debug!(value = %value, "MediaType converted");
        Ok(Json(MediaTypeAsStrResult { value }))
    }
}
