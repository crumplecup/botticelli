//! Content-addressable media storage for Botticelli.
//!
//! This crate provides pluggable storage backends for media files (images, audio, video).
//! The abstraction separates metadata (stored in PostgreSQL) from content (stored in
//! filesystem, S3, or other backends).
//!
//! # Features
//!
//! - **Content-addressable storage**: Files stored by SHA-256 hash for automatic deduplication
//! - **Pluggable backends**: Trait-based abstraction supports filesystem, S3, etc.
//! - **Atomic operations**: Safe concurrent access with atomic writes
//!
//! # Example
//!
//! ```rust
//! use botticelli_storage::{FileSystemStorage, MediaStorage, MediaMetadata, MediaType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = FileSystemStorage::new("/tmp/media")?;
//! let metadata = MediaMetadata {
//!     media_type: MediaType::Image,
//!     mime_type: "image/png".to_string(),
//!     filename: Some("test.png".to_string()),
//!     width: Some(800),
//!     height: Some(600),
//!     duration_seconds: None,
//! };
//!
//! // Store media
//! let data = vec![0u8; 1024]; // PNG data
//! let reference = storage.store(&data, &metadata).await?;
//!
//! // Retrieve media
//! let retrieved = storage.retrieve(&reference).await?;
//! assert_eq!(data, retrieved);
//! # Ok(())
//! # }
//! ```

use botticelli_error::BotticelliResult;
use std::time::Duration;
use uuid::Uuid;

mod filesystem;

pub use botticelli_error::{StorageError, StorageErrorKind};
pub use filesystem::FileSystemStorage;

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
    async fn store(&self, data: &[u8], metadata: &MediaMetadata)
    -> BotticelliResult<MediaReference>;

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

/// Metadata about media being stored.
#[derive(Debug, Clone)]
pub struct MediaMetadata {
    /// Type of media (image, audio, video)
    pub media_type: MediaType,
    /// MIME type (e.g., "image/png", "video/mp4")
    pub mime_type: String,
    /// Original filename (if available)
    pub filename: Option<String>,
    /// Image/video width in pixels
    pub width: Option<u32>,
    /// Image/video height in pixels
    pub height: Option<u32>,
    /// Audio/video duration in seconds
    pub duration_seconds: Option<f32>,
}

/// Reference to stored media.
///
/// This structure contains all the information needed to retrieve media
/// from a storage backend, plus metadata for database storage.
#[derive(Debug, Clone, PartialEq)]
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

/// Type of media content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaType {
    /// Image content (PNG, JPEG, WebP, etc.)
    Image,
    /// Audio content (MP3, WAV, OGG, etc.)
    Audio,
    /// Video content (MP4, WebM, AVI, etc.)
    Video,
}

impl MediaType {
    /// Convert to string representation for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
        }
    }
}

impl std::str::FromStr for MediaType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image" => Ok(MediaType::Image),
            "audio" => Ok(MediaType::Audio),
            "video" => Ok(MediaType::Video),
            _ => Err(format!("Unknown media type: {}", s)),
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
