//! HuggingFace Inference API driver using OpenAI-compatible client.

use crate::openai_compat::{OpenAICompatError, OpenAICompatibleClient};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{BotticelliResult, HuggingFaceErrorKind, ModelsError, ModelsResult};
use botticelli_interface::{BotticelliDriver, StreamChunk, Streaming};
use botticelli_rate_limit::RateLimitConfig;
use futures_util::stream::Stream;
use std::pin::Pin;
use tracing::{debug, instrument};

/// HuggingFace Inference API driver.
#[derive(Debug, Clone)]
pub struct HuggingFaceDriver {
    inner: OpenAICompatibleClient,
}

impl HuggingFaceDriver {
    /// Creates a new HuggingFace driver.
    ///
    /// Reads API token from `HUGGINGFACE_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns error if API token is not set.
    #[instrument(skip_all, fields(model = %model))]
    pub fn new(model: String) -> ModelsResult<Self> {
        let api_token = std::env::var("HUGGINGFACE_API_KEY").map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::InvalidRequest(format!("HUGGINGFACE_API_KEY not set: {}", e)),
            ))
        })?;

        Self::with_api_token(api_token, model)
    }

    /// Creates a new HuggingFace driver with explicit API token.
    ///
    /// # Errors
    ///
    /// Returns error if client cannot be initialized.
    #[instrument(skip(api_token), fields(model = %model))]
    pub fn with_api_token(api_token: String, model: String) -> ModelsResult<Self> {
        let inner = OpenAICompatibleClient::new(
            api_token,
            model,
            "https://router.huggingface.co/v1/chat/completions".to_string(),
            "huggingface",
        );

        Ok(Self { inner })
    }

    /// Converts OpenAICompatError to HuggingFace-specific error.
    fn convert_error(error: OpenAICompatError) -> ModelsError {
        let kind = match error {
            OpenAICompatError::Http(msg) => HuggingFaceErrorKind::Api(msg),
            OpenAICompatError::Api { status, message } => {
                HuggingFaceErrorKind::Api(format!("API error {}: {}", status, message))
            }
            OpenAICompatError::RateLimit => HuggingFaceErrorKind::RateLimit,
            OpenAICompatError::ModelNotFound(model) => HuggingFaceErrorKind::ModelNotFound(model),
            OpenAICompatError::InvalidRequest(msg) => HuggingFaceErrorKind::InvalidRequest(msg),
            OpenAICompatError::ResponseParsing(msg) => {
                HuggingFaceErrorKind::ResponseConversion(msg)
            }
            OpenAICompatError::Builder(msg) => {
                HuggingFaceErrorKind::RequestConversion(format!("Builder error: {}", msg))
            }
        };

        ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(kind))
    }
}

#[async_trait]
impl BotticelliDriver for HuggingFaceDriver {
    #[instrument(skip(self, req), fields(provider = "huggingface", model = %self.inner.model_name()))]
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
impl Streaming for HuggingFaceDriver {
    #[instrument(skip(self, req), fields(provider = "huggingface", model = %self.inner.model_name()))]
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

impl botticelli_interface::TokenCounting for HuggingFaceDriver {
    #[instrument(skip(self, text), fields(text_len = text.len()))]
    fn count_tokens(&self, text: &str) -> Result<usize, botticelli_error::BotticelliError> {
        // Use tiktoken approximation for HuggingFace
        let tokenizer = crate::gpt_tokenizer()?;
        let count = crate::count_tokens_tiktoken(text, &tokenizer);
        debug!(token_count = count, "Counted tokens for HuggingFace");
        Ok(count)
    }
}
