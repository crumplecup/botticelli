//! Metadata trait implementation for Gemini client.

use botticelli_interface::Metadata;
use crate::GeminiModel;
use super::GeminiClient;

impl Metadata for GeminiClient {
    type ModelMetadata = GeminiModel;

    fn metadata(&self) -> &Self::ModelMetadata {
        // Return metadata about the default model
        &GeminiModel::Gemini25Flash
    }
}
