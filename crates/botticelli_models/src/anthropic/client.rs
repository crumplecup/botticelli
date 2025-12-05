use crate::{AnthropicContentBlock, AnthropicMessage, AnthropicRequest, AnthropicResponse};
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output, Role};
use botticelli_error::{AnthropicErrorKind, ModelsError};
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
use reqwest::Client;
use tracing::{debug, error, instrument};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic API client.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicClient {
    /// Creates a new Anthropic client.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Anthropic API key
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let model = model.into();
        debug!("Creating new Anthropic client");
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    /// Sends a request to the Anthropic API.
    #[instrument(skip(self, request), fields(model = %request.model()))]
    pub async fn generate_anthropic(
        &self,
        request: &AnthropicRequest,
    ) -> Result<AnthropicResponse, ModelsError> {
        debug!("Sending request to Anthropic API");

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, "Failed to send request to Anthropic API");
                ModelsError::new(AnthropicErrorKind::Http(format!("Request failed: {}", e)).into())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "Anthropic API returned error");
            return Err(ModelsError::new(
                AnthropicErrorKind::ApiError {
                    status: status.as_u16(),
                    message: body,
                }
                .into(),
            ));
        }

        let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
            error!(error = ?e, "Failed to parse Anthropic response");
            ModelsError::new(
                AnthropicErrorKind::Parse(format!("Failed to parse response: {}", e)).into(),
            )
        })?;

        debug!(response_id = %anthropic_response.id(), "Received response from Anthropic");
        Ok(anthropic_response)
    }

    /// Converts a Botticelli GenerateRequest to an Anthropic API request.
    #[instrument(skip(request))]
    fn convert_request(&self, request: &GenerateRequest) -> Result<AnthropicRequest, ModelsError> {
        debug!("Converting GenerateRequest to AnthropicRequest");

        let messages: Result<Vec<AnthropicMessage>, ModelsError> = request
            .messages()
            .iter()
            .map(|msg| {
                let content: Vec<AnthropicContentBlock> = msg
                    .content()
                    .iter()
                    .filter_map(|input| match input {
                        Input::Text(text) => {
                            Some(AnthropicContentBlock::Text { text: text.clone() })
                        }
                        _ => {
                            debug!("Skipping non-text input (not supported by Anthropic)");
                            None
                        }
                    })
                    .collect();

                if content.is_empty() {
                    return Err(ModelsError::new(
                        AnthropicErrorKind::ConversionError(
                            "Message must have at least one text content block".to_string(),
                        )
                        .into(),
                    ));
                }

                let role = match msg.role() {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => {
                        return Err(ModelsError::new(
                            AnthropicErrorKind::ConversionError(
                                "System role not supported in messages (use system parameter)"
                                    .to_string(),
                            )
                            .into(),
                        ));
                    }
                };

                AnthropicMessage::builder()
                    .role(role)
                    .content(content)
                    .build()
                    .map_err(|e| {
                        ModelsError::new(AnthropicErrorKind::Builder(e.to_string()).into())
                    })
            })
            .collect();

        let messages = messages?;

        let mut builder = AnthropicRequest::builder()
            .model(&self.model)
            .max_tokens(4096u32) // Default, could be configurable
            .messages(messages);

        if let Some(temp) = request.temperature() {
            builder = builder.temperature(*temp);
        }

        builder
            .build()
            .map_err(|e| ModelsError::new(AnthropicErrorKind::Builder(e.to_string()).into()))
    }

    /// Converts an Anthropic API response to a Botticelli GenerateResponse.
    #[instrument(skip(response))]
    fn convert_response(response: &AnthropicResponse) -> Result<GenerateResponse, ModelsError> {
        debug!("Converting AnthropicResponse to GenerateResponse");

        let outputs: Vec<Output> = response
            .content()
            .iter()
            .map(|content| Output::Text(content.text().clone()))
            .collect();

        GenerateResponse::builder()
            .outputs(outputs)
            .build()
            .map_err(|e| ModelsError::new(AnthropicErrorKind::Builder(e.to_string()).into()))
    }
}

#[async_trait::async_trait]
impl BotticelliDriver for AnthropicClient {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        // Default conservative rate limits for Anthropic
        // TODO: Make this configurable
        static DEFAULT_RATE_LIMITS: RateLimitConfig = RateLimitConfig {
            requests_per_minute: 50,
            tokens_per_minute: 40_000,
            requests_per_day: 1_000,
            tokens_per_day: 1_000_000,
        };
        &DEFAULT_RATE_LIMITS
    }

    #[instrument(skip(self, request))]
    async fn generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, botticelli_error::BotticelliError> {
        debug!("Generating response with Anthropic");

        let anthropic_request = self.convert_request(request)?;
        let anthropic_response = self.generate_anthropic(&anthropic_request).await?;
        let response = Self::convert_response(&anthropic_response)?;

        Ok(response)
    }
}

impl botticelli_interface::TokenCounting for AnthropicClient {
    #[instrument(skip(self, text), fields(text_len = text.len()))]
    fn count_tokens(&self, text: &str) -> Result<usize, botticelli_error::BotticelliError> {
        // Use tiktoken approximation for Claude (cl100k_base encoding)
        let tokenizer = crate::claude_tokenizer()?;
        let count = crate::count_tokens_tiktoken(text, &tokenizer);
        debug!(token_count = count, "Counted tokens for Anthropic");
        Ok(count)
    }
}
