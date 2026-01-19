//! Message types for Gemini Live API WebSocket protocol.
//!
//! This module defines the JSON message structures used to communicate with the
//! Gemini Live API over WebSockets.
//!
//! # Protocol Overview
//!
//! 1. Client connects to WebSocket endpoint
//! 2. Client sends `SetupMessage` with model and configuration
//! 3. Server responds with `setupComplete`
//! 4. Client and server exchange messages bidirectionally
//! 5. Connection closes when done
//!
//! # Message Types
//!
//! **Client Messages** (sent to server):
//! - `SetupMessage` - Initial configuration (first message only)
//! - `ClientContentMessage` - Text conversation turns
//! - `RealtimeInputMessage` - Audio/video streaming data
//! - `ToolResponseMessage` - Responses to function calls
//!
//! **Server Messages** (received from server):
//! - `setupComplete` - Handshake confirmation
//! - `serverContent` - Model-generated content
//! - `toolCall` - Request to execute functions
//! - `toolCallCancellation` - Cancel previous tool calls
//! - `goAway` - Disconnect warning
//!
//! # Example
//!
//! ```ignore
//! // This example shows the protocol structure but is not runnable
//! // as it uses internal types for documentation purposes.
//! use botticelli_models::{SetupMessage, SetupConfig, GenerationConfig, ClientContentMessage,
//!                         ClientContent, Turn, Part, TextPart};
//!
//! // Setup message (first message after WebSocket connection)
//! let setup = SetupMessage {
//!     setup: SetupConfig {
//!         model: "models/gemini-2.0-flash-exp".to_string(),
//!         generation_config: Some(GenerationConfig {
//!             response_modalities: Some(vec!["TEXT".to_string()]),
//!             temperature: Some(1.0),
//!             max_output_tokens: Some(100),
//!             ..Default::default()
//!         }),
//!         system_instruction: None,
//!         tools: None,
//!     }
//! };
//!
//! // Client content message (conversation turn)
//! let message = ClientContentMessage {
//!     client_content: ClientContent {
//!         turns: vec![Turn {
//!             role: "user".to_string(),
//!             parts: vec![Part::Text(TextPart { text: "Hello!".to_string() })],
//!         }],
//!         turn_complete: true,
//!     }
//! };
//! ```

use derive_getters::Getters;
use derive_setters::Setters;
use elicitation::{Prompt, Select};
use rmcp::tool;
use serde::{Deserialize, Serialize};

//
// ─── CLIENT MESSAGES ────────────────────────────────────────────────────────
//

/// Initial setup message sent immediately after WebSocket connection.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct SetupMessage {
    setup: SetupConfig,
}

/// Configuration for the Live API session.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Default,
    Getters,
    derive_builder::Builder,
    elicitation::Elicit,
)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into), default)]
pub struct SetupConfig {
    /// Model to use (e.g., "models/gemini-2.0-flash-exp")
    model: String,

    /// Generation parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,

    /// System instruction for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,

    /// Tools/functions available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

/// Generation configuration parameters.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Default,
    Getters,
    Setters,
    derive_builder::Builder,
    elicitation::Elicit,
)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into, strip_option), default)]
#[setters(prefix = "with_")]
pub struct GenerationConfig {
    /// Number of candidates to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_count: Option<i32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,

    /// Temperature for sampling (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,

    /// Top-k sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,

    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f64>,

    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f64>,

    /// Response modalities (e.g., ["TEXT"], ["AUDIO"])
    #[serde(skip_serializing_if = "Option::is_none")]
    response_modalities: Option<Vec<String>>,
}

/// System instruction for the model.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, elicitation::Elicit)]
#[serde(rename_all = "camelCase")]
pub struct SystemInstruction {
    parts: Vec<Part>,
}

/// Tool/function definition.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, elicitation::Elicit)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Client content message for conversation turns.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct ClientContentMessage {
    client_content: ClientContent,
}

/// Client conversation content.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct ClientContent {
    /// Conversation turns
    turns: Vec<Turn>,

    /// Whether this turn is complete
    turn_complete: bool,
}

/// A single conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, derive_builder::Builder)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct Turn {
    /// Role ("user", "model")
    role: String,

    /// Content parts
    parts: Vec<Part>,
}

/// Content part (text, inline data, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, elicitation::Elicit)]
#[serde(untagged)]
pub enum Part {
    /// Text content
    Text(TextPart),
    /// Inline data (images, audio, etc.)
    InlineData(InlineDataPart),
}

/// Text content part.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, elicitation::Elicit)]
pub struct TextPart {
    text: String,
}

/// Inline data content part.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, elicitation::Elicit)]
#[serde(rename_all = "camelCase")]
pub struct InlineDataPart {
    inline_data: InlineData,
}

/// Inline data with MIME type.
#[derive(Debug, Clone, Serialize, Deserialize, Getters, elicitation::Elicit)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    mime_type: String,
    data: String, // base64-encoded
}

/// Realtime input message for streaming audio/video.
///
/// Reserved for future realtime input feature.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeInputMessage {
    /// Realtime input
    realtime_input: RealtimeInput,
}

/// Realtime input data.
///
/// Reserved for future realtime input feature.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeInput {
    /// Media chunks
    media_chunks: Vec<MediaChunk>,
}

/// Media chunk for streaming.
///
/// Reserved for future realtime input feature.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct MediaChunk {
    /// MIME type
    mime_type: String,
    /// Base64-encoded data
    data: String,
}

/// Tool response message.
///
/// Reserved for future tool calling feature.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponseMessage {
    /// Tool response
    tool_response: ToolResponse,
}

/// Tool response data.
///
/// Reserved for future tool calling feature.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    /// Function responses
    function_responses: Vec<FunctionResponse>,
}

/// Function call response.
///
/// Reserved for future tool calling feature.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponse {
    /// Function call ID
    id: String,
    /// Function name
    name: String,
    /// Function response
    response: serde_json::Value,
}

//
// ─── SERVER MESSAGES ────────────────────────────────────────────────────────
//

/// Server message (received from WebSocket).
///
/// Contains exactly one of the message type fields plus optional usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct ServerMessage {
    /// Setup confirmation
    #[serde(skip_serializing_if = "Option::is_none")]
    setup_complete: Option<SetupComplete>,

    /// Model-generated content
    #[serde(skip_serializing_if = "Option::is_none")]
    server_content: Option<ServerContent>,

    /// Tool call request
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call: Option<LiveToolCall>,

    /// Tool call cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_cancellation: Option<LiveToolCallCancellation>,

    /// Disconnect warning
    #[serde(skip_serializing_if = "Option::is_none")]
    go_away: Option<GoAway>,

    /// Token usage metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_metadata: Option<UsageMetadata>,
}

/// Setup complete confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupComplete {}

/// Server content (model response).
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct ServerContent {
    /// Model turn content
    model_turn: ModelTurn,

    /// Whether this turn is complete
    turn_complete: bool,

    /// Whether this was interrupted
    #[serde(skip_serializing_if = "Option::is_none")]
    interrupted: Option<bool>,
}

/// Model turn content.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct ModelTurn {
    /// Content parts
    parts: Vec<Part>,
}

/// Tool call request from model.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct LiveToolCall {
    function_calls: Vec<FunctionCall>,
}

/// Function call from model.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    id: String,
    name: String,
    args: serde_json::Value,
}

/// Tool call cancellation.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct LiveToolCallCancellation {
    ids: Vec<String>,
}

/// Server disconnect warning.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct GoAway {
    reason: String,
}

/// Token usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Tokens in the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_token_count: Option<u32>,

    /// Tokens in the candidates (responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    candidates_token_count: Option<u32>,

    /// Total tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    total_token_count: Option<u32>,
}

//
// ─── HELPER IMPLEMENTATIONS ─────────────────────────────────────────────────
//

impl Part {
    /// Create a text part.
    #[tool]
    pub fn text(text: impl Into<String>) -> Self {
        Part::Text(TextPart { text: text.into() })
    }

    /// Extract text from a part, if it contains text.
    #[tool]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Part::Text(TextPart { text }) => Some(text),
            _ => None,
        }
    }
}

impl ServerMessage {
    /// Check if this is a setup complete message.
    #[tool]
    pub fn is_setup_complete(&self) -> bool {
        self.setup_complete.is_some()
    }

    /// Check if this is a server content message.
    ///
    /// Not yet used - reserved for future features.
    #[tool]
    pub fn is_server_content(&self) -> bool {
        self.server_content.is_some()
    }

    /// Check if this is a tool call message.
    ///
    /// Not yet used - reserved for future tool calling feature.
    #[tool]
    pub fn is_tool_call(&self) -> bool {
        self.tool_call.is_some()
    }

    /// Check if this is a go away (disconnect) message.
    #[tool]
    pub fn is_go_away(&self) -> bool {
        self.go_away.is_some()
    }

    /// Extract text from server content, if present.
    #[tool]
    pub fn extract_text(&self) -> Option<String> {
        self.server_content.as_ref().map(|content| {
            content
                .model_turn
                .parts
                .iter()
                .filter_map(|part| part.as_text())
                .collect::<Vec<_>>()
                .join("")
        })
    }

    /// Check if the turn is complete.
    #[tool]
    pub fn is_turn_complete(&self) -> bool {
        self.server_content
            .as_ref()
            .map(|content| content.turn_complete)
            .unwrap_or(false)
    }
}
