// ! Elicitation domain types using paradigm-based design.
//!
//! This module defines types that use the elicitation crate's paradigm traits
//! (Select, Affirm, Survey) to model user interactions in a type-safe,
//! composable way.

use elicitation::{Elicit, Prompt, Select, Survey};

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
pub struct NarrativeMetadata {
    /// Narrative name (alphanumeric and underscores only).
    pub name: String,

    /// Narrative description (what does this workflow do?).
    pub description: String,

    /// Default model for all acts (optional).
    pub default_model: Option<String>,

    /// Default temperature (0.0-2.0, optional).
    pub default_temperature: Option<f64>,

    /// Default max tokens (optional).
    pub default_max_tokens: Option<i64>,
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
