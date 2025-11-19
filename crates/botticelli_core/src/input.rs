//! Input types for LLM requests.

use crate::MediaSource;
use serde::{Deserialize, Serialize};

/// Supported input types to LLMs.
///
/// # Examples
///
/// ```
/// use botticelli_core::{Input, MediaSource};
///
/// // Text input
/// let text = Input::Text("Hello, world!".to_string());
///
/// // Image input with URL
/// let image = Input::Image {
///     mime: Some("image/png".to_string()),
///     source: MediaSource::Url("https://example.com/image.png".to_string()),
/// };
///
/// // Document input with base64
/// let doc = Input::Document {
///     mime: Some("application/pdf".to_string()),
///     source: MediaSource::Base64("JVBERi0xLj...".to_string()),
///     filename: Some("report.pdf".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Input {
    /// Plain text input.
    Text(String),

    /// Image input (PNG, JPEG, WebP, GIF, etc.).
    Image {
        /// MIME type, e.g., "image/png" or "image/jpeg"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
    },

    /// Audio input (MP3, WAV, OGG, etc.).
    Audio {
        /// MIME type, e.g., "audio/mp3" or "audio/wav"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
    },

    /// Video input (MP4, WebM, AVI, etc.).
    Video {
        /// MIME type, e.g., "video/mp4" or "video/webm"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
    },

    /// Document input (PDF, DOCX, TXT, etc.).
    Document {
        /// MIME type, e.g., "application/pdf" or "text/plain"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
        /// Optional filename for context
        filename: Option<String>,
    },
}
