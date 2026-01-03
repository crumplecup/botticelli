//! ToolCalling trait implementation for Gemini.

use std::sync::Arc;
use crate::GeminiClient;
use async_trait::async_trait;
use botticelli_core::{GenerateRequest, GenerateResponse, Output, Role, ToolCall};
use botticelli_error::{GeminiError, GeminiErrorKind};
use botticelli_interface::ToolCalling;
use tracing::{debug, instrument};

use crate::gemini::GeminiResult;

#[async_trait]
impl ToolCalling for GeminiClient {
    type ToolDefinition = botticelli_core::ToolDefinition;
    type ToolResult = botticelli_core::ToolResult;

    #[instrument(skip(self, request, tools), fields(tool_count = tools.len()))]
    async fn generate_with_tools(
        &self,
        request: &GenerateRequest,
        tools: &[botticelli_core::ToolDefinition],
    ) -> GeminiResult<GenerateResponse> {
        use gemini_rust::{FunctionDeclaration, Tool};

        if tools.is_empty() {
            return self.generate_internal(request).await.map_err(Into::into);
        }

        debug!(tool_count = tools.len(), "Generating with tools");

        let start = std::time::Instant::now();
        let metrics = crate::LlmMetrics::get();
        let model_name = request.model().as_ref().map_or_else(
            || self.model_name(),
            |s| s.as_str()
        );

        metrics.requests().add(
            1,
            &[
                opentelemetry::KeyValue::new("provider", "gemini"),
                opentelemetry::KeyValue::new("model", model_name.to_string()),
            ],
        );

        // Convert tools to Gemini FunctionDeclarations
        let function_declarations: Vec<FunctionDeclaration> = tools
            .iter()
            .map(|t| {
                let decl_json = serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.input_schema(),
                });
                serde_json::from_value(decl_json)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| GeminiErrorKind::Serialization(Arc::new(e)))?;
        let gemini_tool = Tool::with_functions(function_declarations);
        debug!(function_count = tools.len(), "Converted tools to Gemini format");

        // Get rate-limited client
        let rate_limited_client = self.get_or_create_client(model_name)?;

        // Estimate tokens
        let estimated_tokens: u64 = request
            .messages()
            .iter()
            .flat_map(|msg| msg.content())
            .filter_map(Self::extract_text)
            .map(|text| Self::estimate_tokens(&text))
            .sum();
        let total_estimate = estimated_tokens + request.max_tokens().unwrap_or(1000) as u64;

        // Clone data for closure
        let messages = request.messages().clone();
        let temperature = request.temperature();
        let max_tokens = request.max_tokens();

        // Execute with rate limiting
        let response = rate_limited_client
            .execute(total_estimate, || async {
                let client = rate_limited_client.inner().client();
                let mut builder = client.generate_content();

                // Process messages
                let mut system_prompt = None;
                for msg in &messages {
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
                if let Some(temp) = temperature {
                    builder = builder.with_temperature(*temp);
                }
                if let Some(max_tok) = max_tokens {
                    builder = builder.with_max_output_tokens(*max_tok as i32);
                }

                builder = builder.with_tool(gemini_tool.clone());
                debug!("Added tools to Gemini builder");

                builder.execute().await.map_err(Self::parse_gemini_error)
            })
            .await;

        match response {
            Ok(resp) => {
                let duration = start.elapsed().as_secs_f64();
                metrics.record_request("gemini", model_name, duration);

                let function_calls = resp.function_calls();

                let outputs = if !function_calls.is_empty() {
                    debug!(call_count = function_calls.len(), "Response contains function calls");
                    let tool_calls: Vec<ToolCall> = function_calls
                        .iter()
                        .enumerate()
                        .map(|(idx, fc)| {
                            let id = format!("call_{}", idx);
                            ToolCall::new(id, fc.name.clone(), fc.args.clone())
                        })
                        .collect();
                    vec![Output::ToolCalls(tool_calls)]
                } else {
                    vec![Output::Text(resp.text())]
                };

                let stop_reason = if !function_calls.is_empty() {
                    botticelli_core::StopReason::ToolUse
                } else {
                    botticelli_core::StopReason::EndTurn
                };

                Ok(GenerateResponse::builder()
                    .outputs(outputs)
                    .stop_reason(stop_reason)
                    .build()
                    .map_err(|e| GeminiError::new(GeminiErrorKind::BuilderError(e.to_string())))?)
            }
            Err(e) => {
                metrics.record_error("gemini", model_name, "tool_calling_error");
                Err(e.into())
            }
        }
    }
}
