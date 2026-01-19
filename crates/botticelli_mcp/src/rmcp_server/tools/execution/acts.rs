//! Single act execution tools.
//!
//! Execute individual narrative acts with optional context from previous acts.

use crate::rmcp_server::BotticelliServer;
use crate::{ExecuteActParams, ExecuteActResult};
use rmcp::handler::server::wrapper::{Json, Parameters};
use tracing::{debug, instrument};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use crate::rmcp_server::helpers::to_mcp_error;

impl BotticelliServer {
    /// Execute a single narrative act with context.
    #[instrument(skip(self, params), fields(model = params.model(), max_tokens = params.max_tokens(), prompt_len = params.prompt().len(), has_context = params.context().is_some()))]
    pub async fn execute_act(
        &self,
        Parameters(params): Parameters<ExecuteActParams>,
    ) -> Result<Json<ExecuteActResult>, rmcp::ErrorData> {
        let prompt = params.prompt().clone();
        let model = params.model().clone();
        let max_tokens = *params.max_tokens();
        let context = params.context().clone();

        debug!(%model, has_context = context.is_some(), "Executing act");

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            // Build system prompt with context
            let system_prompt = if let Some(ctx) = context {
                debug!(
                    context_len = ctx.len(),
                    "Including context from previous acts"
                );
                Some(format!(
                    "You are executing a narrative act. Here is context from previous acts:\n\n{}",
                    ctx
                ))
            } else {
                debug!("No context provided, using default system prompt");
                Some("You are executing a narrative act. Be concise and relevant.".to_string())
            };

            // Select appropriate driver
            let driver = self.select_driver(&model)?;

            // Call execute_act_with_driver with the selected driver
            return self
                .execute_act_with_driver(driver, prompt, model, max_tokens, system_prompt)
                .await;
        }

        #[cfg(not(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        )))]
        {
            // Placeholder implementation for testing when no LLM backends enabled
            let response = format!(
                "Act execution placeholder\n\nPrompt: {}\nModel: {}\nContext: {}\n\nFull execution requires LLM backend integration.",
                prompt,
                model,
                context.as_deref().unwrap_or("(none)")
            );

            debug!(
                response_len = response.len(),
                "Act execution complete (placeholder)"
            );

            Ok(Json(ExecuteActResult::new(
                response,
                model,
                Some(max_tokens),
                true,
            )))
        }
    }

    /// Generic helper to execute act with any ExecutionDriver implementation.
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    #[instrument(skip(self, driver), fields(model, max_tokens))]
    async fn execute_act_with_driver(
        &self,
        driver: std::sync::Arc<
            dyn botticelli_interface::ExecutionDriver<
                    botticelli_core::GenerateRequest,
                    botticelli_core::GenerateResponse,
                >,
        >,
        prompt: String,
        model: String,
        max_tokens: u32,
        system_prompt: Option<String>,
    ) -> Result<Json<ExecuteActResult>, rmcp::ErrorData> {
        // Build request
        let mut messages = Vec::new();

        if let Some(sys_prompt) = system_prompt {
            debug!(sys_prompt_len = sys_prompt.len(), "Adding system prompt");
            messages.push(
                botticelli_core::MessageBuilder::default()
                    .role(botticelli_core::Role::System)
                    .content(vec![botticelli_core::Input::Text(sys_prompt)])
                    .build()
                    .map_err(|e| to_mcp_error(e, "Failed to build system message"))?,
            );
        }

        messages.push(
            botticelli_core::MessageBuilder::default()
                .role(botticelli_core::Role::User)
                .content(vec![botticelli_core::Input::Text(prompt.clone())])
                .build()
                .map_err(|e| to_mcp_error(e, "Failed to build user message"))?,
        );

        // Use a default temperature for act execution
        let temperature = 0.7;

        let request = botticelli_core::GenerateRequest::builder()
            .messages(messages)
            .max_tokens(max_tokens)
            .temperature(temperature)
            .build()
            .map_err(|e| to_mcp_error(e, "Failed to build request"))?;

        debug!("Executing act with driver");

        // Execute generation
        let response = driver.generate(&request).await.map_err(|e| {
            use rmcp::model::ErrorCode;
            use std::borrow::Cow;
            rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Owned(format!("Act execution failed: {}", e)),
                None,
            )
        })?;

        // Extract text from response
        let text = response
            .outputs()
            .first()
            .map(|output| match output {
                botticelli_core::Output::Text(t) => t.clone(),
                _ => format!("{:?}", output),
            })
            .unwrap_or_else(|| "No text generated".to_string());

        let tokens_used = response.usage().map(|u| *u.total_tokens() as u32);

        debug!(
            response_len = text.len(),
            ?tokens_used,
            "Act execution complete"
        );

        Ok(Json(ExecuteActResult::new(text, model, tokens_used, true)))
    }
}
