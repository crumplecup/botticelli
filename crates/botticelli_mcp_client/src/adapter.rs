//! [`ServerDriverAdapter`] — wraps [`BotticelliClient`] as a [`BotticelliDriver`].
//!
//! Routes `generate()` calls through the MCP server's `generate` tool so that
//! server-backed and cloud-backed providers present the same interface to the TUI.

use std::sync::Arc;

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output, StopReason};
use botticelli_error::BotticelliResult;
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
use serde_json::{Map, Value};
use tracing::instrument;

use crate::BotticelliClient;

/// A [`BotticelliDriver`] that delegates to a running `botticelli-server` via MCP.
///
/// Calls the server's `generate` tool, which routes to whichever backend
/// (Gemini, Anthropic, Ollama, …) the server was started with. This lets the
/// TUI treat a local server identically to a direct cloud provider.
#[derive(Clone)]
pub struct ServerDriverAdapter {
    client: Arc<BotticelliClient>,
    model: String,
    rate_limits: RateLimitConfig,
}

impl ServerDriverAdapter {
    /// Wrap an existing client connection.
    ///
    /// `model` is forwarded to the server's `generate` tool so the server can
    /// route to the right backend (e.g. `"gemini-2.0-flash-exp"`, `"claude-sonnet-4-6"`).
    pub fn new(client: Arc<BotticelliClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            rate_limits: RateLimitConfig::unlimited("botticelli-server"),
        }
    }
}

#[async_trait]
impl BotticelliDriver for ServerDriverAdapter {
    #[instrument(skip(self, req), fields(model = %self.model))]
    async fn generate(&self, req: &GenerateRequest) -> BotticelliResult<GenerateResponse> {
        // Collect user text from the last user message.
        let prompt = req
            .messages()
            .iter()
            .rfind(|m| matches!(m.role(), botticelli_core::Role::User))
            .map(|m| {
                m.content()
                    .iter()
                    .filter_map(|i| {
                        if let Input::Text(t) = i {
                            Some(t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();

        let mut args: Map<String, Value> = Map::new();
        args.insert("prompt".into(), Value::String(prompt));
        args.insert("model".into(), Value::String(self.model.clone()));
        if let Some(max_tokens) = req.max_tokens() {
            args.insert("max_tokens".into(), Value::Number((*max_tokens).into()));
        }
        if let Some(temperature) = req.temperature()
            && let Some(n) = serde_json::Number::from_f64(*temperature as f64)
        {
            args.insert("temperature".into(), Value::Number(n));
        }

        let result = self
            .client
            .call_tool("generate", Some(args))
            .await
            .map_err(|e| {
                botticelli_error::BotticelliError::new(
                    botticelli_error::BotticelliErrorKind::Backend(
                        botticelli_error::BackendError::new(e.to_string()),
                    ),
                )
            })?;

        // The server wraps the result as a JSON-pretty-printed text content item.
        let json_str = result
            .content
            .iter()
            .find_map(|c| c.as_text().map(|t| t.text.as_str()))
            .unwrap_or("{}");

        let parsed: Value = serde_json::from_str(json_str).unwrap_or(Value::Null);
        let text = parsed
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        GenerateResponse::builder()
            .outputs(vec![Output::Text(text)])
            .stop_reason(StopReason::EndTurn)
            .build()
            .map_err(|e| {
                botticelli_error::BotticelliError::new(
                    botticelli_error::BotticelliErrorKind::Backend(
                        botticelli_error::BackendError::new(e.to_string()),
                    ),
                )
            })
    }

    fn provider_name(&self) -> &'static str {
        "botticelli-server"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }
}
