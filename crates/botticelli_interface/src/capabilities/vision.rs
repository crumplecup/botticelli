//! Vision (image input) capability.

use crate::BotticelliDriver;

/// Trait for models that support image inputs (multimodal vision).
pub trait Vision: BotticelliDriver {
    /// Maximum number of images per request.
    fn max_images_per_request(&self) -> usize {
        1
    }

    /// Supported image formats (MIME types).
    fn supported_image_formats(&self) -> &[&'static str] {
        &["image/png", "image/jpeg", "image/webp", "image/gif"]
    }

    /// Maximum image size in bytes.
    fn max_image_size_bytes(&self) -> usize {
        5 * 1024 * 1024 // 5MB default
    }
}
