//! Media storage trait definition.

use async_trait::async_trait;
use std::time::Duration;

/// Trait for pluggable media storage backends.
///
/// Implementations handle the actual storage and retrieval of binary media data,
/// while metadata is managed separately in the database.
#[async_trait]
pub trait MediaStorage: Send + Sync {
    /// Error type for storage operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Media metadata type for this storage backend.
    type Metadata: Send + Sync;

    /// Media reference type returned by storage operations.
    type Reference: Send + Sync;

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
    /// A reference containing the storage location and metadata
    async fn store(
        &self,
        data: &[u8],
        metadata: &Self::Metadata,
    ) -> Result<Self::Reference, Self::Error>;

    /// Retrieve media by reference.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference returned by `store()`
    ///
    /// # Returns
    ///
    /// The raw binary media data
    async fn retrieve(&self, reference: &Self::Reference) -> Result<Vec<u8>, Self::Error>;

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
        reference: &Self::Reference,
        expires_in: Duration,
    ) -> Result<Option<String>, Self::Error>;

    /// Delete media by reference.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference to delete
    async fn delete(&self, reference: &Self::Reference) -> Result<(), Self::Error>;

    /// Check if media exists.
    ///
    /// # Arguments
    ///
    /// * `reference` - The media reference to check
    ///
    /// # Returns
    ///
    /// `true` if the media exists, `false` otherwise
    async fn exists(&self, reference: &Self::Reference) -> Result<bool, Self::Error>;
}
