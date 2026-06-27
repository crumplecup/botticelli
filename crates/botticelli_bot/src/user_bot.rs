use botticelli_core::PostingJitter;
use derive_new::new;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// User-defined bot configuration, serialised to `botticelli-user-bots.jsonl`.
///
/// Create interactively via [`crate::BotConfigGenerator`]; only the four
/// human-supplied fields are elicited — scheduling and jitter get sensible
/// defaults and can be edited in the JSONL file later.
#[derive(Debug, Clone, Serialize, Deserialize, elicitation::Elicit, JsonSchema, new)]
#[prompt("Configure a user-defined bot:")]
pub struct UserBotConfig {
    /// Short identifier for this bot (used in logs and UI).
    #[prompt("Bot name (short identifier, e.g. 'morning_news'):")]
    pub name: String,

    /// Path to the narrative TOML file this bot will run.
    #[prompt("Path to the narrative TOML file:")]
    pub narrative_path: String,

    /// Name of the narrative within the TOML file.
    #[prompt("Narrative name within the file (the key under [narratives]):")]
    pub narrative_name: String,

    /// LLM model to use for all acts in the narrative.
    #[prompt("LLM model (e.g. 'claude-sonnet-4-6', 'gemini-2.0-flash', 'ollama/llama3.2'):")]
    pub model: String,

    /// Base interval between runs, in hours.
    #[prompt("Base run interval in hours (e.g. 24 for once a day):")]
    pub interval_hours: u64,

    /// Timing jitter applied around the base interval (irrelevant for generation-only bots).
    #[prompt("Timing jitter:")]
    pub jitter: PostingJitter,

    /// How many times to execute the narrative per cycle.
    #[prompt("Times to run the narrative per cycle (1 for once):")]
    pub run_count: u32,
}
