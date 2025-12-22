//! Conversation state management.

use botticelli_error::{ChatError, ChatResult};

use crate::Message;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "cli")]
use botticelli_interface::{ChatState, Message as InterfaceMessage};

/// Conversation state tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    /// Conversation history.
    history: Vec<Message>,

    /// Currently loaded narrative path.
    current_narrative: Option<PathBuf>,

    /// Current bot context.
    current_bot: Option<String>,

    /// Session start time.
    session_start: DateTime<Utc>,

    /// Last activity timestamp.
    last_activity: DateTime<Utc>,
}

impl Default for ConversationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationState {
    /// Create a new conversation state.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            history: Vec::new(),
            current_narrative: None,
            current_bot: None,
            session_start: now,
            last_activity: now,
        }
    }

    /// Add a message to the history.
    pub fn add_message(&mut self, message: Message) {
        self.history.push(message);
        self.last_activity = Utc::now();
    }

    /// Get conversation history.
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.last_activity = Utc::now();
    }

    /// Set current narrative path.
    pub fn set_narrative(&mut self, path: impl Into<PathBuf>) {
        self.current_narrative = Some(path.into());
        self.last_activity = Utc::now();
    }

    /// Get current narrative path.
    pub fn current_narrative(&self) -> Option<&PathBuf> {
        self.current_narrative.as_ref()
    }

    /// Clear current narrative.
    pub fn clear_narrative(&mut self) {
        self.current_narrative = None;
        self.last_activity = Utc::now();
    }

    /// Set current bot.
    pub fn set_bot(&mut self, bot_id: impl Into<String>) {
        self.current_bot = Some(bot_id.into());
        self.last_activity = Utc::now();
    }

    /// Get current bot.
    pub fn current_bot(&self) -> Option<&str> {
        self.current_bot.as_deref()
    }

    /// Clear current bot.
    pub fn clear_bot(&mut self) {
        self.current_bot = None;
        self.last_activity = Utc::now();
    }

    /// Get session start time.
    pub fn session_start(&self) -> DateTime<Utc> {
        self.session_start
    }

    /// Get last activity timestamp.
    pub fn last_activity(&self) -> DateTime<Utc> {
        self.last_activity
    }

    /// Get session duration in seconds.
    pub fn session_duration(&self) -> i64 {
        (Utc::now() - self.session_start).num_seconds()
    }

    /// Get time since last activity in seconds.
    pub fn time_since_last_activity(&self) -> i64 {
        (Utc::now() - self.last_activity).num_seconds()
    }

    /// Save state to TOML file.
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> ChatResult<()> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| ChatError::execution_failed(format!("TOML serialization error: {}", e)))?;
        std::fs::write(path, toml_string)
            .map_err(|e| ChatError::execution_failed(format!("IO error: {}", e)))?;
        Ok(())
    }

    /// Load state from TOML file.
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> ChatResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ChatError::execution_failed(format!("IO error: {}", e)))?;
        let state = toml::from_str(&content)
            .map_err(|e| ChatError::parse_error(format!("TOML parse error: {}", e)))?;
        Ok(state)
    }
}

#[cfg(feature = "cli")]
impl ChatState for ConversationState {
    fn add_message(&mut self, message: InterfaceMessage) {
        // Convert interface message to chat message based on role
        let chat_message = match message.role.as_str() {
            "user" => Message::user(message.content.iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>().join("\n")),
            "assistant" => Message::assistant(message.content.iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>().join("\n")),
            _ => Message::system(message.content.iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>().join("\n")),
        };
        self.add_message(chat_message);
    }

    fn get_history(&self) -> Vec<InterfaceMessage> {
        self.history
            .iter()
            .map(|msg| {
                let (role, content_str) = match msg {
                    Message::User { content, .. } => ("user", content),
                    Message::Assistant { content, .. } => ("assistant", content),
                    Message::System { content, .. } => ("system", content),
                };
                InterfaceMessage {
                    role: role.to_string(),
                    content: vec![botticelli_core::Input::Text(content_str.clone())],
                }
            })
            .collect()
    }

    fn clear_history(&mut self) {
        self.history.clear();
        self.last_activity = Utc::now();
    }
}
