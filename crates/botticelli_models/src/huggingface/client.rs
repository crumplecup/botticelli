//! HuggingFace Inference API client.

use crate::huggingface::{conversion, HuggingFaceRequest};
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Output};
use botticelli_error::{BotticelliResult, HuggingFaceErrorKind, ModelsError, ModelsResult};
use botticelli_interface::{BotticelliDriver, StreamChunk, Streaming};
use botticelli_rate_limit::RateLimitConfig;
use futures_util::{stream::Stream, StreamExt};
use reqwest::Client;
use std::pin::Pin;
use tracing::{debug, instrument};

/// HuggingFace Inference API client.
#[derive(Debug, Clone)]
pub struct HuggingFaceClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    rate_limits: RateLimitConfig,
}

impl HuggingFaceClient {
    /// Creates a new HuggingFace client.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key is not set in the environment
    /// - The HTTP client cannot be initialized
    #[instrument(skip_all)]
    pub fn new(model: String) -> ModelsResult<Self> {
        let api_key = std::env::var("HUGGINGFACE_API_KEY").map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::InvalidConfiguration(format!(
                    "HUGGINGFACE_API_KEY not set: {}",
                    e
                )),
            ))
        })?;

        let client = Client::new();
        let base_url = "https://router.huggingface.co/v1/models".to_string();
        let rate_limits = RateLimitConfig::unlimited("huggingface");

        Ok(Self {
            client,
            api_key,
            base_url,
            model,
            rate_limits,
        })
    }

    /// Creates a new HuggingFace client with a specific API key.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be initialized.
    #[instrument(skip_all)]
    pub fn with_api_key(api_key: String, model: String) -> ModelsResult<Self> {
        let client = Client::new();
        let base_url = "https://api-inference.huggingface.co/models".to_string();
        let rate_limits = RateLimitConfig::unlimited("huggingface");

        Ok(Self {
            client,
            api_key,
            base_url,
            model,
            rate_limits,
        })
    }
}

#[async_trait]
impl BotticelliDriver for HuggingFaceClient {
    #[instrument(skip(self, req))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        // Convert to HuggingFace format
        let hf_request: HuggingFaceRequest = conversion::to_huggingface_request(req, &self.model)?;
        
        let url = format!("{}/{}", self.base_url, self.model);
        debug!(url = %url, "Sending HuggingFace API request");
        
        // Send request
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&hf_request)
            .send()
            .await
            .map_err(|e| {
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::Http(format!("Request failed: {}", e)),
                ))
            })?;
        
        // Check status
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ModelsError::new(
                botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::ApiError {
                        status,
                        message: error_text,
                    },
                ),
            ).into());
        }
        
        // Parse response
        let hf_response: crate::huggingface::HuggingFaceResponse = response.json().await.map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::ConversionError(format!("Failed to parse response: {}", e)),
            ))
        })?;
        
        // Convert back to Botticelli format
        conversion::from_huggingface_response(&hf_response).map_err(Into::into)
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
impl Streaming for HuggingFaceClient {
    #[instrument(skip(self, req))]
    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<Pin<Box<dyn Stream<Item = BotticelliResult<StreamChunk>> + Send>>> {
        // Convert to HuggingFace format
        let hf_request: HuggingFaceRequest = conversion::to_huggingface_request(req, &self.model)?;
        
        let url = format!("{}/{}", self.base_url, self.model);
        debug!(url = %url, "Sending streaming HuggingFace API request");
        
        // Send request
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&hf_request)
            .send()
            .await
            .map_err(|e| {
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::Http(format!("Request failed: {}", e)),
                ))
            })?;
        
        // Check status
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ModelsError::new(
                botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::ApiError {
                        status,
                        message: error_text,
                    },
                ),
            ).into());
        }
        
        // Create stream
        let stream = response.bytes_stream().map(|chunk_result| {
            let chunk_bytes = chunk_result.map_err(|e| {
                ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                    HuggingFaceErrorKind::Http(format!("Stream error: {}", e)),
                ))
            })?;
            
            // Parse chunk as JSON
            let chunk_str = String::from_utf8_lossy(&chunk_bytes);
            if chunk_str.trim().is_empty() {
                return StreamChunk::builder()
                    .content(Output::Text(String::new()))
                    .is_final(false)
                    .build()
                    .map_err(|e| {
                        ModelsError::new(botticelli_error::ModelsErrorKind::Builder(
                            e.to_string(),
                        )).into()
                    });
            }
            
            // Create StreamChunk
            StreamChunk::builder()
                .content(Output::Text(chunk_str.to_string()))
                .is_final(false)
                .build()
                .map_err(|e| {
                    ModelsError::new(botticelli_error::ModelsErrorKind::Builder(
                        e.to_string(),
                    )).into()
                })
        });
        
        Ok(Box::pin(stream))
    }
}
