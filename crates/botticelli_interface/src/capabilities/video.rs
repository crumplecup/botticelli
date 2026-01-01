//! Video input/output capability.

use crate::BotticelliDriver;

/// Trait for models that support video inputs or outputs.
pub trait Video: BotticelliDriver {
    /// Supported video formats for input (MIME types).
    fn supported_video_input_formats(&self) -> &[&'static str] {
        &["video/mp4", "video/mpeg", "video/webm"]
    }

    /// Supported video formats for output (MIME types).
    fn supported_video_output_formats(&self) -> &[&'static str] {
        &["video/mp4"]
    }

    /// Maximum video duration in seconds.
    fn max_video_duration_seconds(&self) -> u32 {
        600 // 10 minutes default
    }

    /// Maximum video file size in bytes.
    fn max_video_size_bytes(&self) -> usize {
        100 * 1024 * 1024 // 100MB default
    }
}
