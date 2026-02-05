//! Storage trait wrapper tools and primitive delegation.
//!
//! This module provides:
//! 1. MCP tool access to MediaStorage trait methods
//! 2. Orchestrator delegation wrappers for storage primitives

use crate::rmcp_server::BotticelliServer;
use botticelli_error::BotticelliError;
use botticelli_interface::MediaStorage;
use botticelli_storage::{FileSystemStorage, MediaMetadata, MediaReference, MediaType};
use elicitation::Elicit;
use rmcp::tool;
use rmcp::tool_router;
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

/// Store media content.
#[tool]
#[instrument(skip(params), fields(data_len = params.data.len()))]
pub async fn media_storage_store(
    params: MediaStorageStoreParams,
) -> Result<MediaStorageStoreResult, BotticelliError> {
    let metadata = MediaMetadata::new(params.media_type);
    let storage = FileSystemStorage::new(PathBuf::from("./media"))?;
    let reference = storage.store(&params.data, &metadata).await?;
    
    tracing::info!(reference = ?reference, "Media stored");
    Ok(MediaStorageStoreResult { reference })
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

/// Retrieve media content.
#[tool]
#[instrument]
pub async fn media_storage_retrieve(
    params: MediaStorageRetrieveParams,
) -> Result<MediaStorageRetrieveResult, BotticelliError> {
    let storage = FileSystemStorage::new(PathBuf::from("./media"))?;
    let data = storage.retrieve(&params.reference).await?;
    
    tracing::info!(data_len = data.len(), "Media retrieved");
    Ok(MediaStorageRetrieveResult { data })
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

/// Get public URL for media.
#[tool]
#[instrument]
pub async fn media_storage_get_url(
    params: MediaStorageGetUrlParams,
) -> Result<MediaStorageGetUrlResult, BotticelliError> {
    let storage = FileSystemStorage::new(PathBuf::from("./media"))?;
    let expires_in = Duration::from_secs(params.expires_in_secs);
    let url = storage.get_url(&params.reference, expires_in).await?;
    
    tracing::info!(has_url = url.is_some(), "URL obtained");
    Ok(MediaStorageGetUrlResult { url })
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

/// Delete media content.
#[tool]
#[instrument]
pub async fn media_storage_delete(
    params: MediaStorageDeleteParams,
) -> Result<MediaStorageDeleteResult, BotticelliError> {
    let storage = FileSystemStorage::new(PathBuf::from("./media"))?;
    storage.delete(&params.reference).await?;
    
    tracing::info!("Media deleted");
    Ok(MediaStorageDeleteResult { success: true })
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

/// Check if media exists.
#[tool]
#[instrument]
pub async fn media_storage_exists(
    params: MediaStorageExistsParams,
) -> Result<MediaStorageExistsResult, BotticelliError> {
    let storage = FileSystemStorage::new(PathBuf::from("./media"))?;
    let exists = storage.exists(&params.reference).await?;
    
    tracing::info!(exists = exists, "Existence checked");
    Ok(MediaStorageExistsResult { exists })
}

// ============================================================================
// Storage Primitive Delegation Wrappers
// ============================================================================

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

impl BotticelliServer {
    /// Create a new filesystem storage backend.
    ///
    /// Creates the base directory if it doesn't exist.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "storage_new", base_path = %params.base_path.display()))]
    pub fn storage_new(&self, params: StorageNewParams) -> Result<FileSystemStorage, BotticelliError> {
        tracing::debug!("Delegating to botticelli_storage::FileSystemStorage::new");
        
        let result = FileSystemStorage::new(params.base_path);
        
        match &result {
            Ok(_) => tracing::debug!("Storage creation succeeded"),
            Err(e) => tracing::error!(error = ?e, "Storage creation failed"),
        }
        
        result
    }

    /// Compute SHA-256 hash of data.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "storage_compute_hash", data_len = params.data.len()))]
    pub fn storage_compute_hash(&self, params: StorageComputeHashParams) -> StorageComputeHashResult {
        tracing::debug!("Delegating to botticelli_storage::FileSystemStorage::compute_hash");
        
        let hash = FileSystemStorage::compute_hash(&params.data);
        
        tracing::debug!(hash = %hash, "Hash computed");
        StorageComputeHashResult { hash }
    }

    /// Get the filesystem path for a given hash and media type.
    ///
    /// Structure: `{base}/{type}/{hash[0:2]}/{hash[2:4]}/{hash}`
    #[tool]
    #[instrument(skip(self, params), fields(tool = "storage_get_path", hash = %params.hash))]
    pub fn storage_get_path(&self, params: StorageGetPathParams) -> Result<StorageGetPathResult, BotticelliError> {
        tracing::debug!("Delegating to botticelli_storage::FileSystemStorage::get_path");
        
        let storage = FileSystemStorage::new(params.base_path)?;
        let path = storage.get_path(&params.hash, params.media_type);
        
        tracing::debug!(path = %path.display(), "Path computed");
        Ok(StorageGetPathResult { path })
    }

    /// Verify content hash matches expected hash.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "storage_verify_hash", expected = %params.expected_hash))]
    pub fn storage_verify_hash(&self, params: StorageVerifyHashParams) -> Result<(), BotticelliError> {
        tracing::debug!("Delegating to botticelli_storage::FileSystemStorage::verify_hash");
        
        let result = FileSystemStorage::verify_hash(&params.data, &params.expected_hash);
        
        match &result {
            Ok(_) => tracing::debug!("Hash verification succeeded"),
            Err(e) => tracing::error!(error = ?e, "Hash verification failed"),
        }
        
        result
    }

    /// Convert MediaType to string representation.
    #[tool]
    #[instrument(skip(self, params), fields(tool = "media_type_as_str"))]
    pub fn media_type_as_str(&self, params: MediaTypeAsStrParams) -> MediaTypeAsStrResult {
        tracing::debug!("Delegating to botticelli_storage::MediaType::as_str");
        
        let value = params.media_type.as_str().to_string();
        
        tracing::debug!(value = %value, "MediaType converted");
        MediaTypeAsStrResult { value }
    }
}
