//! Input types for LLM requests.

use crate::MediaSource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Controls how an input is retained in conversation history.
///
/// In multi-act narratives, large inputs (especially table queries) can cause
/// token explosion as they're re-sent with every subsequent act. This enum
/// allows controlling retention behavior per input.
///
/// # Examples
///
/// ```
/// use botticelli_core::HistoryRetention;
///
/// // Retain full input (default)
/// let full = HistoryRetention::Full;
///
/// // Replace with summary after processing
/// let summary = HistoryRetention::Summary;
///
/// // Remove from history after processing
/// let drop = HistoryRetention::Drop;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, JsonSchema, elicitation::Elicit)]
#[serde(rename_all = "lowercase")]
pub enum HistoryRetention {
    /// Retain the entire input in conversation history (default).
    ///
    /// Use when:
    /// - Single-act narratives
    /// - Small inputs (< 5KB)
    /// - Subsequent acts need to re-examine the data
    #[default]
    Full,

    /// Replace with a concise summary after processing.
    ///
    /// The input is sent to the LLM, but after the response is received,
    /// it's replaced with a summary like `[Table: name, 10 rows, ~18KB]`.
    ///
    /// Use when:
    /// - Multi-act narratives
    /// - Large inputs (> 5KB)
    /// - Subsequent acts only need the decision/result
    Summary,

    /// Remove from conversation history after processing.
    ///
    /// The input is sent to the LLM, but completely removed from history
    /// after the response. Use with caution.
    ///
    /// Use when:
    /// - Maximum token savings needed
    /// - Input is truly one-time (never referenced again)
    Drop,
}

/// Supported input types to LLMs.
///
/// # Examples
///
/// ```
/// use botticelli_core::{Input, MediaSource};
///
/// // Text input
/// let text = Input::Text("Hello, world!".to_string());
///
/// // Image input with URL
/// let image = Input::Image {
///     mime: Some("image/png".to_string()),
///     source: MediaSource::Url("https://example.com/image.png".to_string()),
/// };
///
/// // Document input with base64
/// let doc = Input::Document {
///     mime: Some("application/pdf".to_string()),
///     source: MediaSource::Base64("JVBERi0xLj...".to_string()),
///     filename: Some("report.pdf".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[serde(tag = "type", content = "data")]
pub enum Input {
    /// Plain text input.
    Text(String),

    /// Image input (PNG, JPEG, WebP, GIF, etc.).
    Image {
        /// MIME type, e.g., "image/png" or "image/jpeg"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
    },

    /// Audio input (MP3, WAV, OGG, etc.).
    Audio {
        /// MIME type, e.g., "audio/mp3" or "audio/wav"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
    },

    /// Video input (MP4, WebM, AVI, etc.).
    Video {
        /// MIME type, e.g., "video/mp4" or "video/webm"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
    },

    /// Document input (PDF, DOCX, TXT, etc.).
    Document {
        /// MIME type, e.g., "application/pdf" or "text/plain"
        mime: Option<String>,
        /// Media source (URL, base64, or raw bytes)
        source: MediaSource,
        /// Optional filename for context
        filename: Option<String>,
    },

    /// Bot command execution (Discord, Slack, etc.).
    BotCommand {
        /// Platform name (e.g., "discord", "slack")
        platform: String,
        /// Command to execute (e.g., "server.get_stats")
        command: String,
        /// Command arguments as JSON values
        args: std::collections::HashMap<String, serde_json::Value>,
        /// Halt execution if command fails (default: false)
        #[serde(default)]
        required: bool,
        /// Cache duration in seconds
        cache_duration: Option<u64>,
        /// How to retain this input in conversation history (default: Full)
        ///
        /// Available with the `history-retention` feature.
        #[serde(default)]
        history_retention: HistoryRetention,
    },

    /// Table reference for querying database tables.
    Table {
        /// Name of the table to query
        table_name: String,
        /// Specific columns to select (default: all)
        columns: Option<Vec<String>>,
        /// WHERE clause for filtering
        where_clause: Option<String>,
        /// Maximum number of rows
        limit: Option<u32>,
        /// Offset for pagination
        offset: Option<u32>,
        /// ORDER BY clause
        order_by: Option<String>,
        /// Alias for {{alias}} interpolation
        alias: Option<String>,
        /// Output format (JSON, Markdown, CSV)
        format: TableFormat,
        /// Random sample N rows
        sample: Option<u32>,
        /// Pull rows and delete them from table (default: false)
        ///
        /// When enabled, performs an atomic pull-and-delete operation.
        /// Useful for pipeline workflows where content moves between tables.
        #[serde(default)]
        destructive_read: bool,
        /// How to retain this input in conversation history (default: Full)
        ///
        /// Available with the `history-retention` feature.
        #[serde(default)]
        history_retention: HistoryRetention,
    },

    /// Narrative reference for composing narratives.
    ///
    /// When encountered, the executor will load and execute the referenced
    /// narrative, using its final output as input for the current act.
    Narrative {
        /// Name of the narrative file (without .toml extension)
        name: String,
        /// Optional path relative to calling narrative (defaults to same directory)
        path: Option<String>,
        /// How to retain this input in conversation history (default: Full)
        ///
        /// Available with the `history-retention` feature.
        #[serde(default)]
        history_retention: HistoryRetention,
    },

    /// Tool call (for assistant messages with tool use).
    ///
    /// Represents a tool that the assistant wants to call.
    ToolCall {
        /// Unique ID for this tool call
        id: String,
        /// Name of the tool to call
        name: String,
        /// Arguments as JSON
        arguments: serde_json::Value,
    },

    /// Tool result (for user messages with tool responses).
    ///
    /// Represents the result from executing a tool.
    ToolResult {
        /// ID of the tool call this responds to
        tool_call_id: String,
        /// Result content as string
        content: String,
        /// Whether this was an error
        is_error: bool,
    },
}

impl Input {
    /// Get the history retention policy for this input.
    ///
    /// Returns `HistoryRetention::Full` for input types that don't support
    /// retention configuration.
    pub fn history_retention(&self) -> HistoryRetention {
        match self {
            Input::BotCommand {
                history_retention, ..
            } => *history_retention,
            Input::Table {
                history_retention, ..
            } => *history_retention,
            Input::Narrative {
                history_retention, ..
            } => *history_retention,
            _ => HistoryRetention::Full,
        }
    }

    /// Set the history retention policy for this input (if applicable).
    ///
    /// Only applies to BotCommand, Table, and Narrative inputs.
    /// Other input types are unaffected.
    pub fn with_history_retention(mut self, retention: HistoryRetention) -> Self {
        match &mut self {
            Input::BotCommand {
                history_retention, ..
            } => *history_retention = retention,
            Input::Table {
                history_retention, ..
            } => *history_retention = retention,
            Input::Narrative {
                history_retention, ..
            } => *history_retention = retention,
            _ => {}
        }
        self
    }
}

/// Output format for table data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[serde(rename_all = "lowercase")]
pub enum TableFormat {
    /// JSON array of objects
    Json,
    /// Markdown table
    Markdown,
    /// CSV format
    Csv,
}
