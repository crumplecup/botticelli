//! Groq AI LPU Inference API driver using OpenAI-compatible client.

use crate::openai_compat::{OpenAICompatError, OpenAICompatibleClient};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{BotticelliError, BotticelliResult, GroqErrorKind, ModelsError, ModelsResult};
use botticelli_core::Capabilities;
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
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
        let api_key = std::env::var("GROQ_API_KEY")
            .map_err(|e| GroqErrorKind::EnvVar(e))?;

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
            "groq".to_string(),
        );

        Ok(Self { inner })
    }

    /// Converts OpenAICompatError to Groq-specific error.
    fn convert_error(error: OpenAICompatError) -> GroqErrorKind {
        match error {
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
        }
    }
}

#[async_trait]
impl BotticelliDriver for GroqDriver {
    type Request = GenerateRequest;
    type Response = GenerateResponse;
    type Error = botticelli_error::BotticelliError;
    type RateLimitConfig = botticelli_rate_limit::RateLimitConfig;
    type Capabilities = Capabilities;

    #[instrument(skip(self, req), fields(provider = "groq", model = %self.inner.model_name()))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        self.inner
            .generate(req)
            .await
            .map_err(|e| {
                let kind = Self::convert_error(e);
                ModelsError::from(kind).into()
            })
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
            .with_streaming(false)   // Groq doesn't have real streaming
            .with_tool_calling(true) // Groq supports tool calling via OpenAI-compatible API
            .with_vision(false)
            .with_audio(false)
            .with_video(false)
            .with_embeddings(false)
            .with_json_mode(true)
            .with_batch_generation(false)
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

    #[instrument(skip(self, req))]
    fn count_request_tokens(&self, req: &GenerateRequest) -> Result<usize, botticelli_error::BotticelliError> {
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

#[async_trait]
impl botticelli_interface::ToolCalling for GroqDriver {
    type Error = BotticelliError;
    type ToolDefinition = botticelli_core::ToolDefinition;

    #[instrument(skip(self, request, tools), fields(tool_count = tools.len()))]
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[botticelli_core::ToolDefinition],
    ) -> BotticelliResult<GenerateResponse> {
        self.inner.generate_with_tools(request, tools).await
    }
}
