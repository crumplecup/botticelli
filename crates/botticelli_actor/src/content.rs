//! Content types for social media posts.

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Media types supported for attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaType {
    /// Image (PNG, JPEG, GIF, etc.)
    Image,
    /// Video (MP4, MOV, etc.)
    Video,
    /// Audio (MP3, WAV, etc.)
    Audio,
}

/// Media attachment for content.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Getters, Serialize, Deserialize, derive_builder::Builder,
)]
pub struct MediaAttachment {
    /// URL to media file.
    url: String,

    /// Type of media.
    media_type: MediaType,

    /// Alt text for accessibility.
    #[builder(default)]
    #[serde(default)]
    alt_text: Option<String>,
}

/// Content to be posted to social media.
#[derive(Debug, Clone, PartialEq, Eq, Getters, Serialize, Deserialize, derive_builder::Builder)]
pub struct Content {
    /// Text content.
    #[builder(default)]
    #[serde(default)]
    text: Option<String>,

    /// Media attachments.
    #[builder(default)]
    #[serde(default)]
    media: Vec<MediaAttachment>,

    /// Platform-specific metadata.
    #[builder(default)]
    #[serde(default)]
    metadata: HashMap<String, String>,
}
