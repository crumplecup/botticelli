// ! Elicitation domain types using paradigm-based design.
//!
//! This module defines types that use the elicitation crate's paradigm traits
//! (Select, Affirm, Survey) to model user interactions in a type-safe,
//! composable way.

use elicitation::{Elicit, Prompt, Select};

/// Approach for defining acts in a narrative.
///
/// This type uses the Select paradigm for finite choice selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Elicit)]
pub enum ActApproach {
    /// Auto-extract acts from narrative description using LLM.
    AutoExtract,

    /// Manually specify act count and names.
    ManualCount,

    /// Enter acts one-by-one interactively.
    Interactive,
}

/// Type of input for an act.
///
/// This type uses the Select paradigm for finite choice selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Elicit)]
pub enum InputType {
    /// Text input.
    Text,

    /// Image input.
    Image,

    /// Audio input.
    Audio,

    /// Video input.
    Video,

    /// Document input.
    Document,

    /// Command execution input.
    Command,

    /// Database query input.
    Database,

    /// Narrative call input (composability).
    NarrativeCall,
}

/// Source type for media inputs (image, audio, video, document).
///
/// This type uses the Select paradigm for finite choice selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Elicit)]
pub enum MediaSource {
    /// Load media from a URL.
    Url,

    /// Provide media as base64-encoded data.
    Base64,
}

/// Output format for database query results.
///
/// This type uses the Select paradigm for finite choice selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Elicit)]
pub enum OutputFormat {
    /// JSON format.
    Json,

    /// CSV format.
    Csv,

    /// Plain text format.
    Text,

    /// Markdown table format.
    Markdown,
}

/// History retention mode for narrative calls.
///
/// This type uses the Select paradigm for finite choice selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Elicit)]
pub enum HistoryRetentionMode {
    /// Keep all history from previous acts.
    KeepAll,

    /// Keep only the last N messages.
    KeepLast,

    /// Clear all history before calling.
    Clear,
}

/// Narrative metadata configuration.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
#[prompt("Let's create a new narrative!")]
pub struct NarrativeMetadata {
    /// Narrative name (alphanumeric and underscores only).
    #[prompt("Enter narrative name (alphanumeric and underscores):")]
    pub name: String,

    /// Narrative description (what does this workflow do?).
    #[prompt("Enter narrative description (what does this workflow do?):")]
    pub description: String,

    /// Default model for all acts (optional).
    #[prompt("Enter default model for all acts (or leave empty):")]
    pub default_model: Option<String>,

    /// Default temperature (0.0-2.0, optional).
    #[prompt("Enter default temperature (0.0-2.0, or leave empty):")]
    pub default_temperature: Option<f64>,

    /// Default max tokens (optional).
    #[prompt("Enter default max tokens (or leave empty):")]
    pub default_max_tokens: Option<i64>,
}

/// Definition of a single narrative act.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct ActDefinition {
    /// Act name (alphanumeric and underscores only).
    #[prompt("Enter act name (alphanumeric and underscores):")]
    pub name: String,

    /// System prompt for this act.
    #[prompt("Enter system prompt for this act:")]
    pub prompt: String,
}

/// Carousel configuration for iterative execution.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, Copy, PartialEq, Elicit)]
pub struct CarouselConfig {
    /// Number of iterations (1-1000).
    pub iterations: i64,

    /// Estimated tokens per iteration (for rate limiting).
    pub estimated_tokens: i64,

    /// Continue execution if an iteration fails?
    pub continue_on_error: bool,
}

/// Configuration for text input.
///
/// This type uses the Survey paradigm for single-field elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct TextInputConfig {
    /// Text prompt content.
    #[prompt("Enter text prompt:")]
    pub text: String,
}

/// Configuration for media input (Image/Audio/Video).
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct MediaInputConfig {
    /// Media source type (URL or Base64).
    #[prompt("How will you provide the media?")]
    pub source_type: MediaSource,

    /// Source data (URL or base64 string).
    #[prompt("Enter URL or base64 data:")]
    pub source_data: String,

    /// Optional MIME type (e.g., image/png, audio/mp3).
    #[prompt("Enter MIME type (or leave empty):")]
    pub mime_type: Option<String>,
}

/// Configuration for document input.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct DocumentInputConfig {
    /// Media source type (URL or Base64).
    #[prompt("How will you provide the document?")]
    pub source_type: MediaSource,

    /// Source data (URL or base64 string).
    #[prompt("Enter URL or base64 data:")]
    pub source_data: String,

    /// Optional MIME type (e.g., application/pdf).
    #[prompt("Enter MIME type (or leave empty):")]
    pub mime_type: Option<String>,

    /// Optional filename.
    #[prompt("Enter filename (or leave empty):")]
    pub filename: Option<String>,
}

/// Configuration for table query input.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct TableQueryConfig {
    /// Table name.
    #[prompt("Enter table name:")]
    pub table_name: String,

    /// Comma-separated column names (or empty for all).
    #[prompt("Enter column names (comma-separated, or leave empty for all):")]
    pub columns: Option<String>,

    /// WHERE clause (without WHERE keyword).
    #[prompt("Enter WHERE clause (or leave empty):")]
    pub where_clause: Option<String>,

    /// Maximum number of rows.
    #[prompt("Enter row limit (or leave empty):")]
    pub limit: Option<i64>,

    /// Row offset.
    #[prompt("Enter offset (or leave empty):")]
    pub offset: Option<i64>,

    /// ORDER BY clause (without ORDER BY keyword).
    #[prompt("Enter ORDER BY clause (or leave empty):")]
    pub order_by: Option<String>,

    /// Alias for {{alias}} interpolation.
    #[prompt("Enter alias (or leave empty):")]
    pub alias: Option<String>,

    /// Output format.
    #[prompt("Select output format:")]
    pub format: OutputFormat,

    /// Random sample size.
    #[prompt("Enter sample size (or leave empty):")]
    pub sample: Option<i64>,

    /// Destructive read (pull and delete rows)?
    #[prompt("Destructive read (pull and delete rows)?")]
    pub destructive_read: bool,

    /// History retention mode.
    #[prompt("Select history retention mode:")]
    pub history_retention: HistoryRetentionMode,
}

/// Configuration for bot command input.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
/// Note: Arguments are collected separately via a loop.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct BotCommandConfig {
    /// Platform (e.g., discord, slack).
    #[prompt("Enter platform (e.g., discord, slack):")]
    pub platform: String,

    /// Command (e.g., server.get_stats).
    #[prompt("Enter command (e.g., server.get_stats):")]
    pub command: String,

    /// Is this command required (halt on failure)?
    #[prompt("Is this command required (halt on failure)?")]
    pub required: bool,

    /// Cache duration in seconds (optional).
    #[prompt("Enter cache duration in seconds (or leave empty):")]
    pub cache_duration: Option<i64>,

    /// History retention mode.
    #[prompt("Select history retention mode:")]
    pub history_retention: HistoryRetentionMode,
}

/// Configuration for narrative reference input.
///
/// This type uses the Survey paradigm for multi-field form elicitation.
#[derive(Debug, Clone, PartialEq, Elicit)]
pub struct NarrativeReferenceConfig {
    /// Narrative name (without .toml extension).
    #[prompt("Enter narrative name (without .toml):")]
    pub name: String,

    /// Optional custom path.
    #[prompt("Enter custom path (or leave empty):")]
    pub path: Option<String>,

    /// History retention mode.
    #[prompt("Select history retention mode:")]
    pub history_retention: HistoryRetentionMode,
}
