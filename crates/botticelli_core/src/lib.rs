//! Core data types for the Botticelli LLM API library.
//!
//! This crate provides the foundation data types used across all Botticelli interfaces.

use serde::{Deserialize, Serialize};

/// Roles are the same across modalities (text, image, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Supported input types to LLMs.
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
}

/// Where media content is sourced from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaSource {
    /// URL to fetch the content from
    Url(String),
    /// Base64-encoded content
    Base64(String),
    /// Raw binary data
    Binary(Vec<u8>),
}

/// Supported output types from LLMs.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool/function to call
    pub name: String,
    /// Arguments to pass to the tool (as JSON)
    pub arguments: serde_json::Value,
}

/// A multimodal message in a conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Input>,
}

/// Generic generation request (multimodal-safe).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GenerateRequest {
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub model: Option<String>,
}

/// The unified response object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub outputs: Vec<Output>,
}
