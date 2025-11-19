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

mod filesystem;
mod media_type;
mod metadata;
mod reference;
mod storage;

pub use filesystem::FileSystemStorage;
pub use media_type::MediaType;
pub use metadata::MediaMetadata;
pub use reference::MediaReference;
pub use storage::MediaStorage;
