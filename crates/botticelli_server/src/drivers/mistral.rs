//! Embedded model inference via mistral-rs.
//!
//! Accepts any model format mistral-rs supports: GGUF quantized files, safetensors
//! checkpoints, and HuggingFace repo IDs. The model loads once on startup and runs
//! in mistral-rs's internal task. All inference calls go through a channel, so
//! callers are never blocked on the inference hot-path.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output, Role, StopReason};
use botticelli_error::{BotticelliResult, ServerError, ServerErrorKind};
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
use derive_builder::Builder;
use mistralrs::{Model, ModelBuilder, TextMessageRole, TextMessages};
use std::time::Instant;
use tracing::{debug, info, instrument};

/// Configuration for the embedded mistral-rs backend.
#[derive(Debug, Clone, Builder)]
pub struct MistralConfig {
    /// Local path to a model directory (safetensors or GGUF) or a HuggingFace repo ID.
    pub model_path: String,
    /// Human-readable model identifier returned by `model_name()`.
    pub model_id: String,
}

/// Embedded inference backend backed by mistral-rs.
///
/// Wraps a mistral-rs [`Model`], which internally runs the inference engine
/// in its own task. Construct with [`MistralDriver::load`].
pub struct MistralDriver {
    model: Model,
    config: MistralConfig,
    rate_limits: RateLimitConfig,
}

impl MistralDriver {
    /// Load a model from a local path or HuggingFace repo ID.
    ///
    /// Model loading is spawned so the async executor is not blocked during
    /// potentially multi-GB file I/O.
    #[instrument(skip(config), fields(model_id = %config.model_id, path = %config.model_path))]
    pub async fn load(config: MistralConfig) -> BotticelliResult<Self> {
        let path = config.model_path.clone();

        info!("Model loading started — this may take a minute on first run");

        let start = Instant::now();
        let model = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(ModelBuilder::new(path).build())
        })
        .await
        .map_err(|e| {
            ServerError::new(ServerErrorKind::ServerStartFailed(format!(
                "model load task panicked: {e}"
            )))
        })?
        .map_err(|e| {
            ServerError::new(ServerErrorKind::ModelDownloadFailed(format!(
                "model load failed: {e}"
            )))
        })?;

        info!(
            elapsed_secs = start.elapsed().as_secs(),
            "Model loaded and ready"
        );

        let rate_limits = RateLimitConfig::unlimited("mistral-rs");
        Ok(Self {
            model,
            config,
            rate_limits,
        })
    }
}

/// Convert our `Role` to mistral-rs `TextMessageRole`.
fn to_mistral_role(role: &Role) -> TextMessageRole {
    match role {
        Role::System => TextMessageRole::System,
        Role::User => TextMessageRole::User,
        Role::Assistant => TextMessageRole::Assistant,
    }
}

/// Extract text from a slice of `Input` values.
///
/// Non-text inputs (images, audio, etc.) are skipped — text-only models only.
/// Returns an empty string if no text inputs are present.
fn inputs_to_text(inputs: &[Input]) -> String {
    inputs
        .iter()
        .filter_map(|i| {
            if let Input::Text(t) = i {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[async_trait]
impl BotticelliDriver for MistralDriver {
    #[instrument(skip(self, req), fields(messages = req.messages().len()))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        let mut messages = TextMessages::new();
        let mut total_input_chars = 0usize;
        for msg in req.messages() {
            let role = to_mistral_role(msg.role());
            let text = inputs_to_text(msg.content());
            total_input_chars += text.len();
            messages = messages.add_message(role, text);
        }

        debug!(
            input_chars = total_input_chars,
            "Sending request to inference engine"
        );
        let start = Instant::now();

        let response = self.model.send_chat_request(messages).await.map_err(|e| {
            ServerError::new(ServerErrorKind::Api(format!("inference failed: {e}")))
        })?;

        let text = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        info!(
            elapsed_ms = start.elapsed().as_millis(),
            output_chars = text.len(),
            "Inference complete"
        );

        let response = GenerateResponse::builder()
            .outputs(vec![Output::Text(text)])
            .stop_reason(StopReason::EndTurn)
            .build()
            .expect("all required fields set");
        Ok(response)
    }

    fn provider_name(&self) -> &'static str {
        "mistral-rs"
    }

    fn model_name(&self) -> &str {
        &self.config.model_id
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }
}
