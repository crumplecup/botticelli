//! Google Gemini API implementation.
//!
//! This module provides a client for the Google Gemini API with support for:
//! - Per-request model selection (different requests can use different models)
//! - Client pooling with lazy initialization (one client per model)
//! - Per-model rate limiting (each model has independent rate limits)
//! - Thread-safe concurrent access
//!
//! # Architecture
//!
//! The [`GeminiClient`] maintains a pool of model-specific clients, each wrapped in its own
//! rate limiter. When a request specifies a model (via `GenerateRequest.model`), the client
//! either retrieves the existing client for that model or creates a new one on-demand.
//!
//! This design enables:
//! - Multi-model narratives where different acts use different models
//! - Cost optimization by selecting appropriate models per task
//! - Independent rate limiting per model
//!
//! # Example
//!
//! ```no_run
//! use boticelli::{BoticelliDriver, GeminiClient, GenerateRequest, Message, Role, Input};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiClient::new()?;
//!
//! // Use default model (gemini-2.0-flash)
//! let request1 = GenerateRequest {
//!     messages: vec![Message {
//!         role: Role::User,
//!         content: vec![Input::Text("Hello".to_string())],
//!     }],
//!     model: None,
//!     ..Default::default()
//! };
//! let response1 = client.generate(&request1).await?;
//!
//! // Override to use a different model
//! let request2 = GenerateRequest {
//!     messages: vec![Message {
//!         role: Role::User,
//!         content: vec![Input::Text("Complex task".to_string())],
//!     }],
//!     model: Some("gemini-2.5-flash".to_string()),
//!     ..Default::default()
//! };
//! let response2 = client.generate(&request2).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

use gemini_rust::Gemini;

use crate::{
    BoticelliConfig, BoticelliDriver, BoticelliResult, GenerateRequest, GenerateResponse,
    GeminiTier, Input, Metadata, ModelMetadata, Output, RateLimiter, Role, Tier, Vision,
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
// ─── TIERED GEMINI ──────────────────────────────────────────────────────────────
//

/// Couples a Gemini API client with its rate limiting tier.
///
/// This type wraps a `Gemini` client and a tier (implementing `Tier`) together,
/// enabling the `RateLimiter` to own both the client and its rate limit configuration.
/// This ensures that clients cannot be accessed without going through rate limiting.
///
/// The struct implements `Tier` by delegating all methods to the inner tier,
/// allowing it to be used anywhere a `Tier` is expected (e.g., in `RateLimiter`).
///
/// # Type Parameters
///
/// * `T` - Any type implementing `Tier`, typically a concrete tier enum like `GeminiTier`
///
/// # Example
///
/// ```rust,ignore
/// use boticelli::{TieredGemini, GeminiTier};
/// use gemini_rust::Gemini;
///
/// let client = Gemini::with_model(api_key, "gemini-2.0-flash")?;
/// let tiered = TieredGemini {
///     client,
///     tier: GeminiTier::Free,
/// };
/// ```
#[derive(Clone)]
pub struct TieredGemini<T: Tier> {
    /// The Gemini API client
    pub client: Gemini,
    /// The tier configuration for rate limiting
    pub tier: T,
}

impl<T: Tier> Tier for TieredGemini<T> {
    fn rpm(&self) -> Option<u32> {
        self.tier.rpm()
    }

    fn tpm(&self) -> Option<u64> {
        self.tier.tpm()
    }

    fn rpd(&self) -> Option<u32> {
        self.tier.rpd()
    }

    fn max_concurrent(&self) -> Option<u32> {
        self.tier.max_concurrent()
    }

    fn daily_quota_usd(&self) -> Option<f64> {
        self.tier.daily_quota_usd()
    }

    fn cost_per_million_input_tokens(&self) -> Option<f64> {
        self.tier.cost_per_million_input_tokens()
    }

    fn cost_per_million_output_tokens(&self) -> Option<f64> {
        self.tier.cost_per_million_output_tokens()
    }

    fn name(&self) -> &str {
        self.tier.name()
    }
}

//
// ─── CLIENT ─────────────────────────────────────────────────────────────────────
//

/// Client for Google Gemini API with per-model client pooling.
///
/// This client maintains a cache of model-specific Gemini clients, each with its own
/// rate limiter. Clients are created lazily on first use for each model.
///
/// # Architecture
///
/// - **Client Pool**: `HashMap<String, RateLimiter<TieredGemini<GeminiTier>>>`
/// - **Lazy Creation**: Clients are created on first request for each model
/// - **Rate Limiting**: Each model has its own rate limiter with tier-specific limits
/// - **Thread-Safe**: Uses `Arc<Mutex<HashMap>>` for concurrent access
pub struct GeminiClient {
    /// Cache of model-specific clients with rate limiting
    clients: Arc<Mutex<HashMap<String, RateLimiter<TieredGemini<GeminiTier>>>>>,
    /// API key for creating new clients
    api_key: String,
    /// Default model name when req.model is None
    model_name: String,
    /// Default tier for rate limiting
    default_tier: GeminiTier,
}

impl std::fmt::Debug for GeminiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let client_count = self.clients.lock().unwrap().len();
        f.debug_struct("GeminiClient")
            .field("model_name", &self.model_name)
            .field("default_tier", &self.default_tier)
            .field("cached_clients", &client_count)
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
    /// The tier is converted to a `GeminiTier` by matching on the tier name.
    /// Supported tier names: "Free", "Pay-as-you-go". Defaults to Free for unknown names.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use boticelli::{GeminiClient, GeminiTier};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Using GeminiTier directly is preferred
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

        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| GeminiError::new(GeminiErrorKind::MissingApiKey))?;

        // Convert Box<dyn Tier> to GeminiTier by matching on tier name
        // This is a pragmatic approach to handle the architecture mismatch
        // between generic RateLimiter<T> and the Box<dyn Tier> API
        let default_tier = if let Some(tier) = tier {
            match tier.name() {
                "Free" => GeminiTier::Free,
                "Pay-as-you-go" => GeminiTier::PayAsYouGo,
                _ => GeminiTier::Free, // Default to Free for unknown tier names
            }
        } else {
            GeminiTier::Free // Default when no tier specified
        };

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            api_key,
            model_name: "gemini-2.0-flash".to_string(),
            default_tier,
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
        // Determine which model to use
        let model_name = req.model.as_ref().unwrap_or(&self.model_name);

        // Get or create rate-limited client for this model
        let rate_limited_client = {
            let mut clients = self.clients.lock().unwrap();
            clients
                .entry(model_name.clone())
                .or_insert_with(|| {
                    // Create new Gemini client for this model
                    let client = Gemini::with_model(&self.api_key, model_name.clone())
                        .expect("Failed to create Gemini client for model");

                    // Wrap client with tier
                    let tiered = TieredGemini {
                        client,
                        tier: self.default_tier,
                    };

                    // Wrap in rate limiter
                    RateLimiter::new(tiered)
                })
                .clone()
        };

        // Estimate tokens for rate limiting
        let estimated_tokens: u64 = req
            .messages
            .iter()
            .flat_map(|msg| &msg.content)
            .filter_map(Self::extract_text)
            .map(|text| Self::estimate_tokens(&text))
            .sum();

        // Add max_tokens if specified (output token estimate)
        let total_estimate = estimated_tokens + req.max_tokens.unwrap_or(1000) as u64;

        // Acquire rate limit permission
        let _guard = rate_limited_client.acquire(total_estimate).await;

        // Access the client through the rate limiter
        let client = &rate_limited_client.inner().client;

        // Start building the request
        let mut builder = client.generate_content();

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
        let response = builder
            .execute()
            .await
            .map_err(|e| GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string())))?;

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

    /// Returns the default model name used when `GenerateRequest.model` is None.
    ///
    /// Note: This returns the default model configured at client creation time.
    /// Individual requests may use different models by specifying `GenerateRequest.model`.
    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl Metadata for GeminiClient {
    /// Returns metadata for the default model.
    ///
    /// Note: This returns capabilities for the default model configured at client creation.
    /// Different Gemini models may have different capabilities and limits. When using
    /// per-request model selection via `GenerateRequest.model`, verify that the requested
    /// model supports the features you need.
    ///
    /// Current metadata reflects Gemini 2.0 Flash capabilities.
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
        &[
            "image/png",
            "image/jpeg",
            "image/webp",
            "image/heic",
            "image/heif",
        ]
    }

    fn max_image_size_bytes(&self) -> usize {
        20 * 1024 * 1024 // 20MB
    }
}
