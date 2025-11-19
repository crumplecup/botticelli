//! Configuration for local inference server connection

use crate::{ServerError, ServerErrorKind};

/// Configuration for local inference server connection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerConfig {
    /// Base URL of the server (e.g., "http://localhost:8080")
    pub base_url: String,
    /// Model identifier to use for inference
    pub model: String,
    /// Optional API key (mistral.rs doesn't require one by default)
    pub api_key: Option<String>,
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
        }
    }

    /// Create config from environment variables
    ///
    /// Reads:
    /// - `INFERENCE_SERVER_BASE_URL` (default: "http://localhost:8080")
    /// - `INFERENCE_SERVER_MODEL` (required)
    /// - `INFERENCE_SERVER_API_KEY` (optional)
    pub fn from_env() -> Result<Self, ServerError> {
        let base_url = std::env::var("INFERENCE_SERVER_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let model = std::env::var("INFERENCE_SERVER_MODEL").map_err(|_| {
            ServerError::new(ServerErrorKind::Configuration(
                "INFERENCE_SERVER_MODEL not set".into(),
            ))
        })?;
        let api_key = std::env::var("INFERENCE_SERVER_API_KEY").ok();

        Ok(Self {
            base_url,
            model,
            api_key,
        })
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}
