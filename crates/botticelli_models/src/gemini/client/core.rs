//! Core Gemini client implementation.

use botticelli_rate_limit::TierConfigBuilder;
use rmcp::tool;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use tracing::instrument;

use gemini_rust::{Gemini, client::Model};

use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output, Role};
use botticelli_error::{BotticelliResult, GeminiError, GeminiErrorKind};
use botticelli_interface::Tier;
use botticelli_rate_limit::{BotticelliConfig, RateLimiter, TierConfig};

use super::tiered::TieredGemini;
use crate::gemini::GeminiResult;

/// Helper to convert builder errors to GeminiError
pub(crate) fn builder_error(e: impl std::fmt::Display) -> botticelli_error::GeminiError {
    use botticelli_error::{GeminiError, GeminiErrorKind};
    GeminiError::new(GeminiErrorKind::BuilderError(e.to_string()))
}

/// Client for Google Gemini API with per-model client pooling.
///
/// This client maintains a cache of model-specific Gemini clients, each with its own
/// rate limiter. Clients are created lazily on first use for each model.
#[derive(derive_getters::Getters)]
pub struct GeminiClient {
    /// Cache of model-specific REST API clients with rate limiting
    clients: Arc<Mutex<HashMap<String, RateLimiter<TieredGemini<TierConfig>>>>>,
    /// WebSocket Live API client (for live models)
    live_client: Option<crate::gemini::live_client::GeminiLiveClient>,
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
        let client_count = self.clients.lock().map(|guard| guard.len()).unwrap_or(0);
        f.debug_struct("GeminiClient")
            .field("model_name", &self.model_name)
            .field("base_tier", &self.base_tier.name())
            .field("cached_clients", &client_count)
            .finish_non_exhaustive()
    }
}

impl GeminiClient {
    /// Get the capabilities of this Gemini model.
    ///
    /// Returns capabilities based on model name. All Gemini models support:
    /// - Streaming
    /// - Tool calling
    /// - Token counting
    /// - JSON mode
    ///
    /// Additional capabilities vary by model (vision, video, audio, etc.).
    #[tool]
    pub fn capabilities(&self) -> crate::gemini::ModelCapabilities {
        // For now, return standard capabilities for all models
        // In the future, this could be model-specific
        crate::gemini::ModelCapabilities::standard()
    }

    /// Convert a model name string to a gemini-rust Model enum variant.
    fn model_name_to_enum(name: &str) -> Model {
        match name {
            "gemini-2.5-flash" => Model::Gemini25Flash,
            "gemini-2.5-flash-lite" => Model::Custom("models/gemini-2.5-flash-lite".to_string()),
            "gemini-2.5-pro" => Model::Gemini25Pro,
            "text-embedding-004" => Model::TextEmbedding004,
            other => {
                if other.starts_with("models/") {
                    Model::Custom(other.to_string())
                } else {
                    Model::Custom(format!("models/{}", other))
                }
            }
        }
    }

    /// Create a new Gemini client without rate limiting.
    #[tool]
    #[instrument(name = "gemini_client_new")]
    pub fn new() -> BotticelliResult<Self> {
        Self::new_with_config(None)
    }

    /// Create a new Gemini client with rate limiting.
    #[tool]
    #[instrument(name = "gemini_client_new_with_tier", skip(tier))]
    pub fn new_with_tier(tier: Option<Box<dyn Tier>>) -> BotticelliResult<Self> {
        Self::new_internal(tier).map_err(Into::into)
    }

    /// Create a new Gemini client with rate limiting and retry configuration.
    #[tool]
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
    #[tool]
    #[instrument(name = "gemini_client_new_with_config")]
    pub fn new_with_config(tier_name: Option<&str>) -> BotticelliResult<Self> {
        let tier_config = BotticelliConfig::load()
            .ok()
            .and_then(|config| config.get_tier("gemini", tier_name));

        Self::new_with_tier_config(tier_config)
    }

    /// Create a new Gemini client with a TierConfig (preserves model-specific overrides).
    fn new_with_tier_config(tier_config: Option<TierConfig>) -> BotticelliResult<Self> {
        let api_key = env::var("GEMINI_API_KEY").map_err(|_| GeminiErrorKind::MissingApiKey)?;

        let base_tier = match tier_config {
            Some(config) => config,
            None => TierConfigBuilder::default()
                .name("Free")
                .rpm(10u32)
                .tpm(250_000u64)
                .rpd(250u32)
                .max_concurrent(1u32)
                .cost_per_million_input_tokens(0.0)
                .cost_per_million_output_tokens(0.0)
                .models(HashMap::new())
                .build()
                .map_err(|e| GeminiError::new(GeminiErrorKind::BuilderError(e.to_string())))?,
        };

        let live_client = {
            let rpm = *base_tier.rpm();
            crate::gemini::live_client::GeminiLiveClient::new_with_rate_limit(rpm).ok()
        };

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            live_client,
            api_key,
            model_name: "gemini-2.5-flash".to_string(),
            base_tier,
            no_retry: false,
            max_retries: None,
            retry_backoff_ms: None,
        })
    }

    /// Build TierConfig from Tier trait.
    fn build_tier_config_from_trait(tier: Box<dyn Tier>) -> GeminiResult<TierConfig> {
        let mut builder = TierConfigBuilder::default();
        builder.name(tier.name());
        if let Some(rpm) = tier.rpm() {
            builder.rpm(rpm);
        }
        if let Some(tpm) = tier.tpm() {
            builder.tpm(tpm);
        }
        if let Some(rpd) = tier.rpd() {
            builder.rpd(rpd);
        }
        if let Some(max_concurrent) = tier.max_concurrent() {
            builder.max_concurrent(max_concurrent);
        }
        if let Some(daily_quota) = tier.daily_quota_usd() {
            builder.daily_quota_usd(daily_quota);
        }
        if let Some(input_cost) = tier.cost_per_million_input_tokens() {
            builder.cost_per_million_input_tokens(input_cost);
        }
        if let Some(output_cost) = tier.cost_per_million_output_tokens() {
            builder.cost_per_million_output_tokens(output_cost);
        }
        builder.models(HashMap::new());
        builder
            .build()
            .map_err(|e| GeminiError::new(GeminiErrorKind::BuilderError(e.to_string())))
    }

    /// Build default free tier config.
    fn build_default_tier_config() -> GeminiResult<TierConfig> {
        TierConfigBuilder::default()
            .name("Free")
            .rpm(10u32)
            .tpm(250_000u64)
            .rpd(250u32)
            .max_concurrent(1u32)
            .cost_per_million_input_tokens(0.0)
            .cost_per_million_output_tokens(0.0)
            .models(HashMap::new())
            .build()
            .map_err(|e| GeminiError::new(GeminiErrorKind::BuilderError(e.to_string())))
    }

    /// Internal constructor that returns Gemini-specific errors.
    fn new_internal(tier: Option<Box<dyn Tier>>) -> GeminiResult<Self> {
        let api_key = env::var("GEMINI_API_KEY").map_err(|_| GeminiErrorKind::MissingApiKey)?;

        let base_tier = if let Some(tier) = tier {
            Self::build_tier_config_from_trait(tier)?
        } else {
            Self::build_default_tier_config()?
        };

        let live_client = {
            let rpm = *base_tier.rpm();
            crate::gemini::live_client::GeminiLiveClient::new_with_rate_limit(rpm).ok()
        };

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            live_client,
            api_key,
            model_name: "gemini-2.5-flash".to_string(),
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

    /// Set the default model for this client.
    #[tool]
    pub fn set_default_model(&mut self, model: String) {
        self.model_name = model;
    }

    /// Check if a model name indicates a Live API model (requires WebSocket).
    fn is_live_model(model_name: &str) -> bool {
        model_name.contains("-live") || model_name.contains("-exp")
    }

    /// Extract text content from an input
    pub(crate) fn extract_text(input: &Input) -> Option<String> {
        match input {
            Input::Text(text) => Some(text.clone()),
            _ => None,
        }
    }

    /// Check if input contains non-text media
    pub(crate) fn has_media(inputs: &[Input]) -> bool {
        inputs.iter().any(|i| !matches!(i, Input::Text(_)))
    }

    /// Estimate token count from text (rough approximation: chars / 4).
    pub(crate) fn estimate_tokens(text: &str) -> u64 {
        (text.len() / 4).max(1) as u64
    }

    /// Generate response using Live API (WebSocket).
    async fn generate_via_live_api(
        &self,
        req: &GenerateRequest,
        model_name: &str,
    ) -> GeminiResult<GenerateResponse> {
        use tokio_retry2::{Retry, RetryError, strategy::ExponentialBackoff, strategy::jitter};
        use tracing::{info, warn};

        let live_client = self.live_client.as_ref().ok_or_else(|| {
            GeminiError::new(GeminiErrorKind::LiveClientUnavailable(
                "Live API client not available".to_string(),
            ))
        })?;

        let mut config_builder = crate::gemini::live_protocol::GenerationConfigBuilder::default();

        if let Some(max_tokens) = req.max_tokens() {
            config_builder.max_output_tokens(*max_tokens as i32);
        }
        if let Some(temp) = req.temperature() {
            config_builder.temperature(*temp as f64);
        }

        let config = config_builder
            .build()
            .map_err(|e| GeminiErrorKind::BuilderError(e.to_string()))?;

        if self.no_retry {
            let mut session = live_client.connect_with_config(model_name, config).await?;
            let combined_text = self.combine_messages(req);
            let response_text = session.send_text(&combined_text).await?;
            let _ = session.close().await;

            return GenerateResponse::builder()
                .outputs(vec![Output::Text(response_text)])
                .stop_reason(botticelli_core::StopReason::EndTurn)
                .usage(None)
                .build()
                .map_err(builder_error);
        }

        let model = model_name.to_string();
        let gen_config = config.clone();
        let client = live_client.clone();

        let first_result = client.connect_with_config(&model, gen_config.clone()).await;

        let (initial_ms, max_retries, max_delay_secs) = match first_result {
            Ok(mut session) => {
                let combined_text = self.combine_messages(req);
                let response_text = session.send_text(&combined_text).await?;
                let _ = session.close().await;

                return GenerateResponse::builder()
                    .outputs(vec![Output::Text(response_text)])
                    .stop_reason(botticelli_core::StopReason::EndTurn)
                    .usage(None)
                    .build()
                    .map_err(builder_error);
            }
            Err(e) => {
                if !e.kind.is_retryable() {
                    warn!(error = %e, "Permanent Live API error, failing immediately");
                    return Err(e.clone());
                }

                let (mut init_ms, mut retries, delay_secs) = e.kind.retry_strategy_params();

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

        let retry_strategy = ExponentialBackoff::from_millis(initial_ms)
            .factor(2)
            .max_delay(std::time::Duration::from_secs(max_delay_secs))
            .map(jitter)
            .take(max_retries);

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

        let combined_text = self.combine_messages(req);
        let response_text = session.send_text(&combined_text).await?;
        let _ = session.close().await;

        GenerateResponse::builder()
            .outputs(vec![Output::Text(response_text)])
            .stop_reason(botticelli_core::StopReason::EndTurn)
            .build()
            .map_err(builder_error)
    }

    /// Helper to combine all message content into a single text string.
    fn combine_messages(&self, req: &GenerateRequest) -> String {
        let mut combined_text = String::new();
        for msg in req.messages() {
            for input in msg.content() {
                if let Some(text) = Self::extract_text(input) {
                    combined_text.push_str(&text);
                    combined_text.push('\n');
                }
            }
        }
        tracing::debug!(
            combined_text_length = combined_text.len(),
            combined_text_preview = &combined_text[..combined_text.len().min(200)],
            "Combined messages for Gemini Live API"
        );
        combined_text
    }

    /// Get or create rate-limited client for a model.
    pub(crate) fn get_or_create_client(
        &self,
        model_name: &str,
    ) -> GeminiResult<RateLimiter<TieredGemini<TierConfig>>> {
        let mut clients = self
            .clients
            .lock()
            .map_err(|e| GeminiError::new(GeminiErrorKind::MutexPoisoned(e.to_string())))?;

        if !clients.contains_key(model_name) {
            let model_enum = Self::model_name_to_enum(model_name);
            let client =
                Gemini::with_model(&self.api_key, model_enum).map_err(GeminiError::from)?;
            let model_tier = self.base_tier.for_model(model_name);
            let tiered = TieredGemini::new(client, model_tier);
            let limiter = RateLimiter::new_with_retry(
                tiered,
                self.no_retry,
                self.max_retries,
                self.retry_backoff_ms,
            );
            clients.insert(model_name.to_string(), limiter);
        }

        clients
            .get(model_name)
            .ok_or_else(|| GeminiError::new(GeminiErrorKind::InvalidModel(model_name.to_string())))
            .cloned()
    }

    /// Build request to gemini-rust from our GenerateRequest.
    fn build_gemini_request(
        &self,
        req: &GenerateRequest,
        client: &Gemini,
    ) -> GeminiResult<gemini_rust::ContentBuilder> {
        let mut builder = client.generate_content();
        let mut system_prompt = None;

        for msg in req.messages() {
            match msg.role() {
                Role::System => {
                    if let Some(text) = msg.content().iter().find_map(Self::extract_text) {
                        system_prompt = Some(text);
                    }
                }
                Role::User => {
                    for input in msg.content() {
                        if let Some(text) = Self::extract_text(input) {
                            builder = builder.with_user_message(&text);
                        }
                    }
                    if Self::has_media(msg.content()) {
                        return Err(GeminiError::new(GeminiErrorKind::MultimodalNotSupported));
                    }
                }
                Role::Assistant => {
                    if let Some(text) = msg.content().iter().find_map(Self::extract_text) {
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
        if let Some(max_tok) = req.max_tokens() {
            builder = builder.with_max_output_tokens(*max_tok as i32);
        }

        Ok(builder)
    }

    /// Execute REST API generation with rate limiting.
    async fn generate_via_rest_api(
        &self,
        req: &GenerateRequest,
        model_name: &str,
    ) -> GeminiResult<GenerateResponse> {
        let rate_limited_client = self.get_or_create_client(model_name)?;

        let estimated_tokens: u64 = req
            .messages()
            .iter()
            .flat_map(|msg| msg.content())
            .filter_map(Self::extract_text)
            .map(|text| Self::estimate_tokens(&text))
            .sum();

        let total_estimate = estimated_tokens + req.max_tokens().unwrap_or(1000) as u64;

        let response = rate_limited_client
            .execute(total_estimate, || async {
                let client = &rate_limited_client.inner().client();
                let builder = self.build_gemini_request(req, client)?;
                builder.execute().await.map_err(Self::parse_gemini_error)
            })
            .await?;

        let text = response.text();
        GenerateResponse::builder()
            .outputs(vec![Output::Text(text)])
            .stop_reason(botticelli_core::StopReason::EndTurn)
            .usage(None)
            .build()
            .map_err(builder_error)
    }

    /// Internal generate method that returns Gemini-specific errors.
    pub(crate) async fn generate_internal(
        &self,
        req: &GenerateRequest,
    ) -> GeminiResult<GenerateResponse> {
        use crate::{LlmMetrics, classify_error};

        let start = std::time::Instant::now();
        let metrics = LlmMetrics::get();
        let model_name = req.model().as_ref().unwrap_or(&self.model_name);

        metrics.requests().add(
            1,
            &[
                opentelemetry::KeyValue::new("provider", "gemini"),
                opentelemetry::KeyValue::new("model", model_name.to_string()),
            ],
        );

        let result = if Self::is_live_model(model_name) {
            self.generate_via_live_api(req, model_name).await
        } else {
            self.generate_via_rest_api(req, model_name).await
        };

        let duration = start.elapsed().as_secs_f64();
        match &result {
            Ok(_) => {
                metrics.record_request("gemini", model_name, duration);
            }
            Err(e) => {
                let error_type = classify_error(e);
                metrics.record_error("gemini", model_name, error_type);
            }
        }

        result
    }

    /// Streaming generation (currently not implemented).
    ///
    /// Streaming support was removed in Phase 5. This method returns an error.
    pub(crate) async fn generate_stream_internal(
        &self,
        req: &GenerateRequest,
    ) -> GeminiResult<
        std::pin::Pin<
            Box<
                dyn futures_util::stream::Stream<Item = GeminiResult<botticelli_core::StreamChunk>>
                    + Send,
            >,
        >,
    > {
        use botticelli_core::{Output, StreamChunk};
        use futures_util::TryStreamExt;

        let model_name = req
            .model()
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(&self.model_name);

        // Get or create rate-limited client
        let rate_limited_client = self.get_or_create_client(model_name)?;
        let client = rate_limited_client.inner().client();

        // Build the request
        let builder = self.build_gemini_request(req, client)?;

        // Execute streaming request
        let stream = builder
            .execute_stream()
            .await
            .map_err(|e| GeminiErrorKind::GeminiRust(Arc::new(e)))?;

        // Convert gemini-rust stream to our StreamChunk format
        let converted_stream = stream
            .map_err(|e| GeminiError::from(GeminiErrorKind::GeminiRust(Arc::new(e))))
            .and_then(|response| async move {
                // Extract text from response candidates
                let text = response
                    .candidates
                    .first()
                    .map(|candidate| &candidate.content)
                    .and_then(|content| content.parts.as_ref())
                    .and_then(|parts| parts.first())
                    .and_then(|part| match part {
                        gemini_rust::Part::Text { text, .. } => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                StreamChunk::builder()
                    .content(Output::Text(text))
                    .is_final(false)
                    .build()
                    .map_err(builder_error)
            });

        Ok(Box::pin(converted_stream))
    }

    /// Parse gemini-rust errors to extract HTTP status codes.
    pub(crate) fn parse_gemini_error(err: gemini_rust::client::Error) -> GeminiError {
        let err_msg = err.to_string();

        if let Some(status_code) = Self::extract_status_code(&err_msg) {
            GeminiError::new(GeminiErrorKind::HttpError {
                status_code,
                message: err_msg,
            })
        } else {
            GeminiError::new(GeminiErrorKind::GeminiRust(std::sync::Arc::new(err)))
        }
    }

    /// Extract HTTP status code from error message string.
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
