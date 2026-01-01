//! Audio input/output capability.

use crate::BotticelliDriver;

/// Trait for models that support audio inputs or outputs.
pub trait Audio: BotticelliDriver {
    /// Supported audio formats for input (MIME types).
    fn supported_audio_input_formats(&self) -> &[&'static str] {
        &["audio/mpeg", "audio/wav", "audio/ogg"]
    }

    /// Supported audio formats for output (MIME types).
    fn supported_audio_output_formats(&self) -> &[&'static str] {
        &["audio/mpeg", "audio/wav"]
    }

    /// Maximum audio duration in seconds.
    fn max_audio_duration_seconds(&self) -> u32 {
        300 // 5 minutes default
    }
}
