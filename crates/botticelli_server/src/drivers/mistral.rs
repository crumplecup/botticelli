//! Embedded model inference via mistral-rs.
//!
//! Accepts any model format mistral-rs supports: GGUF quantized files, safetensors
//! checkpoints, and HuggingFace repo IDs. The model loads once on startup and runs
//! in mistral-rs's internal task. All inference calls go through a channel, so
//! callers are never blocked on the inference hot-path.

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output, Role, StopReason};
use botticelli_error::{BotticelliError, BotticelliResult, ServerError, ServerErrorKind};
use botticelli_interface::{BotticelliDriver, StreamChunk};
use botticelli_rate_limit::RateLimitConfig;
use derive_builder::Builder;
use mistralrs::{Model, ModelBuilder, Response, TextMessageRole, TextMessages};
use std::pin::Pin;
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};

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
/// Wraps a mistral-rs [`Model`] behind an `Arc` so the handle can be moved into
/// spawned tasks for streaming without requiring `Model: Clone`.
pub struct MistralDriver {
    model: std::sync::Arc<Model>,
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
            model: std::sync::Arc::new(model),
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

    #[instrument(skip(self, req), fields(messages = req.messages().len()))]
    async fn stream_generate(
        &self,
        req: &GenerateRequest,
    ) -> BotticelliResult<
        Option<Pin<Box<dyn futures::stream::Stream<Item = BotticelliResult<StreamChunk>> + Send>>>,
    > {
        // Build thinking-enabled messages and a plain fallback for models that
        // reject the thinking flag (e.g. non-reasoning GGUF quantisations).
        let mut messages_thinking = TextMessages::new().enable_thinking(true);
        let mut messages_plain = TextMessages::new();
        for msg in req.messages() {
            let text = inputs_to_text(msg.content());
            messages_thinking =
                messages_thinking.add_message(to_mistral_role(msg.role()), text.clone());
            messages_plain = messages_plain.add_message(to_mistral_role(msg.role()), text);
        }

        info!(
            message_count = req.messages().len(),
            "Starting streaming generate"
        );

        let model = std::sync::Arc::clone(&self.model);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<BotticelliResult<StreamChunk>>();

        tokio::spawn(async move {
            let inner: BotticelliResult<()> = (async {
                // Try thinking-enabled first; fall back to plain if the model rejects it.
                let mut mistral_stream = match model.stream_chat_request(messages_thinking).await {
                    Ok(s) => {
                        info!("Stream session open (thinking enabled)");
                        s
                    }
                    Err(e) => {
                        warn!(error = %e, "Thinking-enabled stream failed — retrying without thinking");
                        match model.stream_chat_request(messages_plain).await {
                            Ok(s) => {
                                info!("Stream session open (thinking disabled, plain fallback)");
                                s
                            }
                            Err(e2) => {
                                return Err(ServerError::new(ServerErrorKind::Api(format!(
                                    "stream init failed: {e2}"
                                )))
                                .into());
                            }
                        }
                    }
                };

                // mistral-rs separates reasoning from answer via Delta fields:
                //   delta.reasoning_content → thinking tokens (is_thinking = true)
                //   delta.content           → answer tokens  (is_thinking = false)
                // No <think> tag parsing needed — the driver handles the split.
                let mut chunk_count = 0usize;
                let mut thinking_chunks = 0usize;
                let mut answer_chunks = 0usize;

                info!("Entering mistral-rs stream poll loop");

                while let Some(response) = mistral_stream.next().await {
                    let delta = match response {
                        Response::Chunk(c) => {
                            match c.choices.into_iter().next().map(|ch| ch.delta) {
                                Some(d) => d,
                                None => {
                                    debug!("Chunk with no choices — skipping");
                                    continue;
                                }
                            }
                        }
                        Response::Done(r) => {
                            info!(
                                usage = ?r.usage,
                                "mistral-rs stream Done response received"
                            );
                            continue;
                        }
                        _ => {
                            warn!("Unexpected non-Chunk/non-Done mistral-rs response variant — skipping");
                            continue;
                        }
                    };

                    chunk_count += 1;
                    if chunk_count == 1 {
                        info!("First streaming chunk received from mistral-rs");
                    }

                    // Emit reasoning content as thinking chunks.
                    if let Some(thinking_text) = delta.reasoning_content
                        && !thinking_text.is_empty()
                    {
                        thinking_chunks += 1;
                        debug!(
                            chunk_count,
                            thinking_chunks,
                            text_len = thinking_text.len(),
                            "Emitting thinking chunk"
                        );
                        let chunk = StreamChunk::builder()
                            .content(Output::Text(thinking_text))
                            .is_final(false)
                            .is_thinking(true)
                            .build()
                            .map_err(|e| {
                                ServerError::new(ServerErrorKind::Api(format!(
                                    "thinking chunk build failed: {e}"
                                )))
                            })?;
                        if tx.send(Ok(chunk)).is_err() {
                            warn!("Receiver dropped during thinking phase");
                            return Ok(());
                        }
                    }

                    // Emit answer content as regular chunks.
                    if let Some(answer_text) = delta.content
                        && !answer_text.is_empty()
                    {
                        answer_chunks += 1;
                        debug!(
                            chunk_count,
                            answer_chunks,
                            text_len = answer_text.len(),
                            "Emitting answer chunk"
                        );
                        let chunk = StreamChunk::builder()
                            .content(Output::Text(answer_text))
                            .is_final(false)
                            .build()
                            .map_err(|e| {
                                ServerError::new(ServerErrorKind::Api(format!(
                                    "answer chunk build failed: {e}"
                                )))
                            })?;
                        if tx.send(Ok(chunk)).is_err() {
                            warn!("Receiver dropped during answer phase");
                            return Ok(());
                        }
                    }
                }

                info!(
                    chunk_count,
                    thinking_chunks,
                    answer_chunks,
                    "Mistral stream exhausted"
                );

                // Final sentinel — receiver uses this to mark the stream done.
                let sentinel = StreamChunk::builder()
                    .content(Output::Text(String::new()))
                    .is_final(true)
                    .build()
                    .map_err(|e| {
                        ServerError::new(ServerErrorKind::Api(format!(
                            "sentinel build failed: {e}"
                        )))
                    })?;
                let _ = tx.send(Ok(sentinel));
                info!("Final sentinel chunk sent");

                Ok::<(), BotticelliError>(())
            })
            .await;

            if let Err(e) = inner {
                error!(error = %e, "Streaming worker failed");
                let _ = tx.send(Err(e));
            }
        });

        let stream = futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(Some(Box::pin(stream)))
    }
}
