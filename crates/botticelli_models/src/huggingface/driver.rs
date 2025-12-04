//! HuggingFace Inference API driver using reqwest.

use crate::huggingface::{HuggingFaceResponse, conversions};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{BotticelliResult, HuggingFaceErrorKind, ModelsError, ModelsResult};
use botticelli_interface::{BotticelliDriver, StreamChunk, Streaming};
use botticelli_rate_limit::RateLimitConfig;
use futures_util::stream::Stream;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;
use tracing::{debug, error, instrument};

/// HuggingFace Inference API driver.
#[derive(Debug, Clone)]
pub struct HuggingFaceDriver {
    client: Client,
    api_token: String,
    model: String,
    base_url: String,
    rate_limits: RateLimitConfig,
}

impl HuggingFaceDriver {
    /// Creates a new HuggingFace driver.
    ///
    /// Reads API token from `HUGGINGFACE_API_TOKEN` environment variable.
    ///
    /// # Errors
    ///
    /// Returns error if API token is not set.
    #[instrument(skip_all, fields(model = %model))]
    pub fn new(model: String) -> ModelsResult<Self> {
        let api_token = std::env::var("HUGGINGFACE_API_TOKEN").map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::InvalidRequest(format!(
                    "HUGGINGFACE_API_TOKEN not set: {}",
                    e
                )),
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
        let client = Client::new();
        let base_url = "https://api-inference.huggingface.co/models".to_string();
        let rate_limits = RateLimitConfig::unlimited("huggingface");

        debug!(model = %model, "Created HuggingFace driver");

        Ok(Self {
            client,
            api_token,
            model,
            base_url,
            rate_limits,
        })
    }
}

#[async_trait]
impl BotticelliDriver for HuggingFaceDriver {
    #[instrument(skip(self, req), fields(model = %self.model))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        let hf_request = conversions::to_huggingface_request(req, &self.model)?;

        let url = format!("{}/{}", self.base_url, self.model);

        let mut body = json!({
            "inputs": hf_request.inputs(),
        });

        if let Some(params) = hf_request.parameters() {
            let mut params_json = json!({});

            if let Some(max_tokens) = params.max_new_tokens() {
                params_json["max_new_tokens"] = json!(max_tokens);
            }
            if let Some(temp) = params.temperature() {
                params_json["temperature"] = json!(temp);
            }

            body["parameters"] = params_json;
        }

        debug!(
            model = %self.model,
            url = %url,
            inputs_len = hf_request.inputs().len(),
            "Sending request to HuggingFace"
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, "HTTP request failed");
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::Api(format!("Request failed: {}", e)),
                ))
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!(status = %status, error = %error_text, "API error");

            return Err(
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::Api(format!("API error {}: {}", status, error_text)),
                ))
                .into(),
            );
        }

        let response_text = response.text().await.map_err(|e| {
            error!(error = ?e, "Failed to read response");
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::ResponseConversion(format!("Failed to read response: {}", e)),
            ))
        })?;

        debug!(response_len = response_text.len(), "Received response");

        let response_json: serde_json::Value =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(error = ?e, response = %response_text, "Failed to parse JSON");
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::ResponseConversion(format!(
                        "Failed to parse JSON: {}",
                        e
                    )),
                ))
            })?;

        let generated_text = if let Some(array) = response_json.as_array() {
            array
                .first()
                .and_then(|v| v.get("generated_text"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                        HuggingFaceErrorKind::ResponseConversion(
                            "Missing generated_text in array response".to_string(),
                        ),
                    ))
                })?
                .to_string()
        } else if let Some(text) = response_json.get("generated_text").and_then(|v| v.as_str()) {
            text.to_string()
        } else {
            return Err(
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::ResponseConversion(
                        "Missing generated_text in response".to_string(),
                    ),
                ))
                .into(),
            );
        };

        let hf_response = HuggingFaceResponse::builder()
            .generated_text(generated_text)
            .build()
            .map_err(|e| {
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::ResponseConversion(format!(
                        "Failed to build response: {}",
                        e
                    )),
                ))
            })?;

        conversions::from_huggingface_response(&hf_response).map_err(Into::into)
    }

    fn provider_name(&self) -> &'static str {
        "huggingface"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }
}

#[async_trait]
impl Streaming for HuggingFaceDriver {
    #[instrument(skip(self, req), fields(model = %self.model))]
    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<Pin<Box<dyn Stream<Item = BotticelliResult<StreamChunk>> + Send>>> {
        debug!("HuggingFace streaming not yet implemented, falling back to non-streaming");

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
