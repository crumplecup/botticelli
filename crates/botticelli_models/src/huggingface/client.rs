//! HuggingFace Inference API client.

use botticelli_error::{HuggingFaceErrorKind, ModelsError, ModelsResult};
use reqwest::Client;
use tracing::instrument;

/// HuggingFace Inference API client.
#[derive(Debug, Clone)]
pub struct HuggingFaceClient {
    client: Client,
    api_key: String,
    base_url: String,
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
    pub fn new() -> ModelsResult<Self> {
        let api_key = std::env::var("HUGGINGFACE_API_KEY").map_err(|e| {
            ModelsError::new(botticelli_error::ModelsErrorKind::HuggingFace(
                HuggingFaceErrorKind::InvalidConfiguration(format!(
                    "HUGGINGFACE_API_KEY not set: {}",
                    e
                )),
            ))
        })?;

        let client = Client::new();
        let base_url = "https://api-inference.huggingface.co/models".to_string();

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    /// Creates a new HuggingFace client with a specific API key.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be initialized.
    #[instrument(skip_all)]
    pub fn with_api_key(api_key: String) -> ModelsResult<Self> {
        let client = Client::new();
        let base_url = "https://api-inference.huggingface.co/models".to_string();

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }
}

// BotticelliDriver implementation will be added in Phase 6
