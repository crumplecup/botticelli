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
//! use botticelli_models::GeminiClient;
//! use botticelli_core::{GenerateRequest, Message, Role, Input};
//! use botticelli_interface::BotticelliDriver;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiClient::new()?;
//!
//! // Use default model (gemini-2.0-flash-lite)
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
use tracing::instrument;

use gemini_rust::{Gemini, client::Model};

use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output, Role};
use botticelli_error::{BotticelliError, BotticelliResult, GeminiError, GeminiErrorKind};
use botticelli_interface::{
    BotticelliDriver, FinishReason, Metadata, ModelMetadata, StreamChunk, Streaming, Vision,
};
use botticelli_rate_limit::{BotticelliConfig, RateLimiter, Tier, TierConfig};

use super::GeminiResult;

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
/// use botticelli_models::gemini::{TieredGemini, GeminiTier};
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

impl<T: Tier + std::fmt::Debug> std::fmt::Debug for TieredGemini<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TieredGemini")
            .field("tier", &self.tier)
            .finish_non_exhaustive()
    }
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
/// - **Client Pool**: `HashMap<String, RateLimiter<TieredGemini<TierConfig>>>`
/// - **Lazy Creation**: Clients are created on first request for each model
/// - **Model-Specific Rate Limiting**: Each model gets its own rate limits from config
/// - **Thread-Safe**: Uses `Arc<Mutex<HashMap>>` for concurrent access
pub struct GeminiClient {
    /// Cache of model-specific REST API clients with rate limiting
    clients: Arc<Mutex<HashMap<String, RateLimiter<TieredGemini<TierConfig>>>>>,
    /// WebSocket Live API client (for live models)
    live_client: Option<super::live_client::GeminiLiveClient>,
    /// API key for creating new clients
    api_key: String,
    /// Default model name when req.model is None
    model_name: String,
    /// Base tier configuration (tier-level defaults + model-specific overrides)
    base_tier: TierConfig,
    /// Retry configuration
    no_retry: bool,
    max_retries: Option<usize>,
    retry_backoff_ms: Option<u64>,
}

impl std::fmt::Debug for GeminiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let client_count = self.clients.lock().unwrap().len();
        f.debug_struct("GeminiClient")
            .field("model_name", &self.model_name)
            .field("base_tier", &self.base_tier.name())
            .field("cached_clients", &client_count)
            .finish_non_exhaustive()
    }
}

impl GeminiClient {
    /// Convert a model name string to a gemini-rust Model enum variant.
    ///
    /// Maps common model name strings to their corresponding Model enum variants.
    /// Uses Model::Custom for unrecognized model names, automatically adding the
    /// "models/" prefix required by the Gemini API.
    ///
    /// # Examples
    ///
    /// - "gemini-2.5-flash" → Model::Gemini25Flash
    /// - "gemini-2.0-flash" → Model::Custom("models/gemini-2.0-flash")
    /// - "models/gemini-2.0-flash" → Model::Custom("models/gemini-2.0-flash") (preserved)
    fn model_name_to_enum(name: &str) -> Model {
        match name {
            "gemini-2.5-flash" => Model::Gemini25Flash,
            "gemini-2.5-flash-lite" => Model::Gemini25FlashLite,
            "gemini-2.5-pro" => Model::Gemini25Pro,
            "text-embedding-004" => Model::TextEmbedding004,
            // For other model names, use Custom variant with "models/" prefix
            other => {
                // Add "models/" prefix if not already present
                if other.starts_with("models/") {
                    Model::Custom(other.to_string())
                } else {
                    Model::Custom(format!("models/{}", other))
                }
            }
        }
    }

    /// Create a new Gemini client without rate limiting.
    ///
    /// Reads the API key from the `GEMINI_API_KEY` environment variable.
    /// Defaults to using Gemini 2.0 Flash Lite model for development.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::GeminiClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = GeminiClient::new()?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_client_new")]
    pub fn new() -> BotticelliResult<Self> {
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
    /// use botticelli_models::GeminiClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create client with default tier (Free)
    /// let client = GeminiClient::new_with_tier(None)?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_client_new_with_tier", skip(tier))]
    pub fn new_with_tier(tier: Option<Box<dyn Tier>>) -> BotticelliResult<Self> {
        Self::new_internal(tier).map_err(Into::into)
    }

    /// Create a new Gemini client with rate limiting and retry configuration.
    ///
    /// # Arguments
    ///
    /// * `tier` - Optional tier for rate limiting
    /// * `no_retry` - Disable automatic retry
    /// * `max_retries` - Override maximum retry attempts
    /// * `retry_backoff_ms` - Override initial backoff delay
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::GeminiClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create client with retry disabled
    /// let client = GeminiClient::new_with_retry(None, true, None, None)?;
    ///
    /// // Create client with custom retry limits
    /// let client = GeminiClient::new_with_retry(None, false, Some(3), Some(1000))?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_client_new_with_retry", skip(tier))]
    pub fn new_with_retry(
        tier: Option<Box<dyn Tier>>,
        no_retry: bool,
        max_retries: Option<usize>,
        retry_backoff_ms: Option<u64>,
    ) -> BotticelliResult<Self> {
        Self::new_internal_with_retry(tier, no_retry, max_retries, retry_backoff_ms)
            .map_err(Into::into)
    }

    /// Create a new Gemini client with rate limiting from configuration.
    ///
    /// Loads tier configuration from botticelli.toml and applies rate limiting,
    /// including model-specific rate limit overrides.
    ///
    /// # Arguments
    ///
    /// * `tier_name` - Optional tier name (uses provider default if None)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::GeminiClient;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Use default tier from config (includes model-specific limits)
    /// let client = GeminiClient::new_with_config(None)?;
    ///
    /// // Use specific tier
    /// let client = GeminiClient::new_with_config(Some("payasyougo"))?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(name = "gemini_client_new_with_config")]
    pub fn new_with_config(tier_name: Option<&str>) -> BotticelliResult<Self> {
        let tier_config = BotticelliConfig::load()
            .ok()
            .and_then(|config| config.get_tier("gemini", tier_name));

        Self::new_with_tier_config(tier_config)
    }

    /// Create a new Gemini client with a TierConfig (preserves model-specific overrides).
    fn new_with_tier_config(tier_config: Option<TierConfig>) -> BotticelliResult<Self> {
        // Load .env file if present

        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| BotticelliError::from(GeminiError::new(GeminiErrorKind::MissingApiKey)))?;

        let base_tier = tier_config.unwrap_or_else(|| {
            // Default tier configuration (Free tier, gemini-2.0-flash-lite for development)
            TierConfig {
                name: "Free".to_string(),
                rpm: Some(10),
                tpm: Some(250_000),
                rpd: Some(250),
                max_concurrent: Some(1),
                daily_quota_usd: None,
                cost_per_million_input_tokens: Some(0.0),
                cost_per_million_output_tokens: Some(0.0),
                models: HashMap::new(),
            }
        });

        // Create Live API client with rate limiting from tier config
        let live_client = {
            let rpm = base_tier.rpm();
            super::live_client::GeminiLiveClient::new_with_rate_limit(rpm).ok()
        };

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            live_client,
            api_key,
            model_name: "gemini-2.0-flash-lite".to_string(),
            base_tier,
            no_retry: false,
            max_retries: None,
            retry_backoff_ms: None,
        })
    }

    /// Internal constructor that returns Gemini-specific errors.
    fn new_internal(tier: Option<Box<dyn Tier>>) -> GeminiResult<Self> {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| GeminiError::new(GeminiErrorKind::MissingApiKey))?;

        // Convert Box<dyn Tier> to TierConfig
        // We create a TierConfig from the Tier trait methods. This works for all
        // Tier implementations. If called from new_with_config(), the tier will
        // already be a TierConfig with model-specific overrides, and those will
        // be preserved. For other callers (passing GeminiTier or custom tiers),
        // we create a basic TierConfig without model-specific overrides.
        let base_tier = if let Some(tier) = tier {
            TierConfig {
                name: tier.name().to_string(),
                rpm: tier.rpm(),
                tpm: tier.tpm(),
                rpd: tier.rpd(),
                max_concurrent: tier.max_concurrent(),
                daily_quota_usd: tier.daily_quota_usd(),
                cost_per_million_input_tokens: tier.cost_per_million_input_tokens(),
                cost_per_million_output_tokens: tier.cost_per_million_output_tokens(),
                models: HashMap::new(), // Will be empty for non-TierConfig tiers
            }
        } else {
            // Default tier configuration (Free tier, gemini-2.0-flash-lite for development)
            TierConfig {
                name: "Free".to_string(),
                rpm: Some(10),
                tpm: Some(250_000),
                rpd: Some(250),
                max_concurrent: Some(1),
                daily_quota_usd: None,
                cost_per_million_input_tokens: Some(0.0),
                cost_per_million_output_tokens: Some(0.0),
                models: HashMap::new(),
            }
        };

        // Create Live API client with rate limiting from tier config
        let live_client = {
            let rpm = base_tier.rpm();
            super::live_client::GeminiLiveClient::new_with_rate_limit(rpm).ok()
        };

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            live_client,
            api_key,
            model_name: "gemini-2.0-flash-lite".to_string(),
            base_tier,
            no_retry: false,
            max_retries: None,
            retry_backoff_ms: None,
        })
    }

    /// Internal constructor with retry configuration.
    fn new_internal_with_retry(
        tier: Option<Box<dyn Tier>>,
        no_retry: bool,
        max_retries: Option<usize>,
        retry_backoff_ms: Option<u64>,
    ) -> GeminiResult<Self> {
        let mut client = Self::new_internal(tier)?;
        client.no_retry = no_retry;
        client.max_retries = max_retries;
        client.retry_backoff_ms = retry_backoff_ms;
        Ok(client)
    }

    /// Check if a model name indicates a Live API model (requires WebSocket).
    ///
    /// Live API models include:
    /// - Models with "-live" in the name (e.g., "gemini-2.0-flash-live")
    /// - Experimental models with "bidiGenerateContent" support (e.g., "gemini-2.0-flash-exp")
    fn is_live_model(model_name: &str) -> bool {
        model_name.contains("-live") || model_name.contains("-exp")
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

    /// Generate response using Live API (WebSocket).
    ///
    /// This method is used for models that require WebSocket connections (live models, exp models).
    async fn generate_via_live_api(
        &self,
        req: &GenerateRequest,
        model_name: &str,
    ) -> GeminiResult<GenerateResponse> {
        use tokio_retry2::{Retry, RetryError, strategy::ExponentialBackoff, strategy::jitter};
        use tracing::{info, warn};

        // Ensure we have a Live API client
        let live_client = self.live_client.as_ref().ok_or_else(|| {
            GeminiError::new(GeminiErrorKind::ClientCreation(
                "Live API client not available".to_string(),
            ))
        })?;

        // Build generation config from request
        let config = super::live_protocol::GenerationConfig {
            max_output_tokens: req.max_tokens().map(|t| t as i32),
            temperature: req.temperature().map(|t| t as f64),
            ..Default::default()
        };

        // Check if retry is disabled
        if self.no_retry {
            // No retry - attempt once
            let mut session = live_client.connect_with_config(model_name, config).await?;

            let combined_text = self.combine_messages(req);
            let response_text = session.send_text(&combined_text).await?;
            let _ = session.close().await;

            return Ok(GenerateResponse {
                outputs: vec![Output::Text(response_text)],
            });
        }

        // Determine retry strategy from first error or use defaults
        let model = model_name.to_string();
        let gen_config = config.clone();
        let client = live_client.clone();

        // Try connection once to get error-specific strategy
        let first_result = client.connect_with_config(&model, gen_config.clone()).await;

        let (initial_ms, max_retries, max_delay_secs) = match &first_result {
            Ok(_) => {
                // Success on first try
                let mut session = first_result.unwrap();
                let combined_text = self.combine_messages(req);
                let response_text = session.send_text(&combined_text).await?;
                let _ = session.close().await;

                return Ok(GenerateResponse {
                    outputs: vec![Output::Text(response_text)],
                });
            }
            Err(e) => {
                if !e.kind.is_retryable() {
                    warn!(error = %e, "Permanent Live API error, failing immediately");
                    return Err(e.clone());
                }

                // Get error-specific strategy
                let (mut init_ms, mut retries, delay_secs) = e.kind.retry_strategy_params();

                // Apply CLI overrides
                if let Some(override_backoff) = self.retry_backoff_ms {
                    init_ms = override_backoff;
                }
                if let Some(override_retries) = self.max_retries {
                    retries = override_retries;
                }

                info!(
                    error = %e,
                    model = model,
                    initial_backoff_ms = init_ms,
                    max_retries = retries,
                    max_delay_secs = delay_secs,
                    "Live API connection failed, will retry with configured strategy"
                );

                (init_ms, retries, delay_secs)
            }
        };

        // Configure retry strategy
        let retry_strategy = ExponentialBackoff::from_millis(initial_ms)
            .factor(2)
            .max_delay(std::time::Duration::from_secs(max_delay_secs))
            .map(jitter)
            .take(max_retries);

        // Retry connection with backoff
        let mut session = Retry::spawn(retry_strategy, || {
            let m = model.clone();
            let c = gen_config.clone();
            let cli = client.clone();
            async move {
                match cli.connect_with_config(&m, c).await {
                    Ok(session) => Ok(session),
                    Err(e) => {
                        if e.kind.is_retryable() {
                            warn!(error = %e, "Live API connection failed, will retry");
                            Err(RetryError::Transient {
                                err: e,
                                retry_after: None,
                            })
                        } else {
                            warn!(error = %e, "Permanent Live API error, failing immediately");
                            Err(RetryError::Permanent(e))
                        }
                    }
                }
            }
        })
        .await?;

        // Combine all user messages into a single text
        let combined_text = self.combine_messages(req);

        // Send message and collect complete response
        let response_text = session.send_text(&combined_text).await?;

        // Close session
        let _ = session.close().await; // Ignore close errors

        // Return response in GenerateResponse format
        Ok(GenerateResponse {
            outputs: vec![Output::Text(response_text)],
        })
    }

    /// Helper to combine all message content into a single text string.
    fn combine_messages(&self, req: &GenerateRequest) -> String {
        let mut combined_text = String::new();
        for msg in req.messages() {
            for input in &msg.content {
                if let Some(text) = Self::extract_text(input) {
                    combined_text.push_str(&text);
                    combined_text.push('\n');
                }
            }
        }
        combined_text
    }

    /// Internal generate method that returns Gemini-specific errors.
    async fn generate_internal(&self, req: &GenerateRequest) -> GeminiResult<GenerateResponse> {
        // Determine which model to use
        let model_name = req.model().as_ref().unwrap_or(&self.model_name);

        // Check if this is a live model (requires WebSocket Live API)
        if Self::is_live_model(model_name) {
            return self.generate_via_live_api(req, model_name).await;
        }

        // Get or create rate-limited client for this model (REST API)
        let rate_limited_client = {
            let mut clients = self.clients.lock().unwrap();
            clients
                .entry(model_name.clone())
                .or_insert_with(|| {
                    // Convert model name string to Model enum
                    let model_enum = Self::model_name_to_enum(model_name);

                    // Create new Gemini client for this model
                    let client = Gemini::with_model(&self.api_key, model_enum)
                        .expect("Failed to create Gemini client for model");

                    // Get model-specific tier configuration
                    // This applies model-specific overrides if they exist in the config
                    let model_tier = self.base_tier.for_model(model_name);

                    // Wrap client with model-specific tier
                    let tiered = TieredGemini {
                        client,
                        tier: model_tier,
                    };

                    // Wrap in rate limiter with retry configuration
                    RateLimiter::new_with_retry(
                        tiered,
                        self.no_retry,
                        self.max_retries,
                        self.retry_backoff_ms,
                    )
                })
                .clone()
        };

        // Estimate tokens for rate limiting
        let estimated_tokens: u64 = req
            .messages()
            .iter()
            .flat_map(|msg| &msg.content)
            .filter_map(Self::extract_text)
            .map(|text| Self::estimate_tokens(&text))
            .sum();

        // Add max_tokens if specified (output token estimate)
        let total_estimate = estimated_tokens + req.max_tokens().unwrap_or(1000) as u64;

        // Clone data needed in the closure
        let messages = req.messages().clone();
        let temperature = req.temperature();
        let max_tokens = req.max_tokens();

        // Execute with rate limiting and automatic retry
        let response = rate_limited_client
            .execute(total_estimate, || async {
                // Access the client through the rate limiter
                let client = &rate_limited_client.inner().client;

                // Start building the request
                let mut builder = client.generate_content();

                // Process messages in order
                let mut system_prompt = None;

                for msg in &messages {
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
                            if Self::has_media(&msg.content) {
                                return Err(GeminiError::new(
                                    GeminiErrorKind::MultimodalNotSupported,
                                ));
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
                if let Some(temp) = temperature {
                    builder = builder.with_temperature(*temp);
                }

                if let Some(max_tok) = max_tokens {
                    builder = builder.with_max_output_tokens(*max_tok as i32);
                }

                // Execute the request and parse errors
                builder.execute().await.map_err(Self::parse_gemini_error)
            })
            .await?;

        // Extract text from response
        let text = response.text();

        Ok(GenerateResponse {
            outputs: vec![Output::Text(text)],
        })
    }

    /// Parse gemini-rust errors to extract HTTP status codes.
    ///
    /// Converts generic API error strings into structured GeminiError
    /// with HTTP status codes when available.
    fn parse_gemini_error(err: impl std::fmt::Display) -> GeminiError {
        let err_msg = err.to_string();

        // Try to extract HTTP status code from error message
        // Example: "bad response from server; code 503; description: ..."
        if let Some(status_code) = Self::extract_status_code(&err_msg) {
            GeminiError::new(GeminiErrorKind::HttpError {
                status_code,
                message: err_msg,
            })
        } else {
            GeminiError::new(GeminiErrorKind::ApiRequest(err_msg))
        }
    }

    /// Extract HTTP status code from error message string.
    ///
    /// Parses strings like "bad response from server; code 503; description: ..."
    /// and extracts the numeric status code.
    fn extract_status_code(error_msg: &str) -> Option<u16> {
        if let Some(code_start) = error_msg.find("code ") {
            let code_str = &error_msg[code_start + 5..];
            if let Some(end) = code_str.find(|c: char| !c.is_numeric()) {
                return code_str[..end].parse().ok();
            }
        }
        None
    }
}

#[async_trait]
impl BotticelliDriver for GeminiClient {
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
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

    fn rate_limits(&self) -> &botticelli_rate_limit::RateLimitConfig {
        // TODO: This creates a temporary on each call. Consider caching in the struct.
        Box::leak(Box::new(botticelli_rate_limit::RateLimitConfig::from_tier(&self.base_tier)))
    }
}

impl GeminiClient {
    /// Generate streaming response using Live API (WebSocket).
    ///
    /// This method is used for models that require WebSocket connections (live models, exp models).
    async fn generate_stream_via_live_api(
        &self,
        req: &GenerateRequest,
        model_name: &str,
    ) -> BotticelliResult<
        std::pin::Pin<
            Box<dyn futures_util::stream::Stream<Item = BotticelliResult<StreamChunk>> + Send>,
        >,
    > {
        use futures_util::stream::StreamExt;

        // Ensure we have a Live API client
        let live_client = self.live_client.as_ref().ok_or_else(|| {
            BotticelliError::from(GeminiError::new(GeminiErrorKind::ClientCreation(
                "Live API client not available".to_string(),
            )))
        })?;

        // Build generation config from request
        let config = super::live_protocol::GenerationConfig {
            max_output_tokens: req.max_tokens().map(|t| t as i32),
            temperature: req.temperature().map(|t| t as f64),
            ..Default::default()
        };

        // Connect to Live API
        let session = live_client
            .connect_with_config(model_name, config)
            .await
            .map_err(BotticelliError::from)?;

        // Combine all user messages into a single text
        let mut combined_text = String::new();
        for msg in req.messages() {
            for input in &msg.content {
                if let Some(text) = Self::extract_text(input) {
                    combined_text.push_str(&text);
                    combined_text.push('\n');
                }
            }
        }

        // Get stream from Live API (consumes session, stream owns it)
        let live_stream = session
            .send_text_stream(&combined_text)
            .await
            .map_err(BotticelliError::from)?;

        // Convert Live API stream to BotticelliResult stream
        let converted_stream = live_stream.map(|result| result.map_err(BotticelliError::from));

        Ok(Box::pin(converted_stream))
    }
}

#[async_trait]
impl Streaming for GeminiClient {
    async fn generate_stream(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<
        std::pin::Pin<
            Box<dyn futures_util::stream::Stream<Item = BotticelliResult<StreamChunk>> + Send>,
        >,
    > {
        use futures_util::{StreamExt, TryStreamExt};

        // Determine which model to use
        let model_name = req.model().as_ref().unwrap_or(&self.model_name);

        // Check if this is a live model (requires WebSocket Live API)
        if Self::is_live_model(model_name) {
            return self.generate_stream_via_live_api(req, model_name).await;
        }

        // Get or create rate-limited client for this model (REST API)
        let rate_limited_client = {
            let mut clients = self.clients.lock().unwrap();
            clients
                .entry(model_name.clone())
                .or_insert_with(|| {
                    let model_enum = Self::model_name_to_enum(model_name);
                    let client = Gemini::with_model(&self.api_key, model_enum)
                        .expect("Failed to create Gemini client for model");
                    let model_tier = self.base_tier.for_model(model_name);
                    let tiered = TieredGemini {
                        client,
                        tier: model_tier,
                    };
                    RateLimiter::new(tiered)
                })
                .clone()
        };

        // Estimate tokens for rate limiting
        let estimated_tokens: u64 = req
            .messages()
            .iter()
            .flat_map(|msg| &msg.content)
            .filter_map(Self::extract_text)
            .map(|text| Self::estimate_tokens(&text))
            .sum();

        let total_estimate = estimated_tokens + req.max_tokens().unwrap_or(1000) as u64;

        // Acquire rate limit permission (counts stream as single request)
        let _guard = rate_limited_client.acquire(total_estimate).await;

        // Access the client through the rate limiter
        let client = &rate_limited_client.inner().client;

        // Build request using builder API (same as generate_internal)
        let mut builder = client.generate_content();
        let mut system_prompt = None;

        for msg in req.messages() {
            match msg.role {
                Role::System => {
                    if let Some(text) = msg.content.iter().find_map(Self::extract_text) {
                        system_prompt = Some(text);
                    }
                }
                Role::User => {
                    for input in &msg.content {
                        if let Some(text) = Self::extract_text(input) {
                            builder = builder.with_user_message(&text);
                        }
                    }
                    if Self::has_media(&msg.content) {
                        return Err(
                            GeminiError::new(GeminiErrorKind::MultimodalNotSupported).into()
                        );
                    }
                }
                Role::Assistant => {
                    if let Some(text) = msg.content.iter().find_map(Self::extract_text) {
                        builder = builder.with_model_message(&text);
                    }
                }
            }
        }

        if let Some(prompt) = system_prompt {
            builder = builder.with_system_prompt(&prompt);
        }

        if let Some(temp) = req.temperature() {
            builder = builder.with_temperature(*temp);
        }

        if let Some(max_tokens) = req.max_tokens() {
            builder = builder.with_max_output_tokens(*max_tokens as i32);
        }

        // Execute as stream
        let gemini_stream = builder
            .execute_stream()
            .await
            .map_err(|e| GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string())))?;

        // Transform gemini TryStream to Stream<Result>
        // TryStream yields Ok/Err directly, need to map to Result<StreamChunk, Error>
        let chunk_stream = gemini_stream
            .into_stream() // Convert TryStream to Stream
            .map(move |result| match result {
                Ok(response) => Self::convert_to_stream_chunk(response),
                Err(e) => {
                    let gemini_err = GeminiError::new(GeminiErrorKind::ApiRequest(e.to_string()));
                    Err(BotticelliError::from(gemini_err))
                }
            });

        Ok(Box::pin(chunk_stream))
    }
}

impl GeminiClient {
    /// Convert gemini_rust GenerationResponse to our StreamChunk.
    fn convert_to_stream_chunk(
        response: gemini_rust::generation::model::GenerationResponse,
    ) -> BotticelliResult<StreamChunk> {
        // Extract text from response
        let text = response.text();

        // Check if this is the final chunk
        let is_final = response
            .candidates
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some();

        // Determine finish reason
        let finish_reason = if is_final {
            response
                .candidates
                .first()
                .and_then(|c| c.finish_reason.as_ref())
                .map(|reason| match reason {
                    gemini_rust::generation::model::FinishReason::Stop => FinishReason::Stop,
                    gemini_rust::generation::model::FinishReason::MaxTokens => FinishReason::Length,
                    gemini_rust::generation::model::FinishReason::Safety
                    | gemini_rust::generation::model::FinishReason::Recitation
                    | gemini_rust::generation::model::FinishReason::Blocklist
                    | gemini_rust::generation::model::FinishReason::ProhibitedContent
                    | gemini_rust::generation::model::FinishReason::Spii
                    | gemini_rust::generation::model::FinishReason::ImageSafety => {
                        FinishReason::ContentFilter
                    }
                    gemini_rust::generation::model::FinishReason::MalformedFunctionCall => {
                        FinishReason::ToolUse
                    }
                    _ => FinishReason::Other,
                })
        } else {
            None
        };

        Ok(StreamChunk {
            content: Output::Text(text),
            is_final,
            finish_reason,
        })
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
    /// Current metadata reflects Gemini 2.5 Flash capabilities.
    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            provider: "gemini",
            model: self.model_name.clone(),
            max_input_tokens: 1_048_576, // Gemini 2.5 Flash supports up to 1M tokens
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
