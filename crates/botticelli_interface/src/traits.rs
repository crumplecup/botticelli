//! Trait definitions for LLM backends and their capabilities.

use crate::{HealthStatus, ModelMetadata, StreamChunk, ToolDefinition};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Input};
use botticelli_error::BotticelliResult;
use futures_util::stream::Stream;
use std::pin::Pin;

/// Core trait that all LLM backends must implement.
///
/// This provides the minimal interface for synchronous text generation.
/// Additional capabilities are exposed through optional traits.
#[async_trait]
pub trait BotticelliDriver: Send + Sync {
    /// Generate model output given a multimodal request.
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse>;

    /// Provider name (e.g., "anthropic", "openai", "gemini").
    fn provider_name(&self) -> &'static str;

    /// Model identifier (e.g., "claude-3-5-sonnet-20241022").
    fn model_name(&self) -> &str;
}

/// Trait for models that support streaming responses.
#[async_trait]
pub trait Streaming: BotticelliDriver {
    /// Generate a streaming response.
    ///
    /// Returns a stream that yields chunks as they arrive from the API.
    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<Pin<Box<dyn Stream<Item = BotticelliResult<StreamChunk>> + Send>>>;
}

/// Trait for models that can generate embeddings.
#[async_trait]
pub trait Embeddings: BotticelliDriver {
    /// Generate embeddings for one or more text inputs.
    ///
    /// Returns a vector of embedding vectors, one per input.
    async fn embed(&self, inputs: &[String]) -> BotticelliResult<Vec<Vec<f32>>>;

    /// Dimensionality of the embedding vectors.
    fn embedding_dimensions(&self) -> usize;
}

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

/// Trait for models that support audio inputs and/or outputs.
///
/// This includes:
/// - Speech-to-text (audio input → text output)
/// - Text-to-speech (text input → audio output)
/// - Audio understanding (audio input → analysis)
/// - Audio generation (text/audio input → audio output)
pub trait Audio: BotticelliDriver {
    /// Maximum audio duration in seconds for input.
    fn max_audio_duration_seconds(&self) -> usize {
        60 // 1 minute default
    }

    /// Supported audio input formats (MIME types).
    fn supported_audio_input_formats(&self) -> &[&'static str] {
        &["audio/mp3", "audio/wav", "audio/ogg", "audio/webm"]
    }

    /// Supported audio output formats (MIME types).
    fn supported_audio_output_formats(&self) -> &[&'static str] {
        &["audio/mp3", "audio/wav"]
    }

    /// Maximum audio file size in bytes.
    fn max_audio_size_bytes(&self) -> usize {
        25 * 1024 * 1024 // 25MB default
    }
}

/// Trait for models that support video inputs and/or outputs.
///
/// This includes:
/// - Video understanding (video input → analysis)
/// - Video generation (text/image input → video output)
/// - Video-to-text (video input → description/transcript)
pub trait Video: BotticelliDriver {
    /// Maximum video duration in seconds for input.
    fn max_video_duration_seconds(&self) -> usize {
        60 // 1 minute default
    }

    /// Supported video input formats (MIME types).
    fn supported_video_input_formats(&self) -> &[&'static str] {
        &["video/mp4", "video/webm", "video/avi", "video/mov"]
    }

    /// Supported video output formats (MIME types).
    fn supported_video_output_formats(&self) -> &[&'static str] {
        &["video/mp4"]
    }

    /// Maximum video file size in bytes.
    fn max_video_size_bytes(&self) -> usize {
        100 * 1024 * 1024 // 100MB default
    }

    /// Maximum frames per second for analysis.
    fn max_fps(&self) -> Option<u32> {
        Some(1) // 1 FPS default for frame extraction
    }
}

/// Trait for models that can process structured documents.
///
/// This includes:
/// - PDF document understanding
/// - Office documents (DOCX, XLSX, PPTX)
/// - Code files with syntax awareness
/// - Plain text with structure preservation
pub trait DocumentProcessing: BotticelliDriver {
    /// Supported document formats (MIME types).
    fn supported_document_formats(&self) -> &[&'static str] {
        &[
            "application/pdf",
            "text/plain",
            "text/markdown",
            "application/json",
        ]
    }

    /// Maximum document size in bytes.
    fn max_document_size_bytes(&self) -> usize {
        10 * 1024 * 1024 // 10MB default
    }

    /// Maximum number of pages for paginated documents.
    fn max_pages(&self) -> Option<usize> {
        Some(100)
    }

    /// Whether the model preserves document structure (headings, tables, etc.).
    fn preserves_structure(&self) -> bool {
        false
    }
}

/// Trait for models that support function/tool calling.
#[async_trait]
pub trait ToolUse: BotticelliDriver {
    /// Generate with available tools/functions.
    ///
    /// The response may contain tool calls (in `Output::ToolCalls`) instead of
    /// or in addition to text output. Your application should execute these
    /// tools and send results back in a follow-up request.
    async fn generate_with_tools(
        &self,
        req: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse>;

    /// Maximum number of tools that can be provided.
    fn max_tools(&self) -> usize {
        128
    }

    /// Whether the model supports parallel tool calling (multiple tools in one turn).
    fn supports_parallel_tool_calls(&self) -> bool {
        false
    }
}

/// Trait for models that support structured JSON output.
#[async_trait]
pub trait JsonMode: BotticelliDriver {
    /// Generate output conforming to a JSON schema.
    async fn generate_json(
        &self,
        req: &GenerateRequest,
        schema: &serde_json::Value,
    ) -> BotticelliResult<serde_json::Value>;
}

/// Trait for models that can count tokens.
pub trait TokenCounting: BotticelliDriver {
    /// Count tokens in text using the model's tokenizer.
    fn count_tokens(&self, text: &str) -> BotticelliResult<usize>;

    /// Count tokens in a full request (all messages).
    fn count_request_tokens(&self, req: &GenerateRequest) -> BotticelliResult<usize> {
        let mut total = 0;
        for msg in &req.messages {
            for input in &msg.content {
                if let Input::Text(text) = input {
                    total += self.count_tokens(text)?;
                }
            }
        }
        Ok(total)
    }
}

/// Trait for models that support efficient batch processing.
#[async_trait]
pub trait BatchGeneration: BotticelliDriver {
    /// Generate responses for multiple requests in a single batch.
    ///
    /// May be more efficient than individual requests for some providers.
    async fn generate_batch(
        &self,
        requests: &[GenerateRequest],
    ) -> BotticelliResult<Vec<GenerateResponse>>;

    /// Maximum batch size supported.
    fn max_batch_size(&self) -> usize {
        10
    }
}

/// Trait for querying model metadata and capabilities.
pub trait Metadata: BotticelliDriver {
    /// Get comprehensive metadata about this model.
    fn metadata(&self) -> ModelMetadata;

    /// Maximum tokens in input context.
    fn max_input_tokens(&self) -> usize {
        self.metadata().max_input_tokens
    }

    /// Maximum tokens in output.
    fn max_output_tokens(&self) -> usize {
        self.metadata().max_output_tokens
    }
}

/// Trait for backends that support health checks.
#[async_trait]
pub trait Health: BotticelliDriver {
    /// Check if the backend is available and functioning.
    async fn health(&self) -> BotticelliResult<HealthStatus>;
}
