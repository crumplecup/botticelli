//! Media type enumeration.

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
    derive_more::Display,
)]
pub enum MediaType {
    /// Image content (PNG, JPEG, WebP, etc.)
    #[display("image")]
    Image,
    /// Audio content (MP3, WAV, OGG, etc.)
    #[display("audio")]
    Audio,
    /// Video content (MP4, WebM, AVI, etc.)
    #[display("video")]
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
