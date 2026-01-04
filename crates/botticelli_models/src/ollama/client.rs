//! Ollama LLM client implementation.

use std::sync::Arc;

use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest as OllamaRequest;

use super::conversion::{messages_to_prompt, response_to_output};
use botticelli_core::{Capabilities, FinishReason, GenerateRequest, GenerateResponse, StreamChunk};
use botticelli_error::{ModelsError, OllamaErrorKind, OllamaResult};

use botticelli_interface::BotticelliDriver;
use tracing::{debug, info, instrument, warn};

/// Ollama LLM client for local model execution.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    /// Ollama client instance
    client: Ollama,

    /// Model name (e.g., "llama2", "mistral", "codellama")
    model_name: String,

    /// Ollama server URL
    base_url: String,
}

impl OllamaClient {
    /// Create a new Ollama client with default localhost connection.
    #[instrument(skip_all, fields(model_name))]
    pub fn new(model_name: impl Into<String>) -> OllamaResult<Self> {
        Self::new_with_url(model_name, "http://localhost:11434")
    }

    /// Create a new Ollama client with custom server URL.
    #[instrument(skip_all, fields(model_name, base_url))]
    pub fn new_with_url(
        model_name: impl Into<String>,
        base_url: impl Into<String>,
    ) -> OllamaResult<Self> {
        let model_name = model_name.into();
        let base_url = base_url.into();

        info!(
            model = %model_name,
            url = %base_url,
            "Creating Ollama client"
        );

        let client = Ollama::new(base_url.clone(), 11434);

        Ok(Self {
            client,
            model_name,
            base_url,
        })
    }

    /// Check if Ollama server is running and model is available.
    #[instrument(skip(self))]
    pub async fn validate(&self) -> OllamaResult<()> {
        debug!("Validating Ollama server and model availability");

        // Check if server is reachable
        match self.client.list_local_models().await {
            Ok(models) => {
                debug!(count = models.len(), "Found local models");

                // Check if our model exists
                let model_exists = models.iter().any(|m| m.name == self.model_name);

                if !model_exists {
                    warn!(
                        model = %self.model_name,
                        available = ?models.iter().map(|m| &m.name).collect::<Vec<_>>(),
                        "Model not found locally"
                    );

                    return Err(OllamaErrorKind::ModelNotFound(self.model_name.clone()).into());
                }

                info!("Ollama server and model validated");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "Failed to connect to Ollama server");
                Err(OllamaErrorKind::ServerNotRunning(self.base_url.clone()).into())
            }
        }
    }

    /// Pull model if not available locally.
    #[instrument(skip(self))]
    pub async fn ensure_model(&self) -> OllamaResult<()> {
        debug!("Ensuring model is available");

        match self.validate().await {
            Ok(()) => {
                debug!("Model already available");
                Ok(())
            }
            Err(_) => {
                info!(model = %self.model_name, "Pulling model");

                self.client
                    .pull_model(self.model_name.clone(), false)
                    .await
                    .map_err(|e| OllamaErrorKind::ModelPullFailed(Arc::new(e)))?;

                info!("Model pulled successfully");
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl BotticelliDriver for OllamaClient {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = ModelsError;
    type RateLimitConfig = botticelli_rate_limit::RateLimitConfig;
    type Capabilities = Capabilities;

    #[instrument(skip(self, request))]
    async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse, Self::Error> {
        debug!("Generating with Ollama");

        // Convert messages to prompt
        let prompt = messages_to_prompt(request.messages());

        debug!(prompt_length = prompt.len(), "Converted messages to prompt");

        // Create Ollama request
        let ollama_req = OllamaRequest::new(self.model_name.clone(), prompt);

        // Execute generation (no rate limiting needed for local)
        let response = self
            .client
            .generate(ollama_req)
            .await
            .map_err(|e| OllamaErrorKind::ApiError(Arc::new(e)))?;

        debug!(
            response_length = response.response.len(),
            "Received response from Ollama"
        );

        let output = response_to_output(response);
        GenerateResponse::builder()
            .outputs(vec![output])
            .stop_reason(botticelli_core::StopReason::EndTurn)
            .build()
            .map_err(|e| OllamaErrorKind::Builder(e.to_string()).into())
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        // No rate limits for local execution
        static DEFAULT_CONFIG: std::sync::OnceLock<botticelli_rate_limit::RateLimitConfig> =
            std::sync::OnceLock::new();
        DEFAULT_CONFIG
            .get_or_init(|| botticelli_rate_limit::RateLimitConfig::unlimited("ollama-local"))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
            .with_streaming(true)
            .with_tool_calling(false)
            .with_vision(false)
            .with_audio(false)
            .with_video(false)
            .with_embeddings(false)
            .with_json_mode(false)
            .with_batch_generation(false)
    }
}

#[async_trait::async_trait]
impl botticelli_interface::Streaming for OllamaClient {
    type StreamChunk = StreamChunk;

    #[instrument(skip(self, request))]
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
    ) -> Result<
        std::pin::Pin<
            Box<dyn futures_util::Stream<Item = Result<StreamChunk, ModelsError>> + Send>,
        >,
        ModelsError,
    > {
        debug!("Starting streaming generation with Ollama");

        // Convert messages to prompt
        let prompt = messages_to_prompt(request.messages());

        debug!(prompt_length = prompt.len(), "Converted messages to prompt");

        // Create Ollama request
        let ollama_req = OllamaRequest::new(self.model_name.clone(), prompt);

        // Execute streaming generation
        let mut stream = self
            .client
            .generate_stream(ollama_req)
            .await
            .map_err(|e| OllamaErrorKind::ApiError(Arc::new(e)))?;

        // Convert Ollama stream to Botticelli StreamChunk
        // ollama-rs returns a stream of Vec<GenerationResponse>
        let mapped_stream = async_stream::stream! {
            while let Some(responses) = tokio_stream::StreamExt::next(&mut stream).await {
                match responses {
                    Ok(generation_responses) => {
                        for response in generation_responses {
                            let is_final = response.done;

                            let chunk_result = if is_final {
                                StreamChunk::builder()
                                    .content(botticelli_core::Output::Text(response.response.clone()))
                                    .is_final(is_final)
                                    .finish_reason(Some(FinishReason::Stop))
                                    .build()
                            } else {
                                StreamChunk::builder()
                                    .content(botticelli_core::Output::Text(response.response.clone()))
                                    .is_final(is_final)
                                    .build()
                            };

                            match chunk_result {
                                Ok(chunk) => yield Ok(chunk),
                                Err(e) => {
                                    yield Err(OllamaErrorKind::ConversionError(e.to_string()).into());
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(OllamaErrorKind::ApiError(Arc::new(e)).into());
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(mapped_stream))
    }
}

impl botticelli_interface::TokenCounting for OllamaClient {
    #[instrument(skip(self, text), fields(text_len = text.len()))]
    fn count_tokens(&self, text: &str) -> Result<usize, ModelsError> {
        // Use tiktoken approximation for Ollama
        // Different models have different tokenizers, but this is a reasonable default
        let tokenizer = crate::gpt_tokenizer()?;
        let count = crate::count_tokens_tiktoken(text, &tokenizer);
        debug!(token_count = count, "Counted tokens for Ollama");
        Ok(count)
    }

    #[instrument(skip(self, req))]
    fn count_request_tokens(&self, req: &GenerateRequest) -> Result<usize, ModelsError> {
        let tokenizer = crate::gpt_tokenizer()?;
        let mut total = 0;
        for msg in req.messages() {
            for input in msg.content() {
                if let botticelli_core::Input::Text(text) = input {
                    total += crate::count_tokens_tiktoken(text, &tokenizer);
                }
            }
        }
        Ok(total)
    }
}
