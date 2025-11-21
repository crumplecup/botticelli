//! Input types for LLM requests.

use crate::MediaSource;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    },
}

/// Output format for table data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableFormat {
    /// JSON array of objects
    Json,
    /// Markdown table
    Markdown,
    /// CSV format
    Csv,
}
