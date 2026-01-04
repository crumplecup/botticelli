//! HuggingFace Inference API driver using OpenAI-compatible client.

use crate::openai::OpenAIibleClient;
use async_trait::async_trait;
use botticelli_core::Capabilities;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{BotticelliResult, ModelsResult, OpenAIErrorKind};
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
use std::sync::Arc;
use tracing::{debug, instrument};

/// HuggingFace Inference API driver.
#[derive(Debug, Clone)]
pub struct HuggingFaceDriver {
    inner: OpenAIibleClient,
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
        let api_token = std::env::var("HUGGINGFACE_API_KEY")
            .map_err(|e| OpenAIErrorKind::EnvVar(Arc::new(e)))?;

        Self::with_api_token(api_token, model)
    }

    /// Creates a new HuggingFace driver with explicit API token.
    ///
    /// # Errors
    ///
    /// Returns error if client cannot be initialized.
    #[instrument(skip(api_token), fields(model = %model))]
    pub fn with_api_token(api_token: String, model: String) -> ModelsResult<Self> {
        let inner = OpenAIibleClient::new(
            api_token,
            model,
            "https://router.huggingface.co/v1/chat/completions".to_string(),
            "huggingface".to_string(),
        );

        Ok(Self { inner })
    }
}

#[async_trait]
impl BotticelliDriver for HuggingFaceDriver {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = botticelli_error::BotticelliError;
    type RateLimitConfig = botticelli_rate_limit::RateLimitConfig;
    type Capabilities = Capabilities;

    #[instrument(skip(self, req), fields(provider = "huggingface", model = %self.inner.model_name()))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        Ok(self.inner.generate(req).await?)
    }

    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        self.inner.rate_limits()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
            .with_streaming(false) // HuggingFace doesn't have real streaming
            .with_tool_calling(false)
            .with_vision(false)
            .with_audio(false)
            .with_video(false)
            .with_embeddings(false)
            .with_json_mode(true)
            .with_batch_generation(false)
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

    #[instrument(skip(self, req))]
    fn count_request_tokens(
        &self,
        req: &GenerateRequest,
    ) -> Result<usize, botticelli_error::BotticelliError> {
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
