//! Groq AI LPU Inference API driver using OpenAI-compatible client.

use crate::openai_compat::{OpenAICompatError, OpenAICompatibleClient};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{BotticelliResult, GroqErrorKind, ModelsError, ModelsResult};
use botticelli_interface::{BotticelliDriver, StreamChunk, Streaming};
use botticelli_rate_limit::RateLimitConfig;
use futures_util::stream::Stream;
use std::pin::Pin;
use tracing::{debug, instrument};

/// Groq AI LPU Inference API driver.
#[derive(Debug, Clone)]
pub struct GroqDriver {
    inner: OpenAICompatibleClient,
}

impl GroqDriver {
    /// Creates a new Groq driver.
    ///
    /// Reads API token from `GROQ_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns error if API token is not set.
    #[instrument(skip_all, fields(model = %model))]
    pub fn new(model: String) -> ModelsResult<Self> {
        let api_key = std::env::var("GROQ_API_KEY").map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::Groq(
                GroqErrorKind::InvalidRequest(format!("GROQ_API_KEY not set: {}", e)),
            ))
        })?;

        Self::with_api_key(api_key, model)
    }

    /// Creates a new Groq driver with explicit API key.
    ///
    /// # Errors
    ///
    /// Returns error if client cannot be initialized.
    #[instrument(skip(api_key), fields(model = %model))]
    pub fn with_api_key(api_key: String, model: String) -> ModelsResult<Self> {
        let inner = OpenAICompatibleClient::new(
            api_key,
            model,
            "https://api.groq.com/openai/v1/chat/completions".to_string(),
            "groq",
        );

        Ok(Self { inner })
    }

    /// Converts OpenAICompatError to Groq-specific error.
    fn convert_error(error: OpenAICompatError) -> ModelsError {
        let kind = match error {
            OpenAICompatError::Http(msg) => GroqErrorKind::Api(msg),
            OpenAICompatError::Api { status, message } => {
                GroqErrorKind::Api(format!("API error {}: {}", status, message))
            }
            OpenAICompatError::RateLimit => GroqErrorKind::RateLimit,
            OpenAICompatError::ModelNotFound(model) => GroqErrorKind::ModelNotFound(model),
            OpenAICompatError::InvalidRequest(msg) => GroqErrorKind::InvalidRequest(msg),
            OpenAICompatError::ResponseParsing(msg) => GroqErrorKind::ResponseConversion(msg),
            OpenAICompatError::Builder(msg) => {
                GroqErrorKind::RequestConversion(format!("Builder error: {}", msg))
            }
        };

        ModelsError::new(botticelli_error::ModelsErrorKind::Groq(kind))
    }
}

#[async_trait]
impl BotticelliDriver for GroqDriver {
    #[instrument(skip(self, req), fields(provider = "groq", model = %self.inner.model_name()))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        self.inner
            .generate(req)
            .await
            .map_err(|e| Self::convert_error(e).into())
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        self.inner.rate_limits()
    }
}

#[async_trait]
impl Streaming for GroqDriver {
    #[instrument(skip(self, req), fields(provider = "groq", model = %self.inner.model_name()))]
    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<Pin<Box<dyn Stream<Item = BotticelliResult<StreamChunk>> + Send>>> {
        let response = self.generate(req).await?;

        let chunks: Vec<BotticelliResult<StreamChunk>> = response
            .outputs()
            .iter()
            .map(|output| {
                StreamChunk::builder()
                    .content(output.clone())
                    .is_final(true)
                    .build()
                    .map_err(|e| {
                        ModelsError::new(botticelli_error::ModelsErrorKind::Builder(e.to_string()))
                            .into()
                    })
            })
            .collect();

        Ok(Box::pin(futures_util::stream::iter(chunks)))
    }
}

impl botticelli_interface::TokenCounting for GroqDriver {
    #[instrument(skip(self, text), fields(text_len = text.len()))]
    fn count_tokens(&self, text: &str) -> Result<usize, botticelli_error::BotticelliError> {
        // Use tiktoken approximation for Groq (most models are GPT-based)
        let tokenizer = crate::gpt_tokenizer()?;
        let count = crate::count_tokens_tiktoken(text, &tokenizer);
        debug!(token_count = count, "Counted tokens for Groq");
        Ok(count)
    }
}
