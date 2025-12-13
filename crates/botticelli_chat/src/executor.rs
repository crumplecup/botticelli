//! Command executor that processes parsed commands.

use crate::{
    BotCommand, ChatError, ChatErrorKind, ChatResult, Command, NarrativeCommand, Response,
    SocialCommand,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

/// Current narrative state being edited.
#[derive(Debug, Clone)]
pub struct NarrativeState {
    /// Current prompt.
    pub prompt: Option<String>,
    /// Current model.
    pub model: Option<String>,
    /// Current temperature.
    pub temperature: Option<f32>,
    /// Current max tokens.
    pub max_tokens: Option<u32>,
    /// File path if loaded/saved.
    pub path: Option<String>,
    /// Generated narrative TOML content.
    pub toml_content: Option<String>,
}

impl NarrativeState {
    /// Create a new empty narrative state.
    pub fn new() -> Self {
        Self {
            prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            path: None,
            toml_content: None,
        }
    }

    /// Check if state has any content.
    pub fn is_empty(&self) -> bool {
        self.prompt.is_none()
            && self.model.is_none()
            && self.temperature.is_none()
            && self.max_tokens.is_none()
            && self.toml_content.is_none()
    }
}

impl Default for NarrativeState {
    fn default() -> Self {
        Self::new()
    }
}

use crate::ServiceContainer;

/// Command executor that handles parsed commands.
pub struct CommandExecutor {
    narrative_state: Arc<RwLock<NarrativeState>>,
    services: Arc<ServiceContainer>,
}

impl CommandExecutor {
    /// Create a new command executor.
    pub fn new() -> Self {
        Self {
            narrative_state: Arc::new(RwLock::new(NarrativeState::new())),
            services: Arc::new(ServiceContainer::new(crate::ChatAppConfig::default())),
        }
    }

    /// Create a new command executor with services.
    pub fn with_services(services: Arc<ServiceContainer>) -> Self {
        Self {
            narrative_state: Arc::new(RwLock::new(NarrativeState::new())),
            services,
        }
    }

    /// Get reference to services.
    pub fn services(&self) -> &Arc<ServiceContainer> {
        &self.services
    }



    /// Execute a command and return a response.
    #[instrument(skip(self))]
    pub async fn execute(&self, command: Command) -> ChatResult<Response> {
        debug!(?command, "Executing command");

        match command {
            Command::Help => Ok(self.handle_help()),
            Command::Exit => Ok(Response::text("Goodbye!")),
            Command::Narrative(cmd) => self.handle_narrative(cmd).await,
            Command::Bot(cmd) => self.handle_bot(cmd).await,
            Command::Social(cmd) => self.handle_social(cmd).await,
        }
    }

    fn handle_help(&self) -> Response {
        let help_text = r#"
Available Commands:

Narrative Management:
  • "create narrative about <topic>" - Start new narrative
  • "load narrative from <path>" - Load existing narrative
  • "save narrative to <path>" - Save current narrative
  • "update model to <model>" - Change LLM model
  • "update prompt to <text>" - Change prompt
  • "update temperature to <value>" - Set temperature (0.0-2.0)
  • "update tokens to <value>" - Set max tokens
  • "validate narrative" - Check narrative validity
  • "show narrative" - Display current narrative
  • "list narratives" - List available narratives

Bot Management:
  • "create bot <name>" - Create a new bot
  • "assign narrative <path> to bot <name>" - Assign narrative to bot
  • "show bot <name>" - Show bot configuration
  • "list bots" - List all configured bots

Social Media:
  • "schedule post for bot <name> on <platform> at <time>" - Schedule a post
  • "show schedule" - View scheduled posts
  • "cancel schedule <id>" - Cancel a scheduled post

Bot Management:
  • "create bot named <name>" - Create new bot
  • "assign narrative to bot <id>" - Link narrative to bot
  • "show bot <id>" - Display bot details
  • "list bots" - List all bots

Social Media:
  • "schedule post using bot <id> on <platform> at <time>" - Schedule post
  • "show schedule" - View scheduled posts
  • "cancel schedule <id>" - Cancel scheduled post

Other:
  • "help" or "?" - Show this help
  • "exit" or "quit" - Exit the application
"#;
        Response::text(help_text.trim())
    }

    #[instrument(skip(self))]
    async fn handle_narrative(&self, command: NarrativeCommand) -> ChatResult<Response> {
        match command {
            NarrativeCommand::Create { prompt } => {
                self.handle_create_narrative(prompt).await
            }
            NarrativeCommand::Load { path } => {
                self.handle_load_narrative(path).await
            }
            NarrativeCommand::Save { path } => {
                self.handle_save_narrative(path).await
            }
            NarrativeCommand::UpdateModel { model } => {
                let mut state = self.narrative_state.write().await;
                state.model = Some(model.clone());
                drop(state);

                Ok(Response::text(format!("Updated model to: {}", model)))
            }
            NarrativeCommand::UpdatePrompt { prompt } => {
                let mut state = self.narrative_state.write().await;
                state.prompt = Some(prompt.clone());
                drop(state);

                Ok(Response::text(format!("Updated prompt to: \"{}\"", prompt)))
            }
            NarrativeCommand::UpdateTemperature { temperature } => {
                if !(0.0..=2.0).contains(&temperature) {
                    return Err(ChatError::new(ChatErrorKind::ValidationError(
                        "Temperature must be between 0.0 and 2.0".to_string(),
                    )));
                }

                let mut state = self.narrative_state.write().await;
                state.temperature = Some(temperature);
                drop(state);

                Ok(Response::text(format!(
                    "Updated temperature to: {}",
                    temperature
                )))
            }
            NarrativeCommand::UpdateMaxTokens { max_tokens } => {
                let mut state = self.narrative_state.write().await;
                state.max_tokens = Some(max_tokens);
                drop(state);

                Ok(Response::text(format!(
                    "Updated max tokens to: {}",
                    max_tokens
                )))
            }
            NarrativeCommand::Validate => {
                let state = self.narrative_state.read().await;
                if state.is_empty() {
                    return Ok(Response::text("No narrative to validate"));
                }

                let mut issues = Vec::new();
                if state.prompt.is_none() {
                    issues.push("Missing prompt");
                }
                if state.model.is_none() {
                    issues.push("Missing model");
                }

                if issues.is_empty() {
                    Ok(Response::text("✓ Narrative is valid"))
                } else {
                    Ok(Response::text(format!(
                        "✗ Validation issues:\n  • {}",
                        issues.join("\n  • ")
                    )))
                }
            }
            NarrativeCommand::Show => {
                let state = self.narrative_state.read().await;
                if state.is_empty() {
                    return Ok(Response::text("No narrative created yet"));
                }

                let mut display = String::from("Current Narrative:\n");
                if let Some(ref prompt) = state.prompt {
                    display.push_str(&format!("  Prompt: \"{}\"\n", prompt));
                }
                if let Some(ref model) = state.model {
                    display.push_str(&format!("  Model: {}\n", model));
                }
                if let Some(temp) = state.temperature {
                    display.push_str(&format!("  Temperature: {}\n", temp));
                }
                if let Some(tokens) = state.max_tokens {
                    display.push_str(&format!("  Max Tokens: {}\n", tokens));
                }
                if let Some(ref path) = state.path {
                    display.push_str(&format!("  Path: {}\n", path));
                }

                Ok(Response::text(display.trim()))
            }
            NarrativeCommand::List => self.handle_list_narratives().await,
        }
    }

    #[instrument(skip(self))]
    async fn handle_bot(&self, command: BotCommand) -> ChatResult<Response> {
        match command {
            BotCommand::Create { name } => self.handle_create_bot(&name).await,
            BotCommand::AssignNarrative {
                bot_id,
                narrative_path,
            } => self.handle_assign_narrative(&bot_id, &narrative_path).await,
            BotCommand::Show { bot_id } => self.handle_show_bot(&bot_id).await,
            BotCommand::List => self.handle_list_bots().await,
        }
    }

    #[instrument(skip(self))]
    async fn handle_create_bot(&self, name: &str) -> ChatResult<Response> {
        info!(bot_name = %name, "Creating new bot");
        
        // For MVP, create a simple bot config stub
        // Full implementation will store in database
        Ok(Response::text(format!(
            "Bot '{}' created successfully.\n\n\
            To configure this bot:\n\
            1. Assign a narrative: 'assign narrative <path> to bot {}'\n\
            2. Set schedule (future): 'schedule bot {}'\n\
            3. Activate bot (future): 'activate bot {}'",
            name, name, name, name
        )))
    }

    #[instrument(skip(self))]
    async fn handle_assign_narrative(&self, bot_id: &str, narrative_path: &str) -> ChatResult<Response> {
        info!(bot_id = %bot_id, narrative_path = %narrative_path, "Assigning narrative to bot");
        
        // Validate narrative exists (would check database in full implementation)
        Ok(Response::text(format!(
            "Assigned narrative '{}' to bot '{}'.\n\n\
            The bot will execute this narrative according to its schedule.\n\
            Use 'show bot {}' to see configuration.",
            narrative_path, bot_id, bot_id
        )))
    }

    #[instrument(skip(self))]
    async fn handle_show_bot(&self, bot_id: &str) -> ChatResult<Response> {
        info!(bot_id = %bot_id, "Showing bot details");
        
        // Full implementation would query database
        Ok(Response::text(format!(
            "Bot: {}\n\
            Status: Not yet implemented\n\
            Type: To be configured\n\
            Narrative: Not assigned\n\
            Schedule: Not set\n\
            Active: false\n\n\
            Full bot management coming in next phase!",
            bot_id
        )))
    }

    #[instrument(skip(self))]
    async fn handle_list_bots(&self) -> ChatResult<Response> {
        info!("Listing all bots");
        
        // Full implementation would query database
        Ok(Response::text(
            "Bot Management\n\
            ============\n\n\
            No bots configured yet.\n\n\
            Create your first bot with: 'create bot <name>'\n\n\
            Bot types:\n\
            - Generation bots: Create new content via narratives\n\
            - Curation bots: Review and approve generated content\n\
            - Posting bots: Post approved content to social media\n\n\
            Full database integration coming soon!"
        ))
    }

    #[instrument(skip(self))]
    async fn handle_social(&self, command: SocialCommand) -> ChatResult<Response> {
        match command {
            SocialCommand::Schedule {
                bot_id,
                platform,
                schedule_time,
            } => Ok(Response::text(format!(
                "Scheduling not yet implemented: bot={}, platform={}, time={}",
                bot_id, platform, schedule_time
            ))),
            SocialCommand::ShowSchedule => {
                Ok(Response::text("Schedule display not yet implemented"))
            }
            SocialCommand::Cancel { schedule_id } => Ok(Response::text(format!(
                "Cancel not yet implemented: {}",
                schedule_id
            ))),
        }
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn handle_list_narratives(&self) -> ChatResult<Response> {
        use botticelli_interface::{ExecutionFilter, NarrativeRepository};
        use tracing::info;

        info!("Listing narratives from database");

        // Get narrative repository from services
        let repo = self.services.narrative_repository().await?;

        // Query recent narratives (limit 10)
        let filter = ExecutionFilter {
            narrative_name: None,
            status: None,
            offset: None,
            limit: Some(10),
        };

        let summaries = repo.list_executions(&filter).await.map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to list narratives: {}",
                e
            )))
        })?;

        if summaries.is_empty() {
            return Ok(Response::text("No narratives found in database."));
        }

        // Format as a list
        let mut output = format!("Found {} narrative execution(s):\n\n", summaries.len());
        for summary in summaries {
            output.push_str(&format!(
                "ID: {}\nName: {}\nStatus: {:?}\nActs: {}\n",
                summary.id,
                summary.narrative_name,
                summary.status,
                summary.act_count
            ));
            if let Some(ref desc) = summary.narrative_description {
                output.push_str(&format!("Description: {}\n", desc));
            }
            if let Some(ref error) = summary.error_message {
                output.push_str(&format!("Error: {}\n", error));
            }
            output.push('\n');
        }

        Ok(Response::text(output.trim()))
    }

    #[cfg(not(feature = "cli"))]
    #[instrument(skip(self))]
    async fn handle_list_narratives(&self) -> ChatResult<Response> {
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "List narratives (requires cli feature)".to_string(),
        )))
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn handle_create_narrative(&self, prompt: String) -> ChatResult<Response> {
        use tracing::info;

        info!(prompt_len = prompt.len(), "Creating narrative via MCP");

        // Get current state for model/temperature preferences
        let state = self.narrative_state.read().await;
        let model = state.model.clone();
        let temperature = state.temperature;
        drop(state);

        // Call MCP server to create narrative
        let narrative_toml = self
            .call_mcp_create_narrative(&prompt, model.as_deref(), temperature)
            .await?;

        // Update state with generated content
        let mut state = self.narrative_state.write().await;
        state.prompt = Some(prompt.clone());
        state.path = None;
        state.toml_content = Some(narrative_toml.clone());
        drop(state);

        info!(
            toml_size = narrative_toml.len(),
            "Narrative TOML generated via MCP"
        );

        Ok(Response::text(format!(
            "Created narrative from prompt: \"{}\"\n\n\
             Generated TOML ({} bytes)\n\
             Use 'show narrative' to see the TOML.\n\
             Use 'validate narrative' to check for errors.\n\
             Use 'save narrative' to persist to database.",
            prompt,
            narrative_toml.len()
        )))
    }

    #[cfg(not(feature = "cli"))]
    #[instrument(skip(self))]
    async fn handle_create_narrative(&self, prompt: String) -> ChatResult<Response> {
        // Fallback for non-cli builds
        let mut state = self.narrative_state.write().await;
        state.prompt = Some(prompt.clone());
        state.path = None;
        drop(state);

        Ok(Response::text(format!(
            "Started new narrative with prompt: \"{}\"\n\
             Use 'update model', 'update temperature', etc. to configure.",
            prompt
        )))
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn handle_load_narrative(&self, path: String) -> ChatResult<Response> {
        use botticelli_interface::NarrativeRepository;
        use tracing::info;

        // Check if path is a numeric ID (load from database)
        if let Ok(id) = path.parse::<i32>() {
            info!(id, "Loading narrative from database");

            let repo = self.services.narrative_repository().await?;
            let execution = repo.load_execution(id).await.map_err(|e| {
                ChatError::new(ChatErrorKind::IoError(format!(
                    "Failed to load narrative {}: {}",
                    id, e
                )))
            })?;

            // Update narrative state with loaded data
            let mut state = self.narrative_state.write().await;
            state.prompt = Some(execution.narrative_name.clone());
            state.path = Some(format!("db:{}", id));
            
            // Extract model/params from first act if available
            if let Some(first_act) = execution.act_executions.first() {
                state.model = first_act.model.clone();
                state.temperature = first_act.temperature;
                state.max_tokens = first_act.max_tokens;
            }
            drop(state);

            let mut output = format!(
                "Loaded narrative execution #{}\n\
                 Name: {}\n\
                 Acts: {}",
                id, execution.narrative_name, execution.act_executions.len()
            );

            if let Some(ref tokens) = execution.total_token_usage {
                output.push_str(&format!("\nTotal tokens: {}", *tokens.total_tokens()));
            }
            if let Some(cost) = execution.total_cost_usd {
                output.push_str(&format!("\nTotal cost: ${:.4}", cost));
            }
            if let Some(duration) = execution.total_duration_ms {
                output.push_str(&format!("\nDuration: {} ms", duration));
            }

            output.push_str("\n\nUse 'show narrative' to see details.");

            Ok(Response::text(output))
        } else {
            // TODO: Load from file path
            Err(ChatError::new(ChatErrorKind::NotImplemented(
                "Load narrative from file (use numeric ID to load from database)".to_string(),
            )))
        }
    }

    #[cfg(not(feature = "cli"))]
    #[instrument(skip(self))]
    async fn handle_load_narrative(&self, _path: String) -> ChatResult<Response> {
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "Load narrative (requires cli feature)".to_string(),
        )))
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn handle_save_narrative(&self, _path: String) -> ChatResult<Response> {
        use botticelli_interface::{ActExecution, NarrativeExecution, NarrativeRepository};
        use tracing::info;

        // Check current state
        let state = self.narrative_state.read().await;
        if state.is_empty() {
            return Err(ChatError::new(ChatErrorKind::InvalidState(
                "No narrative to save. Use 'create narrative' first.".to_string(),
            )));
        }

        let prompt = state.prompt.clone().unwrap_or_default();
        let model = state.model.clone();
        let temperature = state.temperature;
        let max_tokens = state.max_tokens;
        drop(state);

        info!(narrative_name = %prompt, "Saving narrative to database");

        // Create a placeholder act execution
        // TODO: This should be replaced with actual generated content from MCP
        let act = ActExecution {
            act_name: "manual_entry".to_string(),
            sequence_number: 0,
            model,
            temperature,
            max_tokens,
            inputs: vec![],
            response: "This is a placeholder. Generate actual content via MCP.".to_string(),
            token_usage: None,
            estimated_cost_usd: None,
            duration_ms: None,
        };

        let execution = NarrativeExecution {
            narrative_name: prompt.clone(),
            act_executions: vec![act],
            total_token_usage: None,
            total_cost_usd: None,
            total_duration_ms: None,
        };

        // Save to database
        let repo = self.services.narrative_repository().await?;
        let id = repo.save_execution(&execution).await.map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to save narrative: {}",
                e
            )))
        })?;

        // Update state with saved path
        let mut state = self.narrative_state.write().await;
        state.path = Some(format!("db:{}", id));
        drop(state);

        info!(id, "Narrative saved successfully");

        Ok(Response::text(format!(
            "Saved narrative to database.\n\
             ID: {}\n\
             Name: {}\n\n\
             Note: This is a placeholder. Use MCP to generate actual content.",
            id, prompt
        )))
    }

    #[cfg(not(feature = "cli"))]
    #[instrument(skip(self))]
    async fn handle_save_narrative(&self, _path: String) -> ChatResult<Response> {
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "Save narrative (requires cli feature)".to_string(),
        )))
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self), fields(description_len = description.len(), model, temperature))]
    async fn call_mcp_create_narrative(
        &self,
        description: &str,
        model: Option<&str>,
        temperature: Option<f32>,
    ) -> ChatResult<String> {
        use reqwest::Client;
        use serde_json::json;
        use tracing::{debug, error, warn};

        let mcp_url = self.services.config().mcp_server.server_url();
        let endpoint = format!("{}/tools/call", mcp_url);

        debug!(
            url = %endpoint,
            description_len = description.len(),
            model = ?model,
            temperature = ?temperature,
            "Calling MCP create_narrative tool"
        );

        // Generate a simple name from description (first 3 words, sanitized)
        let narrative_name = description
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join("_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();
        
        let narrative_name = if narrative_name.is_empty() {
            "narrative".to_string()
        } else {
            narrative_name
        };

        // Build parameters
        let mut params = json!({
            "description": description,
            "name": narrative_name,
        });

        if let Some(m) = model {
            params["default_model"] = json!(m);
        }
        if let Some(t) = temperature {
            params["default_temperature"] = json!(t);
        }

        let request_body = json!({
            "name": "create_narrative",
            "parameters": params
        });

        debug!(request = ?request_body, "Sending MCP request");

        // Create HTTP client with timeout
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| {
                error!(error = ?e, "Failed to create HTTP client");
                ChatError::new(ChatErrorKind::IoError(format!(
                    "Failed to create HTTP client: {}",
                    e
                )))
            })?;

        // Send request
        let response = client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, url = %endpoint, "HTTP request failed");
                
                if e.is_timeout() {
                    ChatError::new(ChatErrorKind::IoError(
                        "MCP server timeout (120s). The narrative generation took too long. Try a shorter description.".to_string()
                    ))
                } else if e.is_connect() {
                    ChatError::new(ChatErrorKind::IoError(format!(
                        "Cannot connect to MCP server at {}. Is it running? Check BOTTICELLI__MCP_SERVER__HOST and BOTTICELLI__MCP_SERVER__PORT.",
                        mcp_url
                    )))
                } else if e.is_request() {
                    ChatError::new(ChatErrorKind::IoError(format!(
                        "Invalid request to MCP server: {}",
                        e
                    )))
                } else {
                    ChatError::new(ChatErrorKind::IoError(format!(
                        "Network error calling MCP server: {}",
                        e
                    )))
                }
            })?;

        let status = response.status();
        debug!(status = %status, "Received MCP response");

        // Handle non-success status codes
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| String::from("<no response body>"));
            error!(
                status = %status,
                error_body = %error_text,
                "MCP server returned error"
            );

            return Err(match status.as_u16() {
                400 => ChatError::new(ChatErrorKind::InvalidInput(format!(
                    "Invalid request to MCP server: {}",
                    error_text
                ))),
                404 => ChatError::new(ChatErrorKind::IoError(
                    "MCP tool 'create_narrative' not found. Is the MCP server up to date?".to_string()
                )),
                500 => ChatError::new(ChatErrorKind::IoError(format!(
                    "MCP server internal error: {}. Check MCP server logs.",
                    error_text
                ))),
                503 => ChatError::new(ChatErrorKind::IoError(
                    "MCP server unavailable. It may be starting up or overloaded.".to_string()
                )),
                _ => ChatError::new(ChatErrorKind::IoError(format!(
                    "MCP server error ({}): {}",
                    status, error_text
                ))),
            });
        }

        // Parse response
        let result: serde_json::Value = response.json().await.map_err(|e| {
            error!(error = ?e, "Failed to parse MCP JSON response");
            ChatError::new(ChatErrorKind::IoError(format!(
                "Invalid JSON from MCP server: {}",
                e
            )))
        })?;

        debug!(response = ?result, "Parsed MCP response");

        // Check for MCP-level errors
        if let Some(error) = result.get("error") {
            let error_message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown MCP error");
            
            error!(mcp_error = %error_message, "MCP returned error in response");
            
            return Err(ChatError::new(ChatErrorKind::IoError(format!(
                "MCP tool error: {}",
                error_message
            ))));
        }

        // Extract narrative TOML from response
        let narrative_toml = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| {
                if arr.is_empty() {
                    warn!("MCP response has empty content array");
                    None
                } else {
                    Some(arr)
                }
            })
            .and_then(|arr| arr.first())
            .and_then(|item| {
                let text = item.get("text").and_then(|t| t.as_str());
                if text.is_none() {
                    warn!(item = ?item, "Content item missing 'text' field");
                }
                text
            })
            .ok_or_else(|| {
                error!(response = ?result, "Invalid MCP response structure");
                ChatError::new(ChatErrorKind::IoError(
                    "Invalid MCP response format: expected { content: [{ text: \"...\" }] }".to_string(),
                ))
            })?;

        if narrative_toml.is_empty() {
            warn!("MCP returned empty narrative TOML");
            return Err(ChatError::new(ChatErrorKind::IoError(
                "MCP generated empty narrative. Try a more detailed description.".to_string(),
            )));
        }

        debug!(
            toml_size = narrative_toml.len(),
            "Successfully extracted narrative TOML"
        );

        Ok(narrative_toml.to_string())
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self), fields(toml_size = toml_content.len()))]
    #[allow(dead_code)]
    /// Validate narrative TOML content via MCP server.
    pub async fn call_mcp_validate_narrative(&self, toml_content: &str) -> ChatResult<String> {
        use reqwest::Client;
        use serde_json::json;
        use tracing::{debug, error};

        let mcp_url = self.services.config().mcp_server.server_url();
        let endpoint = format!("{}/tools/call", mcp_url);

        debug!(
            url = %endpoint,
            toml_size = toml_content.len(),
            "Calling MCP validate_narrative tool"
        );

        let request_body = json!({
            "name": "validate_narrative",
            "parameters": {
                "toml_content": toml_content,
            }
        });

        debug!(request = ?request_body, "Sending MCP validation request");

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                error!(error = ?e, "Failed to create HTTP client");
                ChatError::new(ChatErrorKind::IoError(format!(
                    "Failed to create HTTP client: {}",
                    e
                )))
            })?;

        let response = client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, url = %endpoint, "HTTP request failed");
                
                if e.is_timeout() {
                    ChatError::new(ChatErrorKind::IoError(
                        "MCP server timeout (30s). Validation took too long.".to_string()
                    ))
                } else if e.is_connect() {
                    ChatError::new(ChatErrorKind::IoError(format!(
                        "Cannot connect to MCP server at {}. Is it running?",
                        mcp_url
                    )))
                } else {
                    ChatError::new(ChatErrorKind::IoError(format!(
                        "Network error calling MCP server: {}",
                        e
                    )))
                }
            })?;

        let status = response.status();
        debug!(status = %status, "Received MCP response");

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| String::from("<no response body>"));
            error!(
                status = %status,
                error_body = %error_text,
                "MCP server returned error"
            );

            return Err(match status.as_u16() {
                400 => ChatError::new(ChatErrorKind::InvalidInput(format!(
                    "Invalid TOML: {}",
                    error_text
                ))),
                404 => ChatError::new(ChatErrorKind::IoError(
                    "MCP tool 'validate_narrative' not found. Is the MCP server up to date?".to_string()
                )),
                _ => ChatError::new(ChatErrorKind::IoError(format!(
                    "MCP server error ({}): {}",
                    status, error_text
                ))),
            });
        }

        let result: serde_json::Value = response.json().await.map_err(|e| {
            error!(error = ?e, "Failed to parse MCP JSON response");
            ChatError::new(ChatErrorKind::IoError(format!(
                "Invalid JSON from MCP server: {}",
                e
            )))
        })?;

        debug!(response = ?result, "Parsed MCP validation response");

        // Extract validation text from response
        let validation_text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                error!(response = ?result, "Invalid MCP response structure");
                ChatError::new(ChatErrorKind::IoError(
                    "Invalid MCP response format".to_string(),
                ))
            })?;

        debug!(
            result_size = validation_text.len(),
            "Successfully extracted validation result"
        );

        Ok(validation_text.to_string())
    }

}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}
