//! Text generation tools.
//!
//! Simple LLM text generation with model selection and driver dispatch.

use crate::rmcp_server::BotticelliServer;
use crate::{GenerateParams, GenerateResult};
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
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use rmcp::model::ErrorCode;
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use std::borrow::Cow;

impl BotticelliServer {
    /// Generate text using an LLM with the provided prompt.
    #[instrument(skip(self, params), fields(model = params.model(), max_tokens = params.max_tokens(), temperature = params.temperature(), prompt_len = params.prompt().len()))]
    pub async fn generate(
        &self,
        Parameters(params): Parameters<GenerateParams>,
    ) -> Result<Json<GenerateResult>, rmcp::ErrorData> {
        let prompt = params.prompt().clone();
        let model = params.model().clone();
        let max_tokens = *params.max_tokens();
        let temperature = *params.temperature();

        debug!(%model, max_tokens, temperature, "Generating text");

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            // Dispatch to appropriate driver based on model prefix
            #[cfg(feature = "gemini")]
            if model.starts_with("gemini") || model.starts_with("models/gemini") {
                if let Some(driver) = self.gemini_driver().clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            None,
                        )
                        .await;
                } else {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Gemini driver not configured"),
                        None,
                    ));
                }
            }

            #[cfg(feature = "anthropic")]
            if model.starts_with("claude") {
                if let Some(driver) = self.anthropic_driver().clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            None,
                        )
                        .await;
                } else {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Anthropic driver not configured"),
                        None,
                    ));
                }
            }

            #[cfg(feature = "ollama")]
            if model.starts_with("llama")
                || model.starts_with("mistral")
                || model.starts_with("codellama")
            {
                if let Some(driver) = self.ollama_driver().clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            None,
                        )
                        .await;
                } else {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Ollama driver not configured"),
                        None,
                    ));
                }
            }

            #[cfg(feature = "huggingface")]
            if model.contains("huggingface") {
                if let Some(driver) = self.huggingface_driver().clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            None,
                        )
                        .await;
                } else {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("HuggingFace driver not configured"),
                        None,
                    ));
                }
            }

            #[cfg(feature = "groq")]
            if model.contains("groq") {
                if let Some(driver) = self.groq_driver().clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            None,
                        )
                        .await;
                } else {
                    return Err(rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Groq driver not configured"),
                        None,
                    ));
                }
            }

            Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("No driver available for model: {}", model)),
                None,
            ))
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
            let response_text = format!(
                "Placeholder response for: {}\n\nModel: {}\nMax tokens: {}\nTemperature: {}",
                prompt, model, max_tokens, temperature
            );

            debug!(
                response_len = response_text.len(),
                "Generated placeholder text"
            );

            Ok(Json(GenerateResult::new(
                response_text,
                model,
                Some(max_tokens),
            )))
        }
    }

    /// Generic helper to execute generation with any BotticelliDriver implementation.
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    #[instrument(skip(self, driver), fields(model, max_tokens, temperature))]
    async fn generate_with_driver<D>(
        &self,
        driver: D,
        prompt: String,
        model: String,
        max_tokens: u32,
        temperature: f32,
        system_prompt: Option<String>,
    ) -> Result<Json<GenerateResult>, rmcp::ErrorData>
    where
        D: botticelli_interface::BotticelliDriver<
                Request = botticelli_core::GenerateRequest,
                Response = botticelli_core::GenerateResponse,
            >,
    {
        // Build request
        let mut messages = Vec::new();

        if let Some(sys_prompt) = system_prompt {
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

        let request = botticelli_core::GenerateRequest::builder()
            .messages(messages)
            .max_tokens(max_tokens)
            .temperature(temperature)
            .build()
            .map_err(|e| to_mcp_error(e, "Failed to build request"))?;

        // Execute generation
        let response = driver
            .generate(&request)
            .await
            .map_err(|e| to_mcp_error(e, "Generation failed"))?;

        // Extract text from response
        let text = response
            .outputs()
            .first()
            .map(|output| match output {
                botticelli_core::Output::Text(t) => t.clone(),
                _ => format!("{:?}", output),
            })
            .unwrap_or_else(|| "No text generated".to_string());

        let tokens_used = response
            .usage()
            .map(|u| *u.total_tokens() as u32);

        debug!(response_len = text.len(), ?tokens_used, "Generated text");

        Ok(Json(GenerateResult::new(text, model, tokens_used)))
    }
}
