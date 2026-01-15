//! Narrative execution tools.
//!
//! Tools for generating content and executing narrative acts.

use super::super::helpers::{default_model, to_mcp_error};
use super::super::server::BotticelliServer;
use crate::{
    CreateNarrativeSessionParams, CreateNarrativeSessionResult, ExecuteActParams,
    ExecuteActResult, ExecuteNarrativeParams, ExecuteNarrativeResult, GenerateParams,
    GenerateResult, NarrativeAnalysis,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::tool;
use tracing::{debug, instrument};

impl BotticelliServer {
    pub async fn generate(
        &self,
        Parameters(GenerateParams {
            prompt,
            model,
            max_tokens,
            temperature,
            system_prompt,
        }): Parameters<GenerateParams>,
    ) -> Result<Json<GenerateResult>, rmcp::ErrorData> {
        debug!(%model, max_tokens, temperature, "Generating text");

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            use rmcp::model::ErrorCode;
            use std::borrow::Cow;

            // Dispatch to appropriate driver based on model prefix
            #[cfg(feature = "gemini")]
            if model.starts_with("gemini") || model.starts_with("models/gemini") {
                if let Some(driver) = self.gemini_driver.clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            system_prompt,
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
                if let Some(driver) = self.anthropic_driver.clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            system_prompt,
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
                if let Some(driver) = self.ollama_driver.clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            system_prompt,
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
                if let Some(driver) = self.huggingface_driver.clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            system_prompt,
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
                if let Some(driver) = self.groq_driver.clone() {
                    return self
                        .generate_with_driver(
                            driver,
                            prompt,
                            model,
                            max_tokens,
                            temperature,
                            system_prompt,
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

        let tokens_used = response.usage().map(|u| *u.total_tokens() as u32);

        debug!(response_len = text.len(), ?tokens_used, "Generated text");

        Ok(Json(GenerateResult::new(text, model, tokens_used)))
    }
    pub async fn execute_act(
        &self,
        Parameters(ExecuteActParams {
            prompt,
            model,
            max_tokens,
            temperature,
            system_prompt,
            context,
        }): Parameters<ExecuteActParams>,
    ) -> Result<Json<ExecuteActResult>, rmcp::ErrorData> {
        debug!(%model, has_context = context.is_some(), "Executing act");

        // Placeholder implementation
        // Full implementation would:
        // 1. Select driver based on model prefix
        // 2. Build message with system prompt, context, and user prompt
        // 3. Execute with driver
        // 4. Return response with token usage

        let response = format!(
            "Act execution placeholder\n\nPrompt: {}\nModel: {}\nContext: {}\n\nFull execution requires LLM backend integration.",
            prompt,
            model,
            context.as_deref().unwrap_or("(none)")
        );

        debug!(response_len = response.len(), "Act execution complete");

        Ok(Json(ExecuteActResult::new(
            response,
            model,
            Some(max_tokens),
            true,
        )))
    }
    pub async fn execute_narrative(
        &self,
        Parameters(ExecuteNarrativeParams {
            narrative_path,
            prompt,
            model,
            max_tokens,
        }): Parameters<ExecuteNarrativeParams>,
    ) -> Result<Json<ExecuteNarrativeResult>, rmcp::ErrorData> {
        use rmcp::model::ErrorCode;
        use std::borrow::Cow;

        debug!(%narrative_path, model = ?model, "Executing narrative");

        // Validate path exists
        let path = std::path::Path::new(&narrative_path);
        if !path.exists() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative file not found: {}", narrative_path)),
                None,
            ));
        }

        // Placeholder implementation
        // Full implementation would:
        // 1. Load and parse narrative TOML
        // 2. Create NarrativeExecutor with appropriate driver
        // 3. Execute all acts in sequence
        // 4. Track tokens and outputs
        // 5. Return final result

        let final_output = format!(
            "Narrative execution placeholder\n\nPath: {}\nPrompt: {}\nModel: {:?}\n\nFull execution requires LLM backend and narrative executor integration.",
            narrative_path, prompt, model
        );

        debug!("Narrative execution complete (placeholder)");

        Ok(Json(ExecuteNarrativeResult::new(
            final_output,
            0, // acts_executed
            vec![model.unwrap_or_else(default_model)],
            Some(max_tokens),
            true,
            None,
        )))
    }
    pub async fn create_narrative_session(
        &self,
        Parameters(CreateNarrativeSessionParams { description }): Parameters<
            CreateNarrativeSessionParams,
        >,
    ) -> Result<Json<CreateNarrativeSessionResult>, rmcp::ErrorData> {
        use crate::tools::NarrativeHelper;

        debug!(?description, "Creating narrative session");

        // Analyze description
        let acts = NarrativeHelper::extract_acts_from_description(&description);
        let suggested_name = NarrativeHelper::suggest_name_from_description(&description);

        let complexity = if acts.len() == 1 {
            "simple"
        } else if acts.len() <= 3 {
            "moderate"
        } else {
            "complex"
        };

        // Initialize session state
        let mut partial = crate::PartialNarrative::new();
        partial.with_description(Some(description.clone()));
        partial.with_name(Some(suggested_name.clone()));

        // Add acts
        for act in &acts {
            partial.acts_mut().insert(
                act.name.clone(),
                crate::PartialAct::new(act.prompt.clone(), None, None, vec![], None),
            );
            partial.act_order_mut().push(act.name.clone());
        }

        // Store in registry (returns the narrative name as the key/ID)
        let narrative_id = self.narrative_registry.add(partial);

        debug!(narrative_id = %narrative_id, acts = acts.len(), "Session created");

        Ok(Json(CreateNarrativeSessionResult {
            narrative_id,
            suggested_name,
            analysis: NarrativeAnalysis {
                detected_acts: acts.iter().map(|a| a.name.clone()).collect(),
                complexity: complexity.to_string(),
                act_count: acts.len(),
            },
        }))
    }
}
