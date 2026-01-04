//! Generic client for OpenAI-compatible APIs.

use botticelli_error::{OpenAICompatError, OpenAICompatErrorKind};
use crate::openai_compat::{ChatRequest, ChatResponse, conversions};
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_rate_limit::RateLimitConfig;
use derive_getters::Getters;
use reqwest::Client;
use tracing::{debug, error, instrument};

/// Generic client for any OpenAI-compatible API.
///
/// This client handles the common OpenAI chat completions format used by
/// HuggingFace, Groq, and potentially other providers.
#[derive(Debug, Clone, Getters)]
pub struct OpenAICompatibleClient {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Model name
    model: String,
    /// Base URL
    base_url: String,
    /// Provider name
    provider_name: String,
    /// Rate limits configuration
    rate_limits: RateLimitConfig,
}

impl OpenAICompatibleClient {
    /// Creates a new OpenAI-compatible client.
    ///
    /// # Arguments
    ///
    /// * `api_key` - API key for authentication
    /// * `model` - Model identifier
    /// * `base_url` - Base URL for the API endpoint
    /// * `provider_name` - Name of the provider (for logging/tracing)
    #[instrument(skip(api_key), fields(provider = %provider_name, model = %model))]
    pub fn new(
        api_key: String,
        model: String,
        base_url: String,
        provider_name: String,
    ) -> Self {
        let client = Client::new();
        let rate_limits = RateLimitConfig::unlimited(&provider_name);

        debug!(
            provider = %provider_name,
            model = %model,
            url = %base_url,
            "Created OpenAI-compatible client"
        );

        Self {
            client,
            api_key,
            model,
            base_url,
            provider_name,
            rate_limits,
        }
    }

    /// Generates a response from the API.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response cannot be parsed.
    #[instrument(skip(self, req), fields(provider = self.provider_name, model = %self.model))]
    pub async fn generate(
        &self,
        req: &GenerateRequest,
    ) -> Result<GenerateResponse, OpenAICompatError> {
        let chat_request = conversions::to_chat_request(req, &self.model)?;
        let chat_response = self.generate_internal(&chat_request).await?;
        conversions::from_chat_response(&chat_response)
    }



    /// Internal method to send a ChatRequest and get ChatResponse.
    ///
    /// Used by both regular generation and tool calling implementations.
    #[instrument(skip(self, chat_request), fields(provider = self.provider_name))]
    pub(crate) async fn generate_internal(
        &self,
        chat_request: &ChatRequest,
    ) -> Result<ChatResponse, OpenAICompatError> {
        debug!(
            provider = self.provider_name,
            model = %self.model,
            message_count = chat_request.messages().len(),
            "Sending request"
        );

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| {
                error!(provider = self.provider_name, error = ?e, "HTTP request failed");
                OpenAICompatErrorKind::Http(std::sync::Arc::new(e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!(
                provider = self.provider_name,
                status = %status,
                error = %error_text,
                "API error"
            );

            return Err(OpenAICompatErrorKind::Api {
                status: status.as_u16(),
                message: error_text,
            }.into());
        }

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            error!(provider = self.provider_name, error = ?e, "Failed to parse response");
            OpenAICompatErrorKind::Http(std::sync::Arc::new(e))
        })?;

        debug!(
            provider = self.provider_name,
            choices = chat_response.choices().len(),
            "Received response"
        );

        Ok(chat_response)
    }
}
