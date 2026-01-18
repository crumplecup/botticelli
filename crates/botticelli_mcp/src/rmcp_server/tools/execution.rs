//! Narrative execution tools.
//!
//! Tools for generating content and executing narrative acts.

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use crate::rmcp_server::helpers::{default_model, to_mcp_error};
#[cfg(not(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
)))]
use crate::rmcp_server::helpers::to_mcp_error;
use crate::rmcp_server::BotticelliServer;
use crate::tools::NarrativeHelper;
use crate::{
    CreateNarrativeSessionParams, CreateNarrativeSessionResult, ExecuteActParams,
    ExecuteActResult, ExecuteNarrativeParams, ExecuteNarrativeResult, GenerateParams,
    GenerateResult, NarrativeAnalysis,
};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::ErrorCode;
use std::borrow::Cow;
use tracing::{debug, instrument, trace, warn};

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
                if let Some(driver) = self.gemini_driver.clone() {
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
                if let Some(driver) = self.anthropic_driver.clone() {
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
                if let Some(driver) = self.ollama_driver.clone() {
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
                if let Some(driver) = self.huggingface_driver.clone() {
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
                if let Some(driver) = self.groq_driver.clone() {
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
    
    /// Execute a complete narrative from a TOML file.
    #[instrument(skip(self, params), fields(narrative_path = params.narrative_path(), model = ?params.model(), max_tokens = params.max_tokens()))]
    pub async fn execute_narrative(
        &self,
        Parameters(params): Parameters<ExecuteNarrativeParams>,
    ) -> Result<Json<ExecuteNarrativeResult>, rmcp::ErrorData> {
        let narrative_path = params.narrative_path().clone();
        let model = params.model().clone();
        let prompt = params.prompt();
        let max_tokens = *params.max_tokens();

        debug!(%narrative_path, model = ?model, prompt_len = prompt.len(), max_tokens, "Executing narrative");

        // Validate path exists
        let path = std::path::Path::new(&narrative_path);
        if !path.exists() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!("Narrative file not found: {}", narrative_path)),
                None,
            ));
        }

        // Load and parse narrative TOML
        // First, read file to determine available narrative names
        let content = std::fs::read_to_string(path)
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Failed to read narrative file: {}", e)),
                    None,
                )
            })?;
        
        // Parse to extract narrative names
        let toml_file: toml::Value = toml::from_str(&content)
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    Cow::Owned(format!("Failed to parse TOML: {}", e)),
                    None,
                )
            })?;
        
        // Extract narrative name (either from [narrative] or first from [narratives])
        let narrative_name = if let Some(narrative) = toml_file.get("narrative") {
            // Single narrative - get its name
            narrative.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Borrowed("Narrative missing 'name' field"),
                        None,
                    )
                })?
                .to_string()
        } else if let Some(narratives) = toml_file.get("narratives") {
            // Multiple narratives - use first one
            narratives.as_table()
                .and_then(|table| table.keys().next())
                .ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        Cow::Borrowed("No narratives found in TOML file"),
                        None,
                    )
                })?
                .clone()
        } else {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Borrowed("No [narrative] or [narratives] section found in TOML"),
                None,
            ));
        };
        
        debug!(%narrative_name, "Loading narrative from TOML");

        // Now load the MultiNarrative with the identified name
        let multi = botticelli_narrative::MultiNarrative::from_file(path, &narrative_name)
            .map_err(|e| to_mcp_error(e, "Failed to load narrative"))?;
        
        // Get the active narrative
        let narrative = multi.get_narrative(&narrative_name)
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    Cow::Owned(format!("Narrative '{}' not found after loading", narrative_name)),
                    None,
                )
            })?;

        // Clone narrative for modification (preserves original in multi)
        let mut narrative_modified = narrative.clone();

        // Phase 2: Inject prompt into first act if provided and non-empty
        if !prompt.is_empty() {
            let first_act_name = narrative_modified.toc().order().first().cloned();
            
            if let Some(act_name) = first_act_name {
                if let Some(act_config) = narrative_modified.acts_mut().get_mut(&act_name) {
                    debug!(
                        act = %act_name,
                        prompt_len = prompt.len(),
                        original_input_count = act_config.inputs().len(),
                        "Injecting user prompt into first act"
                    );
                    
                    // Prepend prompt as Input::Text to existing inputs
                    let mut new_inputs = vec![botticelli_core::Input::Text(prompt.to_string())];
                    new_inputs.extend_from_slice(act_config.inputs());
                    let new_input_count = new_inputs.len();
                    act_config.set_inputs(new_inputs);
                    
                    trace!(
                        act = %act_name,
                        new_input_count = new_input_count,
                        "Prompt injection complete"
                    );
                } else {
                    warn!(act = %act_name, "First act not found in acts map, cannot inject prompt");
                }
            } else {
                warn!("Narrative has no acts in TOC, cannot inject prompt");
            }
        } else {
            trace!("Prompt is empty, skipping injection");
        }

        // Phase 3: Apply max_tokens override to acts without explicit max_tokens
        let mut override_count = 0;
        for (act_name, act_config) in narrative_modified.acts_mut().iter_mut() {
            if act_config.max_tokens().is_none() {
                debug!(
                    act = %act_name,
                    max_tokens = max_tokens,
                    "Applying max_tokens override"
                );
                act_config.set_max_tokens(Some(max_tokens));
                override_count += 1;
            } else {
                trace!(
                    act = %act_name,
                    act_max_tokens = ?act_config.max_tokens(),
                    "Act has explicit max_tokens, not overriding"
                );
            }
        }
        
        if override_count > 0 {
            debug!(
                acts_overridden = override_count,
                total_acts = narrative_modified.acts_mut().len(),
                "Applied max_tokens override to {} acts",
                override_count
            );
        }

        #[cfg(any(
            feature = "gemini",
            feature = "anthropic",
            feature = "ollama",
            feature = "huggingface",
            feature = "groq"
        ))]
        {
            // Determine which driver to use based on model
            let model_str = model.unwrap_or_else(default_model);
            
            // Use modified narrative for execution
            let narrative_to_execute = narrative_modified;
            
            // Helper to execute narrative and convert result
            let execute_narrative = |executor: botticelli_narrative::NarrativeExecutor<_>| async move {
                let source = botticelli_narrative::NarrativeSource::Single(
                    std::sync::Arc::new(narrative_to_execute)
                );
                
                let execution = executor.execute_from_source(&source).await
                    .map_err(|e| to_mcp_error(e, "Narrative execution failed"))?;
                
                // Extract final output from last act
                let final_output = execution.act_executions()
                    .last()
                    .map(|act| act.response().clone())
                    .unwrap_or_else(|| "No acts executed".to_string());
                
                // Extract models used from acts
                let models_used: Vec<String> = execution.act_executions()
                    .iter()
                    .filter_map(|act| act.model().clone())
                    .collect();
                
                // Calculate total tokens from execution
                let total_tokens = execution.total_token_usage()
                    .and_then(|usage| usage.total_tokens().map(|t| t as u32));
                
                Ok::<_, rmcp::ErrorData>(ExecuteNarrativeResult::new(
                    final_output,
                    execution.act_executions().len(),
                    models_used,
                    total_tokens,
                    true, // success - we got here without errors
                    None, // no error
                ))
            };

            // Select driver based on model prefix
            #[cfg(feature = "gemini")]
            if model_str.starts_with("gemini") || model_str.starts_with("models/gemini") {
                let driver = self.gemini_driver().as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Gemini driver not configured"),
                        None,
                    )
                })?;
                
                let executor = botticelli_narrative::NarrativeExecutor::new(driver.as_ref().clone());
                return execute_narrative(executor).await.map(Json);
            }

            #[cfg(feature = "anthropic")]
            if model_str.starts_with("claude") {
                let driver = self.anthropic_driver().as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Anthropic driver not configured"),
                        None,
                    )
                })?;
                
                let executor = botticelli_narrative::NarrativeExecutor::new(driver.as_ref().clone());
                return execute_narrative(executor).await.map(Json);
            }

            #[cfg(feature = "ollama")]
            if model_str.starts_with("ollama") {
                let driver = self.ollama_driver().as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Ollama driver not configured"),
                        None,
                    )
                })?;
                
                let executor = botticelli_narrative::NarrativeExecutor::new(driver.as_ref().clone());
                return execute_narrative(executor).await.map(Json);
            }

            #[cfg(feature = "huggingface")]
            if model_str.starts_with("hf") || model_str.starts_with("huggingface") {
                let driver = self.huggingface_driver().as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("HuggingFace driver not configured"),
                        None,
                    )
                })?;
                
                let executor = botticelli_narrative::NarrativeExecutor::new(driver.as_ref().clone());
                return execute_narrative(executor).await.map(Json);
            }

            #[cfg(feature = "groq")]
            if model_str.starts_with("groq") {
                let driver = self.groq_driver().as_ref().ok_or_else(|| {
                    rmcp::ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        Cow::Borrowed("Groq driver not configured"),
                        None,
                    )
                })?;
                
                let executor = botticelli_narrative::NarrativeExecutor::new(driver.as_ref().clone());
                return execute_narrative(executor).await.map(Json);
            }

            // No matching driver found
            Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(format!(
                    "No driver configured for model: {}. Available drivers require feature flags.",
                    model_str
                )),
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
            Err(rmcp::ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                Cow::Borrowed("No LLM drivers enabled. Enable at least one feature: gemini, anthropic, ollama, huggingface, or groq"),
                None,
            ))
        }
    }
    
    /// Create a new narrative session from a description.
    #[instrument(skip(self, params), fields(description_len = params.description().len()))]
    pub async fn create_narrative_session(
        &self,
        Parameters(params): Parameters<CreateNarrativeSessionParams>,
    ) -> Result<Json<CreateNarrativeSessionResult>, rmcp::ErrorData> {
        let description = params.description().clone();
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
        let narrative_id = self.narrative_registry().add(partial);

        debug!(narrative_id = %narrative_id, acts = acts.len(), "Session created");

        Ok(Json(CreateNarrativeSessionResult::new(
            narrative_id,
            suggested_name,
            NarrativeAnalysis::new(
                acts.iter().map(|a| a.name.clone()).collect(),
                complexity.to_string(),
                acts.len(),
            ),
        )))
    }
}
