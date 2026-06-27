//! `BotConfigGenerator` — interactive creator for `UserBotConfig`.
//!
//! Implements the `Generator` pattern from the elicitation framework:
//! elicit the generator once (four story-prompted questions), then call
//! `.generate()` to produce the full `UserBotConfig` with sensible defaults.
//!
//! Scheduling (interval, jitter, run count) are set to sensible defaults and
//! not asked; they can be edited in the JSONL file after creation.

use botticelli_core::PostingJitter;
use elicitation::Generator;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::UserBotConfig;

/// Generator for [`UserBotConfig`].
///
/// Elicit this for interactive bot creation. After the four key fields are
/// collected, call `.generate()` to obtain the ready-to-save [`UserBotConfig`].
///
/// The controller must pre-set [`elicitation::StringStyle::Human`] on the
/// communicator before calling `elicit` so the style-selection menu is skipped.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[prompt("Let's set up a new bot. I'll ask for the name, agent, and narrative.")]
pub struct BotConfigGenerator {
    /// Short identifier for the bot.
    #[prompt("What should we call this bot? (short identifier, e.g. morning_news):")]
    pub name: String,

    /// LLM agent model string.
    #[prompt(
        "Which AI agent will power this bot? (e.g. claude-sonnet-4-6, gemini-2.0-flash, ollama/llama3.2):"
    )]
    pub model: String,

    /// Path to the narrative TOML file.
    #[prompt("Path to the narrative TOML file this bot will run:")]
    pub narrative_path: String,

    /// Key of the narrative within the file.
    #[prompt(
        "Which narrative within that file? (the key under [narratives], e.g. article_draft):"
    )]
    pub narrative_name: String,
}

impl Generator for BotConfigGenerator {
    type Target = UserBotConfig;

    #[instrument(skip(self), fields(name = %self.name, model = %self.model))]
    fn generate(&self) -> UserBotConfig {
        UserBotConfig::new(
            self.name.trim().to_string(),
            self.narrative_path.trim().to_string(),
            self.narrative_name.trim().to_string(),
            self.model.trim().to_string(),
            24,
            PostingJitter::new(0, 0),
            1,
        )
    }
}
