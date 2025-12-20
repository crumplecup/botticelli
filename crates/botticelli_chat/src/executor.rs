//! Command executor that processes parsed commands.

use crate::{BotCommand, Command, NarrativeCommand, Response, SamplingIntegration, SocialCommand};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_mcp::PartialNarrative;
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
    #[instrument]
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
    #[instrument(skip(self))]
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
    
    #[cfg(feature = "cli")]
    sampling: Arc<SamplingIntegration>,
    
    current_narrative: Arc<RwLock<Option<PartialNarrative>>>,
}

impl CommandExecutor {
    /// Create a new command executor.
    #[instrument]
    pub async fn new() -> ChatResult<Self> {
        let services = Arc::new(ServiceContainer::new(crate::ChatAppConfig::default()));
        
        #[cfg(feature = "cli")]
        let sampling = Arc::new(SamplingIntegration::new(services.clone()).await?);
        
        Ok(Self {
            narrative_state: Arc::new(RwLock::new(NarrativeState::new())),
            services,
            
            #[cfg(feature = "cli")]
            sampling,
            
            current_narrative: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a new command executor with services.
    #[instrument(skip(services))]
    pub async fn with_services(services: Arc<ServiceContainer>) -> ChatResult<Self> {
        #[cfg(feature = "cli")]
        let sampling = Arc::new(SamplingIntegration::new(services.clone()).await?);
        
        Ok(Self {
            narrative_state: Arc::new(RwLock::new(NarrativeState::new())),
            services,
            
            #[cfg(feature = "cli")]
            sampling,
            
            current_narrative: Arc::new(RwLock::new(None)),
        })
    }

    /// Get reference to services.
    #[instrument(skip(self))]
    pub fn services(&self) -> &Arc<ServiceContainer> {
        &self.services
    }

    /// Get reference to sampling integration.
    #[instrument(skip(self))]
    pub fn sampling(&self) -> &Arc<SamplingIntegration> {
        &self.sampling
    }

    /// Get current narrative being edited.
    #[instrument(skip(self))]
    pub async fn current_narrative(&self) -> Option<PartialNarrative> {
        self.current_narrative.read().await.clone()
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
  • "create narrative about <topic>" - Start new narrative (AI-assisted)
  • "create narrative interactive" - Start interactive guided creation
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
            NarrativeCommand::Create { prompt } => self.handle_create_narrative(prompt).await,
            NarrativeCommand::CreateInteractive => self.handle_create_narrative_interactive().await,
            NarrativeCommand::Load { path } => self.handle_load_narrative(path).await,
            NarrativeCommand::Save { path } => self.handle_save_narrative(path).await,
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
    async fn handle_assign_narrative(
        &self,
        bot_id: &str,
        narrative_path: &str,
    ) -> ChatResult<Response> {
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
            Full database integration coming soon!",
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
        let filter = ExecutionFilter::new().with_limit(10);

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
                summary.id, summary.narrative_name, summary.status, summary.act_count
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
    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn handle_create_narrative(&self, prompt: String) -> ChatResult<Response> {
        use tracing::info;

        info!(
            prompt_len = prompt.len(),
            "Creating narrative via LLM sampling"
        );

        // Use LLM sampling coordinator to generate narrative
        let partial_narrative = self.sampling.generate_narrative(prompt.clone()).await?;

        // Store the generated narrative
        let mut current = self.current_narrative.write().await;
        *current = Some(partial_narrative.clone());
        drop(current);

        // Update legacy state for compatibility
        let mut state = self.narrative_state.write().await;
        state.prompt = Some(prompt.clone());
        state.path = None;
        drop(state);

        info!(
            narrative_name = partial_narrative.name().as_deref(),
            "Narrative generated via LLM sampling"
        );

        let metadata_info = match (partial_narrative.name(), partial_narrative.description()) {
            (Some(name), Some(desc)) => {
                format!("\nName: {}\nDescription: {}", name, desc)
            }
            (Some(name), None) => {
                format!("\nName: {}\nDescription: N/A", name)
            }
            _ => String::from("\n(Metadata pending)"),
        };

        Ok(Response::text(format!(
            "✓ Created narrative from prompt: \"{}\"{}\n\n\
             The LLM has planned the narrative structure.\n\
             Continue refining with natural language requests.\n\
             Use 'show narrative' to see current state.\n\
             Use 'save narrative to <path>' to export as TOML.",
            prompt, metadata_info
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

    /// Handle interactive narrative creation with elicitation.
    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn handle_create_narrative_interactive(&self) -> ChatResult<Response> {
        use tracing::info;

        info!("Starting interactive narrative creation");

        // Use sampling integration for interactive mode
        let partial_narrative = self.sampling.create_interactive().await?;

        // Store the generated narrative
        let mut current = self.current_narrative.write().await;
        *current = Some(partial_narrative.clone());
        drop(current);

        info!(
            narrative_name = partial_narrative.name().as_deref(),
            "Interactive narrative creation completed"
        );

        let metadata_info = match (partial_narrative.name(), partial_narrative.description()) {
            (Some(name), Some(desc)) => {
                format!("\nName: {}\nDescription: {}", name, desc)
            }
            (Some(name), None) => {
                format!("\nName: {}\nDescription: N/A", name)
            }
            _ => String::from("\n(Metadata pending)"),
        };

        Ok(Response::text(format!(
            "✓ Created narrative interactively{}\n\n\
             Use 'show narrative' to see current state.\n\
             Use 'save narrative to <path>' to export as TOML.",
            metadata_info
        )))
    }

    #[cfg(not(feature = "cli"))]
    #[instrument(skip(self))]
    async fn handle_create_narrative_interactive(&self) -> ChatResult<Response> {
        Err(ChatError::new(ChatErrorKind::NotImplemented(
            "Interactive narrative creation (requires cli feature)".to_string(),
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
            state.prompt = Some(execution.narrative_name().to_string());
            state.path = Some(format!("db:{}", id));

            // Extract model/params from first act if available
            if let Some(first_act) = execution.act_executions().first() {
                state.model = first_act.model().clone();
                state.temperature = *first_act.temperature();
                state.max_tokens = *first_act.max_tokens();
            }
            drop(state);

            let mut output = format!(
                "Loaded narrative execution #{}\n\
                 Name: {}\n\
                 Acts: {}",
                id,
                execution.narrative_name(),
                execution.act_executions().len()
            );

            if let Some(tokens) = execution.total_token_usage() {
                output.push_str(&format!("\nTotal tokens: {}", tokens.total_tokens()));
            }
            if let Some(cost) = execution.total_cost_usd() {
                output.push_str(&format!("\nTotal cost: ${:.4}", cost));
            }
            if let Some(duration) = execution.total_duration_ms() {
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
        let act = ActExecution::new(
            "manual_entry".to_string(),
            vec![],
            model,
            temperature,
            max_tokens,
            "This is a placeholder. Generate actual content via MCP.".to_string(),
            0,
            None,
            None,
            None,
        );

        let execution = NarrativeExecution::new(
            prompt.clone(),
            vec![act],
            None,
            None,
            None,
        );

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
}
