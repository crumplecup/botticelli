//! Output types from LLM responses.

use serde::{Deserialize, Serialize};

/// Supported output types from LLMs.
///
/// Note: Cannot derive `Eq`, `Hash`, `PartialOrd`, or `Ord` because the
/// `Embedding` variant contains `Vec<f32>`, and `f32` does not implement
/// these traits (floating point is not totally ordered).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Output {
    /// Plain text output.
    Text(String),

    /// Generated image output.
    Image {
        /// MIME type of the image
        mime: Option<String>,
        /// Binary image data
        data: Vec<u8>,
    },

    /// Generated audio output (text-to-speech, etc.).
    Audio {
        /// MIME type of the audio
        mime: Option<String>,
        /// Binary audio data
        data: Vec<u8>,
    },

    /// Generated video output.
    Video {
        /// MIME type of the video
        mime: Option<String>,
        /// Binary video data
        data: Vec<u8>,
    },

    /// Vector embedding output.
    Embedding(Vec<f32>),

    /// Structured JSON output.
    Json(serde_json::Value),

    /// Tool/function calls requested by the model.
    ///
    /// Contains one or more tool calls that need to be executed.
    /// The results should be sent back in a subsequent request.
    ToolCalls(Vec<ToolCall>),
}

/// A tool/function call made by the model.
///
/// This is returned in Output when the model decides to use a tool
/// rather than (or in addition to) generating text.
///
/// # Examples
///
/// ```
/// use botticelli_core::ToolCall;
/// use serde_json::json;
///
/// let call = ToolCall::new(
///     "call_123".to_string(),
///     "get_weather".to_string(),
///     json!({"location": "San Francisco"})
/// );
///
/// assert_eq!(*call.name(), "get_weather");
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    derive_getters::Getters,
    derive_builder::Builder,
    derive_new::new,
)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    id: String,
    /// Name of the tool/function to call
    name: String,
    /// Arguments to pass to the tool (as JSON)
    arguments: serde_json::Value,
}

/// Reason why generation stopped.
///
/// Per MCP specification, this indicates why the model stopped generating.
///
/// # Examples
///
/// ```
/// use botticelli_core::StopReason;
///
/// let reason = StopReason::EndTurn;
/// assert_eq!(format!("{:?}", reason), "EndTurn");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum StopReason {
    /// Model finished generating naturally (end of turn).
    #[default]
    EndTurn,
    /// Maximum token limit reached.
    MaxTokens,
    /// Model requested tool use.
    ToolUse,
    /// Content filtered/blocked.
    ContentFilter,
    /// User-initiated stop.
    StopSequence,
    /// Other/unknown reason.
    Other,
}
