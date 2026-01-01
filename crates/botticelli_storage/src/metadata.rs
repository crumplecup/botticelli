//! Media metadata types.

use crate::MediaType;

/// Metadata about media being stored.
///
/// Note: Does not derive `Eq` or `Hash` due to `f32` fields which don't support these traits.
///
/// # Example
///
/// ```rust
/// use botticelli_storage::{MediaMetadata, MediaMetadataBuilder, MediaType};
///
/// let metadata = MediaMetadataBuilder::default()
///     .media_type(MediaType::Image)
///     .mime_type("image/png".to_string())
///     .filename(Some("test.png".to_string()))
///     .width(Some(800))
///     .height(Some(600))
///     .build()
///     .expect("Valid metadata");
/// ```
#[derive(Debug, Clone, PartialEq, derive_getters::Getters, derive_builder::Builder)]
#[builder(setter(into))]
pub struct MediaMetadata {
    /// Type of media (image, audio, video)
    media_type: MediaType,
    /// MIME type (e.g., "image/png", "video/mp4")
    mime_type: String,
    /// Original filename (if available)
    #[builder(default)]
    filename: Option<String>,
    /// Image/video width in pixels
    #[builder(default)]
    width: Option<u32>,
    /// Image/video height in pixels
    #[builder(default)]
    height: Option<u32>,
    /// Audio/video duration in seconds
    #[builder(default)]
    duration_seconds: Option<f32>,
}
