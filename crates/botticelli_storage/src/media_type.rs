//! Media type enumeration.

use elicitation::{Prompt, Select};

/// Type of media content.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::EnumIter,
    strum::AsRefStr,
    derive_more::Display,
    derive_more::FromStr,
    elicitation::Elicit,
)]
pub enum MediaType {
    /// Image content (PNG, JPEG, WebP, etc.)
    #[display("image")]
    #[strum(serialize = "image")]
    Image,
    /// Audio content (MP3, WAV, OGG, etc.)
    #[display("audio")]
    #[strum(serialize = "audio")]
    Audio,
    /// Video content (MP4, WebM, AVI, etc.)
    #[display("video")]
    #[strum(serialize = "video")]
    Video,
}

impl MediaType {
    /// Convert to string representation for database storage.
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}
