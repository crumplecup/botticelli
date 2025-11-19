//! Storage trait definition.

use crate::{MediaMetadata, MediaReference};
use botticelli_error::BotticelliResult;
use std::time::Duration;

/// Trait for pluggable media storage backends.
///
/// Implementations handle the actual storage and retrieval of binary media data,
/// while metadata is managed separately in the database.
#[async_trait::async_trait]
pub trait MediaStorage: Send + Sync {
    /// Store media and return a reference.
    ///
    /// The implementation should:
    /// - Compute content hash for deduplication
    /// - Store the binary data in its backend
    /// - Return a reference that can be used to retrieve the data
    ///
    /// # Arguments
    ///
    /// * `data` - The binary media data to store
    /// * `metadata` - Metadata about the media (type, mime type, etc.)
    ///
    /// # Returns
    ///
    /// A `MediaReference` containing the storage location and metadata
    async fn store(
        &self,
        data: &[u8],
        metadata: &MediaMetadata,
    ) -> BotticelliResult<MediaReference>;

    /// Retrieve media by reference.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference returned by `store()`
    ///
    /// # Returns
    ///
    /// The raw binary media data
    async fn retrieve(&self, reference: &MediaReference) -> BotticelliResult<Vec<u8>>;

    /// Get a temporary URL for direct access (if supported).
    ///
    /// Some storage backends (like S3) can generate presigned URLs that allow
    /// direct access to the media without going through the application.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference
    /// * `expires_in` - How long the URL should remain valid
    ///
    /// # Returns
    ///
    /// `Some(url)` if the backend supports direct URLs, `None` otherwise
    async fn get_url(
        &self,
        reference: &MediaReference,
        expires_in: Duration,
    ) -> BotticelliResult<Option<String>>;

    /// Delete media by reference.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference to delete
    async fn delete(&self, reference: &MediaReference) -> BotticelliResult<()>;

    /// Check if media exists.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference to check
    ///
    /// # Returns
    ///
    /// `true` if the media exists, `false` otherwise
    async fn exists(&self, reference: &MediaReference) -> BotticelliResult<bool>;
}
