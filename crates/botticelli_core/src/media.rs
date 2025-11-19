//! Media source types for multimodal content.

use serde::{Deserialize, Serialize};

/// Where media content is sourced from.
///
/// # Examples
///
/// ```
/// use botticelli_core::MediaSource;
///
/// let url = MediaSource::Url("https://example.com/image.png".to_string());
/// let base64 = MediaSource::Base64("iVBORw0KGgo...".to_string());
/// let binary = MediaSource::Binary(vec![0x89, 0x50, 0x4E, 0x47]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaSource {
    /// URL to fetch the content from
    Url(String),
    /// Base64-encoded content
    Base64(String),
    /// Raw binary data
    Binary(Vec<u8>),
}
