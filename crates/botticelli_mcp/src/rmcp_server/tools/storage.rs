//! Storage trait wrapper tools.
//!
//! These wrapper functions provide MCP tool access to MediaStorage trait methods.
//! The trait implementations delegate to these tools for consistent observability.

use botticelli_error::BotticelliError;
use botticelli_interface::MediaStorage;
use botticelli_storage::{FileSystemStorage, MediaMetadata, MediaReference, MediaType};
use elicitation::Elicit;
use rmcp::tool;
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
