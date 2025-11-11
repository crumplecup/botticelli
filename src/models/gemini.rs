//! Google Gemini API implementation.

use async_trait::async_trait;
use std::env;

use gemini_rust::Gemini;

use crate::{
    BoticelliConfig, BoticelliDriver, BoticelliResult, GenerateRequest, GenerateResponse, Input,
    Metadata, ModelMetadata, Output, RateLimiter, Role, Tier, Vision,
};

//
// ─── ERROR TYPES ────────────────────────────────────────────────────────────────
//

/// Gemini-specific error conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeminiErrorKind {
    /// API key not found in environment
    MissingApiKey,
    /// Failed to create Gemini client
    ClientCreation(String),
    /// API request failed
    ApiRequest(String),
    /// Multimodal inputs not yet supported
    MultimodalNotSupported,
    /// URL media sources not yet supported
    UrlMediaNotSupported,
    /// Base64 decoding failed
    Base64Decode(String),
}

impl std::fmt::Display for GeminiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeminiErrorKind::MissingApiKey => {
                write!(f, "GEMINI_API_KEY environment variable not set")
            }
            GeminiErrorKind::ClientCreation(msg) => {
                write!(f, "Failed to create Gemini client: {}", msg)
            }
            GeminiErrorKind::ApiRequest(msg) => write!(f, "Gemini API request failed: {}", msg),
            GeminiErrorKind::MultimodalNotSupported => write!(
                f,
                "Multimodal inputs not yet supported in simple Gemini wrapper"
            ),
            GeminiErrorKind::UrlMediaNotSupported => {
                write!(f, "URL media sources not yet supported for Gemini")
            }
            GeminiErrorKind::Base64Decode(msg) => write!(f, "Base64 decode error: {}", msg),
        }
    }
}

/// Gemini error with source location tracking.
#[derive(Debug, Clone)]
pub struct GeminiError {
    pub kind: GeminiErrorKind,
    pub line: u32,
    pub file: &'static str,
}

impl GeminiError {
    /// Create a new GeminiError with the given kind at the current location.
    #[track_caller]
    pub fn new(kind: GeminiErrorKind) -> Self {
        let location = std::panic::Location::caller();
        Self {
            kind,
            line: location.line(),
            file: location.file(),
        }
    }
}

impl std::fmt::Display for GeminiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gemini Error: {} at line {} in {}",
            self.kind, self.line, self.file
        )
    }
}

impl std::error::Error for GeminiError {}

/// Result type for Gemini operations.
pub type GeminiResult<T> = Result<T, GeminiError>;

//
// ─── CLIENT ─────────────────────────────────────────────────────────────────────
//

/// Client for Google Gemini API.
pub struct GeminiClient {
    client: Gemini,
    model_name: String,
    rate_limiter: Option<RateLimiter>,
}

impl std::fmt::Debug for GeminiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiClient")
            .field("model_name", &self.model_name)
            .field(
                "rate_limiter",
                &if self.rate_limiter.is_some() {
                    "Some(RateLimiter)"
                } else {
                    "None"
                },
            )
            .finish_non_exhaustive()
    }
}

impl GeminiClient {
    /// Create a new Gemini client without rate limiting.
    ///
    /// Reads the API key from the `GEMINI_API_KEY` environment variable.
    /// Defaults to using Gemini 2.0 Flash model.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use boticelli::GeminiClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GeminiClient::new()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> BoticelliResult<Self> {
        Self::new_with_tier(None)
    }

    /// Create a new Gemini client with rate limiting.
    ///
    /// Reads the API key from the `GEMINI_API_KEY` environment variable.
    /// Applies rate limiting according to the provided tier.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use boticelli::{GeminiClient, GeminiTier};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GeminiClient::new_with_tier(Some(Box::new(GeminiTier::Free)))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_tier(tier: Option<Box<dyn Tier>>) -> BoticelliResult<Self> {
        Self::new_internal(tier).map_err(Into::into)
    }

    /// Create a new Gemini client with rate limiting from configuration.
    ///
    /// Loads tier configuration from boticelli.toml and applies rate limiting.
    /// Falls back to no rate limiting if configuration cannot be loaded.
    ///
    /// # Arguments
    ///
    /// * `tier_name` - Optional tier name (uses provider default if None)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use boticelli::GeminiClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Use default tier from config
    /// let client = GeminiClient::new_with_config(None)?;
    ///
    /// // Use specific tier
    /// let client = GeminiClient::new_with_config(Some("payasyougo"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_config(tier_name: Option<&str>) -> BoticelliResult<Self> {
        let tier = BoticelliConfig::load()
            .ok()
            .and_then(|config| config.get_tier("gemini", tier_name))
            .map(|tier_config| Box::new(tier_config) as Box<dyn Tier>);

        Self::new_with_tier(tier)
    }

    /// Internal constructor that returns Gemini-specific errors.
    fn new_internal(tier: Option<Box<dyn Tier>>) -> GeminiResult<Self> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        let api_key =
            env::var("GEMINI_API_KEY").map_err(|_| GeminiError::new(GeminiErrorKind::MissingApiKey))?;

        let client = Gemini::new(api_key).map_err(|e| {
            GeminiError::new(GeminiErrorKind::ClientCreation(e.to_string()))
        })?;

        // Create rate limiter if tier provided
        let rate_limiter = tier.map(RateLimiter::new);

        Ok(Self {
            client,
            model_name: "gemini-2.0-flash".to_string(),
            rate_limiter,
        })
    }

    /// Extract text content from an input
    fn extract_text(input: &Input) -> Option<String> {
        match input {
            Input::Text(text) => Some(text.clone()),
            _ => None,
        }
    }

    /// Check if input contains non-text media
    fn has_media(inputs: &[Input]) -> bool {
        inputs.iter().any(|i| !matches!(i, Input::Text(_)))
    }

    /// Estimate token count from text (rough approximation: chars / 4).
    ///
    /// This is a conservative estimate. Actual token count may be lower.
    fn estimate_tokens(text: &str) -> u64 {
        (text.len() / 4).max(1) as u64
    }

    /// Internal generate method that returns Gemini-specific errors.
    async fn generate_internal(&self, req: &GenerateRequest) -> GeminiResult<GenerateResponse> {
        // Acquire rate limit permission if rate limiting is enabled
        let _guard = if let Some(limiter) = &self.rate_limiter {
            // Estimate tokens for all input messages
            let estimated_tokens: u64 = req
                .messages
                .iter()
                .flat_map(|msg| &msg.content)
                .filter_map(Self::extract_text)
                .map(|text| Self::estimate_tokens(&text))
                .sum();

            // Add max_tokens if specified (output token estimate)
            let total_estimate = estimated_tokens + req.max_tokens.unwrap_or(1000) as u64;

            Some(limiter.acquire(total_estimate).await)
        } else {
            None
        };

        // Start building the request
        let mut builder = self.client.generate_content();

        // Process messages in order
        let mut system_prompt = None;

        for msg in &req.messages {
            match msg.role {
                Role::System => {
                    // Gemini uses a separate system prompt
                    if let Some(text) = msg.content.iter().find_map(Self::extract_text) {
                        system_prompt = Some(text);
                    }
                }
                Role::User => {
                    // Add user message(s)
                    for input in &msg.content {
                        if let Some(text) = Self::extract_text(input) {
                            builder = builder.with_user_message(&text);
                        }
                    }

                    // Note: gemini-rust's simple API doesn't directly support
                    // multimodal inputs through the builder pattern.
                    // For full multimodal support, we'd need to use the lower-level API.
                    if Self::has_media(&msg.content) {
                        return Err(GeminiError::new(GeminiErrorKind::MultimodalNotSupported));
                    }
                }
                Role::Assistant => {
                    // Add model/assistant message
                    if let Some(text) = msg.content.iter().find_map(Self::extract_text) {
                        builder = builder.with_model_message(&text);
                    }
                }
            }
        }

        // Add system prompt if present
        if let Some(prompt) = system_prompt {
            builder = builder.with_system_prompt(&prompt);
        }

        // Apply optional parameters
        if let Some(temp) = req.temperature {
            builder = builder.with_temperature(temp);
        }

        if let Some(max_tokens) = req.max_tokens {
            builder = builder.with_max_output_tokens(max_tokens as i32);
        }

        // Execute the request
        let response = builder.execute().await.map_err(|e| {
            GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string()))
        })?;

        // Extract text from response
        let text = response.text();

        Ok(GenerateResponse {
            outputs: vec![Output::Text(text)],
        })
    }
}

#[async_trait]
impl BoticelliDriver for GeminiClient {
    async fn generate(&self, req: &GenerateRequest) -> BoticelliResult<GenerateResponse> {
        self.generate_internal(req).await.map_err(Into::into)
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl Metadata for GeminiClient {
    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            provider: "gemini",
            model: self.model_name.clone(),
            max_input_tokens: 1_048_576, // Gemini 2.0 Flash supports up to 1M tokens
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_vision: true,
            supports_audio: true,
            supports_video: true,
            supports_documents: true,
            supports_tool_use: true,
            supports_json_mode: true,
            supports_embeddings: true,
            supports_batch: false,
        }
    }
}

impl Vision for GeminiClient {
    fn max_images_per_request(&self) -> usize {
        16 // Gemini supports multiple images
    }

    fn supported_image_formats(&self) -> &[&'static str] {
        &["image/png", "image/jpeg", "image/webp", "image/heic", "image/heif"]
    }

    fn max_image_size_bytes(&self) -> usize {
        20 * 1024 * 1024 // 20MB
    }
}
