//! Media reference types.

use crate::MediaType;
use uuid::Uuid;

/// Reference to stored media.
///
/// This structure contains all the information needed to retrieve media
/// from a storage backend, plus metadata for database storage.
///
/// # Example
///
/// ```rust
/// use botticelli_storage::{MediaReferenceBuilder, MediaType};
/// use uuid::Uuid;
///
/// let reference = MediaReferenceBuilder::default()
///     .id(Uuid::new_v4())
///     .content_hash("abc123...".to_string())
///     .storage_backend("filesystem".to_string())
///     .storage_path("/path/to/file".to_string())
///     .size_bytes(1024)
///     .media_type(MediaType::Image)
///     .mime_type("image/png".to_string())
///     .build()
///     .expect("Valid reference");
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_getters::Getters,
    derive_builder::Builder,
    elicitation::Elicit,
)]
#[builder(setter(into))]
pub struct MediaReference {
    /// Unique identifier for this media reference
    id: Uuid,
    /// SHA-256 hash of the content (for deduplication)
    content_hash: String,
    /// Storage backend name (e.g., "filesystem", "s3", "postgres")
    storage_backend: String,
    /// Backend-specific path/key to the media
    storage_path: String,
    /// Size of the media in bytes
    size_bytes: i64,
    /// Type of media
    media_type: MediaType,
    /// MIME type
    mime_type: String,
}
