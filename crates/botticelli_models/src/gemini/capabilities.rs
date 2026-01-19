use rmcp::tool;

/// Gemini model capabilities configuration.
///
/// This struct describes what features a Gemini model supports.
/// It's used by the `BotticelliDriver::capabilities()` method.
#[derive(Debug, Clone)]
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

    /// Check if model supports streaming.
    #[must_use]
    #[tool]
    pub fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    /// Check if model supports tool calling.
    #[must_use]
    #[tool]
    pub fn supports_tool_calling(&self) -> bool {
        self.supports_tools
    }

    /// Check if model supports vision.
    #[must_use]
    #[tool]
    pub fn supports_vision(&self) -> bool {
        self.supports_vision
    }

    /// Check if model supports video.
    #[must_use]
    #[tool]
    pub fn supports_video(&self) -> bool {
        self.supports_video
    }

    /// Check if model supports audio.
    #[must_use]
    #[tool]
    pub fn supports_audio(&self) -> bool {
        self.supports_audio
    }

    /// Check if model supports documents.
    #[must_use]
    #[tool]
    pub fn supports_documents(&self) -> bool {
        self.supports_documents
    }

    /// Check if model supports JSON mode.
    #[must_use]
    #[tool]
    pub fn supports_json_mode(&self) -> bool {
        self.supports_json_mode
    }

    /// Check if model supports token counting.
    #[must_use]
    #[tool]
    pub fn supports_token_counting(&self) -> bool {
        self.supports_token_counting
    }

    /// Check if model supports batch generation.
    #[must_use]
    #[tool]
    pub fn supports_batch(&self) -> bool {
        self.supports_batch
    }

    /// Check if model supports embeddings.
    #[must_use]
    #[tool]
    pub fn supports_embeddings(&self) -> bool {
        self.supports_embeddings
    }
}
