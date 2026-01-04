//! Metadata trait implementation for Gemini client.

use super::GeminiClient;
use crate::GeminiModel;
use botticelli_interface::Metadata;

impl Metadata for GeminiClient {
    type ModelMetadata = GeminiModel;

    fn metadata(&self) -> &Self::ModelMetadata {
        // Return metadata about the default model
        &GeminiModel::Gemini25Flash
    }
}
