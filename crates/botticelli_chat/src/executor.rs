//! Command executor that processes parsed commands.

use crate::{
    BotCommand, ChatError, ChatErrorKind, ChatResult, Command, NarrativeCommand, Response,
    SocialCommand,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

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
        }
    }

    /// Check if state has any content.
    pub fn is_empty(&self) -> bool {
        self.prompt.is_none()
            && self.model.is_none()
            && self.temperature.is_none()
            && self.max_tokens.is_none()
    }
}

impl Default for NarrativeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Command executor that handles parsed commands.
pub struct CommandExecutor {
    narrative_state: Arc<RwLock<NarrativeState>>,
}

impl CommandExecutor {
    /// Create a new command executor.
    pub fn new() -> Self {
        Self {
            narrative_state: Arc::new(RwLock::new(NarrativeState::new())),
        }
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
            NarrativeCommand::Load { path: _ } => {
                Err(ChatError::new(ChatErrorKind::NotImplemented(
                    "Load narrative".to_string(),
                )))
            }
            NarrativeCommand::Save { path: _ } => {
                let state = self.narrative_state.read().await;
                if state.is_empty() {
                    return Err(ChatError::new(ChatErrorKind::InvalidState(
                        "No narrative to save".to_string(),
                    )));
                }
                drop(state);

                Err(ChatError::new(ChatErrorKind::NotImplemented(
                    "Save narrative".to_string(),
                )))
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
            NarrativeCommand::List => Err(ChatError::new(ChatErrorKind::NotImplemented(
                "List narratives".to_string(),
            ))),
        }
    }

    #[instrument(skip(self))]
    async fn handle_bot(&self, command: BotCommand) -> ChatResult<Response> {
        match command {
            BotCommand::Create { name } => Ok(Response::text(format!(
                "Bot creation not yet implemented: {}",
                name
            ))),
            BotCommand::AssignNarrative {
                bot_id,
                narrative_path,
            } => Ok(Response::text(format!(
                "Narrative assignment not yet implemented: bot={}, narrative={}",
                bot_id, narrative_path
            ))),
            BotCommand::Show { bot_id } => Ok(Response::text(format!(
                "Bot display not yet implemented: {}",
                bot_id
            ))),
            BotCommand::List => Ok(Response::text("Bot listing not yet implemented")),
        }
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
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}
