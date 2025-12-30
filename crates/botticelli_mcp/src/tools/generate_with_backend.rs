use botticelli_core::{GenerateRequest, GenerateResponse};
use botticelli_error::{McpError, McpErrorKind, McpResult};
use botticelli_interface::BotticelliDriver;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, instrument};

/// Request to generate text using a specific LLM backend.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[serde(deny_unknown_fields)]
pub struct GenerateWithBackendRequest {
    /// The backend provider to use (e.g., "groq", "gemini", "anthropic")
    backend: String,
    
    /// The model to use (e.g., "llama-3.1-8b-instant")
    model: String,
    
    /// The generation request parameters
    #[serde(flatten)]
    request: GenerateRequest,
}

/// Response from generating text with an LLM backend.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct GenerateWithBackendResponse {
    /// The provider that handled the request
    provider: String,
    
    /// The model that generated the response
    model: String,
    
    /// The generation response
    #[serde(flatten)]
    response: GenerateResponse,
}

/// Factory for creating LLM backend drivers.
pub trait BackendFactory: Send + Sync {
    /// Create a driver for the specified backend and model.
    ///
    /// # Errors
    ///
    /// Returns error if backend is unsupported or configuration is invalid.
    fn create_driver(
        &self,
        backend: &str,
        model: &str,
    ) -> McpResult<Arc<dyn BotticelliDriver>>;
}

/// Tool for generating text using different LLM backends.
pub struct GenerateWithBackendTool {
    factory: Arc<dyn BackendFactory>,
}

impl GenerateWithBackendTool {
    /// Creates a new generate-with-backend tool.
    pub fn new(factory: Arc<dyn BackendFactory>) -> Self {
        Self { factory }
    }

    /// Execute the generation request.
    ///
    /// # Errors
    ///
    /// Returns error if backend creation or generation fails.
    #[instrument(skip(self, request), fields(backend = %request.backend, model = %request.model))]
    pub async fn execute(
        &self,
        request: GenerateWithBackendRequest,
    ) -> McpResult<GenerateWithBackendResponse> {
        debug!("Creating driver for backend");
        let driver = self.factory.create_driver(&request.backend, &request.model)?;

        debug!("Generating with backend");
        let response: GenerateResponse = driver
            .generate(&request.request)
            .await
            .map_err(|e| {
                error!(error = ?e, "Generation failed");
                McpError::new(McpErrorKind::ExecutionFailed(format!("Generation failed: {}", e)))
            })?;

        debug!("Generation successful");
        Ok(GenerateWithBackendResponse {
            provider: driver.provider_name().to_string(),
            model: driver.model_name().to_string(),
            response,
        })
    }
}

#[cfg(feature = "groq")]
mod groq_factory {
    use super::*;
    use botticelli_models::GroqDriver;

    /// Backend factory that creates Groq drivers.
    #[derive(Debug, Clone, Default)]
    pub struct GroqBackendFactory;

    impl BackendFactory for GroqBackendFactory {
        fn create_driver(
            &self,
            backend: &str,
            model: &str,
        ) -> McpResult<Arc<dyn BotticelliDriver>> {
            if backend != "groq" {
                return Err(McpError::new(McpErrorKind::ExecutionFailed(format!(
                    "Unsupported backend: {}",
                    backend
                ))));
            }

            let driver = GroqDriver::new(model.to_string()).map_err(|e| {
                McpError::new(McpErrorKind::ExecutionFailed(format!(
                    "Failed to create Groq driver: {}",
                    e
                )))
            })?;

            Ok(Arc::new(driver))
        }
    }
}

#[cfg(feature = "groq")]
pub use groq_factory::GroqBackendFactory;
