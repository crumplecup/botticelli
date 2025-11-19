//! Media reference types.

use crate::MediaType;
use uuid::Uuid;

/// Reference to stored media.
///
/// This structure contains all the information needed to retrieve media
/// from a storage backend, plus metadata for database storage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaReference {
    /// Unique identifier for this media reference
    pub id: Uuid,
    /// SHA-256 hash of the content (for deduplication)
    pub content_hash: String,
    /// Storage backend name (e.g., "filesystem", "s3", "postgres")
    pub storage_backend: String,
    /// Backend-specific path/key to the media
    pub storage_path: String,
    /// Size of the media in bytes
    pub size_bytes: i64,
    /// Type of media
    pub media_type: MediaType,
    /// MIME type
    pub mime_type: String,
}
