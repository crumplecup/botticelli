//! vision trait implementation.

use botticelli_interface::Vision;

use crate::gemini::GeminiClient;

impl Vision for GeminiClient {
    fn max_images_per_request(&self) -> usize {
        16 // Gemini supports up to 16 images per request
    }

    fn supported_image_formats(&self) -> &[&'static str] {
        &["image/png", "image/jpeg", "image/webp", "image/heic", "image/heif"]
    }

    fn max_image_size_bytes(&self) -> usize {
        20 * 1024 * 1024 // 20MB for Gemini
    }
}
