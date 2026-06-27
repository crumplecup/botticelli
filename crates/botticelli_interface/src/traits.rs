//! Trait definitions for LLM backends and their capabilities.

use crate::{Capabilities, HealthStatus, ModelMetadata, StreamChunk};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Input, ToolDefinition};
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

    /// Rate limits for this driver.
    ///
    /// Returns the rate limit configuration for carousel budget tracking.
    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig;

    /// Query provider capabilities.
    ///
    /// Returns capability flags indicating which optional features
    /// this provider supports (streaming, tools, vision, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// use botticelli_interface::BotticelliDriver;
    ///
    /// fn check_capabilities(driver: &dyn BotticelliDriver) {
    ///     let caps = driver.capabilities();
    ///     if caps.tool_calling {
    ///         println!("Provider supports tool calling");
    ///     }
    ///     if caps.streaming {
    ///         println!("Provider supports streaming");
    ///     }
    /// }
    /// ```
    fn capabilities(&self) -> Capabilities {
        // Default: no optional capabilities
        Capabilities {
            streaming: false,
            tool_calling: false,
            vision: false,
            audio: false,
            video: false,
            embeddings: false,
            json_mode: false,
            batch_generation: false,
        }
    }

    /// Attempt a streaming generate.
    ///
    /// Returns `Ok(Some(stream))` when this driver supports streaming.
    /// Returns `Ok(None)` (the default) when streaming is not available,
    /// signalling the caller to fall back to [`Self::generate`].
    ///
    /// Models that emit chain-of-thought tokens (DeepSeek-R1, Qwen3…) set
    /// [`StreamChunk::is_thinking`] on those chunks so the UI can render them
    /// separately from the final answer.  Models that do not support thinking
    /// simply omit those chunks — the caller receives only content chunks.
    async fn stream_generate(
        &self,
        _req: &GenerateRequest,
    ) -> BotticelliResult<Option<Pin<Box<dyn Stream<Item = BotticelliResult<StreamChunk>> + Send>>>>
    {
        Ok(None)
    }
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

/// Trait for models that support function/tool calling (new architecture).
///
/// This trait represents the clean "trait sandwich" architecture where
/// tools are passed as explicit parameters, not hidden in request fields.
///
/// # Design Principle
///
/// Tools are a **capability**, not data. They should be passed explicitly
/// to methods that support them, making the API self-documenting and
/// type-safe.
///
/// # Examples
///
/// ```no_run
/// use botticelli_interface::{ToolCalling, BotticelliDriver};
/// use botticelli_core::{GenerateRequest, Message, Role, Input, ToolDefinition};
/// use serde_json::json;
///
/// async fn example(driver: &dyn ToolCalling) -> Result<(), Box<dyn std::error::Error>> {
///     let tool = ToolDefinition::new(
///         "get_weather".to_string(),
///         "Get current weather".to_string(),
///         json!({"type": "object", "properties": {}}),
///     );
///     
///     let message = Message::new(Role::User, vec![Input::Text("What's the weather?".to_string())]);
///     let request = GenerateRequest::new(vec![message]);
///     
///     // Tools passed explicitly via trait method
///     let response = driver.generate_with_tools(&request, &[tool]).await?;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait ToolCalling: BotticelliDriver {
    /// Generate response with tool calling enabled.
    ///
    /// The model can request tool executions via `Output::ToolCalls`.
    /// Client is responsible for executing tools and providing results
    /// in subsequent messages.
    ///
    /// # Arguments
    ///
    /// * `request` - Generation request (messages, temperature, etc.)
    /// * `tools` - Available tools for the model to call
    ///
    /// # Returns
    ///
    /// Response which may contain `Output::ToolCalls` if the model wants
    /// to use tools, or `Output::Text` for final text response.
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[ToolDefinition],
    ) -> BotticelliResult<GenerateResponse>;

    /// Maximum number of tools supported in single request.
    fn max_tools(&self) -> usize {
        128 // Anthropic default
    }

    /// Whether provider supports parallel tool calls.
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
        for msg in req.messages() {
            for input in msg.content() {
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
        *self.metadata().max_input_tokens()
    }

    /// Maximum tokens in output.
    fn max_output_tokens(&self) -> usize {
        *self.metadata().max_output_tokens()
    }
}

/// Trait for backends that support health checks.
#[async_trait]
pub trait Health: BotticelliDriver {
    /// Check if the backend is available and functioning.
    async fn health(&self) -> BotticelliResult<HealthStatus>;
}

/// Trait for querying database tables in narratives.
///
/// This trait provides a platform-agnostic interface for narrative executors
/// to query database tables without directly depending on database crates.
/// Implementations live in `botticelli_database`.
#[async_trait]
pub trait TableQueryRegistry: Send + Sync {
    /// Query a database table and return results in the specified format.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table to query
    /// * `columns` - Optional list of specific columns to select
    /// * `where_clause` - Optional WHERE clause for filtering
    /// * `limit` - Optional maximum number of rows
    /// * `offset` - Optional offset for pagination
    /// * `order_by` - Optional ORDER BY clause
    /// * `format` - Output format: "json", "markdown", or "csv"
    ///
    /// # Returns
    ///
    /// Formatted query results as a string, ready for LLM consumption.
    async fn query_table(
        &self,
        query: &crate::TableQueryView,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Query a table and atomically delete the returned rows (destructive read).
    ///
    /// This is useful for workflows where content should be processed once and removed,
    /// such as pulling posts from a queue for curation.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table to query
    /// * `columns` - Optional list of specific columns to select
    /// * `where_clause` - Optional WHERE clause for filtering
    /// * `limit` - Optional maximum number of rows
    /// * `offset` - Optional offset for pagination
    /// * `order_by` - Optional ORDER BY clause
    /// * `format` - Output format: "json", "markdown", or "csv"
    ///
    /// # Returns
    ///
    /// Formatted query results as a string, with those rows deleted from the table.
    async fn query_and_delete_table(
        &self,
        query: &crate::TableQueryView,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

/// Trait for managing generated content in database tables.
///
/// This trait provides a platform-agnostic interface for working with
/// content generated by LLMs and stored in dynamically created tables.
/// Implementations live in `botticelli_database`.
#[async_trait]
pub trait ContentRepository: Send + Sync {
    /// List generated content from a table.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the content table
    /// * `status_filter` - Optional review status filter
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    ///
    /// Vector of JSON objects representing table rows
    async fn list_content(
        &self,
        table_name: &str,
        status_filter: Option<&str>,
        limit: usize,
    ) -> BotticelliResult<Vec<serde_json::Value>>;

    /// Update the review status of a content item.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the content table
    /// * `id` - ID of the content item
    /// * `new_status` - New review status ("pending", "approved", "rejected")
    async fn update_review_status(
        &self,
        table_name: &str,
        id: i64,
        new_status: &str,
    ) -> BotticelliResult<()>;

    /// Delete a content item.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the content table
    /// * `id` - ID of the content item
    async fn delete_content(&self, table_name: &str, id: i64) -> BotticelliResult<()>;

    /// Pull content items and delete them from the source table (destructive read).
    ///
    /// This is useful for pipeline workflows where content moves between tables.
    /// Atomically retrieves N items and removes them from the source table.
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the content table
    /// * `limit` - Maximum number of items to pull
    ///
    /// # Returns
    ///
    /// Vector of JSON objects representing the pulled (and deleted) rows
    async fn pull_and_delete(
        &self,
        table_name: &str,
        limit: usize,
    ) -> BotticelliResult<Vec<serde_json::Value>>;
}

// Blanket implementations for Arc<T> where T implements the trait
// This allows Arc-wrapped drivers to be used directly

#[async_trait]
impl<T: BotticelliDriver + ?Sized> BotticelliDriver for std::sync::Arc<T> {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        (**self).generate(req).await
    }

    fn provider_name(&self) -> &'static str {
        (**self).provider_name()
    }

    fn model_name(&self) -> &str {
        (**self).model_name()
    }

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        (**self).rate_limits()
    }

    async fn stream_generate(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<Option<Pin<Box<dyn Stream<Item = BotticelliResult<StreamChunk>> + Send>>>>
    {
        (**self).stream_generate(req).await
    }
}
