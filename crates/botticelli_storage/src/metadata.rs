//! Media metadata types.

use crate::MediaType;

/// Metadata about media being stored.
///
/// Note: Does not derive `Eq` or `Hash` due to `f32` fields which don't support these traits.
#[derive(Debug, Clone, PartialEq)]
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
