use derive_getters::Getters;
use rmcp::tool;

/// Gemini model capabilities configuration.
///
/// This struct describes what features a Gemini model supports.
/// It's used by the `BotticelliDriver::capabilities()` method.
#[derive(Debug, Clone, Getters)]
pub struct ModelCapabilities {
    supports_streaming: bool,
    supports_tools: bool,
    supports_vision: bool,
    supports_video: bool,
    supports_audio: bool,
    supports_documents: bool,
    supports_json_mode: bool,
    supports_token_counting: bool,
    supports_batch: bool,
    supports_embeddings: bool,
}

impl ModelCapabilities {
    /// Create capabilities for a standard Gemini model.
    #[tool]
    pub fn standard() -> Self {
        Self {
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_video: true,
            supports_audio: true,
            supports_documents: true,
            supports_json_mode: true,
            supports_token_counting: true,
            supports_batch: false,
            supports_embeddings: false,
        }
    }

    /// Create capabilities for a Gemini embedding model.
    #[tool]
    pub fn embedding() -> Self {
        Self {
            supports_streaming: false,
            supports_tools: false,
            supports_vision: false,
            supports_video: false,
            supports_audio: false,
            supports_documents: false,
            supports_json_mode: false,
            supports_token_counting: false,
            supports_batch: true,
            supports_embeddings: true,
        }
    }
}
