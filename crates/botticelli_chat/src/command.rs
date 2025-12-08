//! Command types for chat operations.

use serde::{Deserialize, Serialize};

/// High-level command categories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Narrative-related commands.
    Narrative(NarrativeCommand),
    /// Bot-related commands.
    Bot(BotCommand),
    /// Social media commands.
    Social(SocialCommand),
    /// Help command.
    Help,
    /// Exit command.
    Exit,
}

/// Commands related to narrative operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NarrativeCommand {
    /// Create a new narrative from natural language prompt.
    Create {
        /// The natural language prompt describing the narrative.
        prompt: String,
    },
    /// Load an existing narrative from file.
    Load {
        /// Path to the narrative TOML file.
        path: String,
    },
    /// Save current narrative to file.
    Save {
        /// Path where to save the narrative.
        path: String,
    },
    /// Update the model in current narrative.
    UpdateModel {
        /// New model to use.
        model: String,
    },
    /// Update the prompt in current narrative.
    UpdatePrompt {
        /// New prompt text.
        prompt: String,
    },
    /// Update temperature parameter.
    UpdateTemperature {
        /// New temperature value.
        temperature: f32,
    },
    /// Update max tokens parameter.
    UpdateMaxTokens {
        /// New max tokens value.
        max_tokens: u32,
    },
    /// Validate current narrative.
    Validate,
    /// Show current narrative.
    Show,
    /// List available narratives.
    List,
}

/// Commands related to bot operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BotCommand {
    /// Create a new bot.
    Create {
        /// Bot name.
        name: String,
    },
    /// Assign narrative to bot.
    AssignNarrative {
        /// Bot ID.
        bot_id: String,
        /// Narrative path.
        narrative_path: String,
    },
    /// Show bot details.
    Show {
        /// Bot ID.
        bot_id: String,
    },
    /// List all bots.
    List,
}

/// Commands related to social media operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SocialCommand {
    /// Schedule a post.
    Schedule {
        /// Bot ID to use for posting.
        bot_id: String,
        /// Platform to post on.
        platform: String,
        /// When to post (ISO 8601 timestamp).
        schedule_time: String,
    },
    /// Show scheduled posts.
    ShowSchedule,
    /// Cancel a scheduled post.
    Cancel {
        /// Schedule ID to cancel.
        schedule_id: String,
    },
}

impl Command {
    /// Check if command is exit.
    pub fn is_exit(&self) -> bool {
        matches!(self, Self::Exit)
    }

    /// Check if command is help.
    pub fn is_help(&self) -> bool {
        matches!(self, Self::Help)
    }

    /// Check if command is narrative-related.
    pub fn is_narrative(&self) -> bool {
        matches!(self, Self::Narrative(_))
    }

    /// Check if command is bot-related.
    pub fn is_bot(&self) -> bool {
        matches!(self, Self::Bot(_))
    }

    /// Check if command is social-related.
    pub fn is_social(&self) -> bool {
        matches!(self, Self::Social(_))
    }
}

impl NarrativeCommand {
    /// Check if command modifies state.
    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::Create { .. }
                | Self::Save { .. }
                | Self::UpdateModel { .. }
                | Self::UpdatePrompt { .. }
                | Self::UpdateTemperature { .. }
                | Self::UpdateMaxTokens { .. }
        )
    }

    /// Check if command is read-only.
    pub fn is_readonly(&self) -> bool {
        !self.is_mutating()
    }
}
