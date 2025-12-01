//! Ollama LLM client implementation.

use ollama_rs::generation::completion::request::GenerationRequest as OllamaRequest;
use ollama_rs::Ollama;

use super::conversion::{messages_to_prompt, response_to_output};
use super::{OllamaError, OllamaErrorKind, OllamaResult};
use botticelli_core::{GenerateRequest, GenerateResponse};
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
    #[instrument(name = "ollama_client_new")]
    pub fn new(model_name: impl Into<String>) -> OllamaResult<Self> {
        Self::new_with_url(model_name, "http://localhost:11434")
    }

    /// Create a new Ollama client with custom server URL.
    #[instrument(name = "ollama_client_new_with_url")]
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

                    return Err(OllamaError::new(OllamaErrorKind::ModelNotFound(
                        self.model_name.clone(),
                    )));
                }

                info!("Ollama server and model validated");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "Failed to connect to Ollama server");
                Err(OllamaError::new(OllamaErrorKind::ServerNotRunning(
                    self.base_url.clone(),
                )))
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
                    .map_err(|e| {
                        OllamaError::new(OllamaErrorKind::ModelPullFailed(e.to_string()))
                    })?;

                info!("Model pulled successfully");
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl BotticelliDriver for OllamaClient {
    #[instrument(skip(self, request))]
    async fn generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, Box<dyn std::error::Error + Send + Sync>> {
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
            .map_err(|e| OllamaError::new(OllamaErrorKind::ApiError(e.to_string())))?;

        debug!(
            response_length = response.response.len(),
            "Received response from Ollama"
        );

        let output = response_to_output(response);
        Ok(GenerateResponse::builder()
            .outputs(vec![output])
            .build()
            .expect("Valid response"))
    }
}
