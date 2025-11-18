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
//! ```no_run
//! use botticelli::{SetupMessage, SetupConfig, GenerationConfig, ClientContentMessage,
//!                  ClientContent, Turn, Part, TextPart};
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

use serde::{Deserialize, Serialize};

//
// ─── CLIENT MESSAGES ────────────────────────────────────────────────────────
//

/// Initial setup message sent immediately after WebSocket connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupMessage {
    pub setup: SetupConfig,
}

/// Configuration for the Live API session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetupConfig {
    /// Model to use (e.g., "models/gemini-2.0-flash-exp")
    pub model: String,

    /// Generation parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,

    /// System instruction for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemInstruction>,

    /// Tools/functions available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

/// Generation configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Number of candidates to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,

    /// Temperature for sampling (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-k sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Response modalities (e.g., ["TEXT"], ["AUDIO"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<String>>,
}

/// System instruction for the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInstruction {
    pub parts: Vec<Part>,
}

/// Tool/function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Client content message for conversation turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientContentMessage {
    pub client_content: ClientContent,
}

/// Client conversation content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientContent {
    /// Conversation turns
    pub turns: Vec<Turn>,

    /// Whether this turn is complete
    pub turn_complete: bool,
}

/// A single conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    /// Role ("user", "model")
    pub role: String,

    /// Content parts
    pub parts: Vec<Part>,
}

/// Content part (text, inline data, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    /// Text content
    Text(TextPart),
    /// Inline data (images, audio, etc.)
    InlineData(InlineDataPart),
}

/// Text content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPart {
    pub text: String,
}

/// Inline data content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineDataPart {
    pub inline_data: InlineData,
}

/// Inline data with MIME type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    pub mime_type: String,
    pub data: String, // base64-encoded
}

/// Realtime input message for streaming audio/video.
///
/// Not yet used - reserved for future realtime input feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeInputMessage {
    pub realtime_input: RealtimeInput,
}

/// Realtime input data.
///
/// Not yet used - reserved for future realtime input feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeInput {
    pub media_chunks: Vec<MediaChunk>,
}

/// Media chunk for streaming.
///
/// Not yet used - reserved for future realtime input feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaChunk {
    pub mime_type: String,
    pub data: String, // base64-encoded
}

/// Tool response message.
///
/// Not yet used - reserved for future tool calling feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponseMessage {
    pub tool_response: ToolResponse,
}

/// Tool response data.
///
/// Not yet used - reserved for future tool calling feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub function_responses: Vec<FunctionResponse>,
}

/// Function call response.
///
/// Not yet used - reserved for future tool calling feature.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponse {
    pub id: String,
    pub name: String,
    pub response: serde_json::Value,
}

//
// ─── SERVER MESSAGES ────────────────────────────────────────────────────────
//

/// Server message (received from WebSocket).
///
/// Contains exactly one of the message type fields plus optional usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMessage {
    /// Setup confirmation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_complete: Option<SetupComplete>,

    /// Model-generated content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_content: Option<ServerContent>,

    /// Tool call request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<LiveToolCall>,

    /// Tool call cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_cancellation: Option<LiveToolCallCancellation>,

    /// Disconnect warning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_away: Option<GoAway>,

    /// Token usage metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// Setup complete confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupComplete {}

/// Server content (model response).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerContent {
    /// Model turn content
    pub model_turn: ModelTurn,

    /// Whether this turn is complete
    pub turn_complete: bool,

    /// Whether this was interrupted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupted: Option<bool>,
}

/// Model turn content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTurn {
    /// Content parts
    pub parts: Vec<Part>,
}

/// Tool call request from model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveToolCall {
    pub function_calls: Vec<FunctionCall>,
}

/// Function call from model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

/// Tool call cancellation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveToolCallCancellation {
    pub ids: Vec<String>,
}

/// Server disconnect warning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoAway {
    pub reason: String,
}

/// Token usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Tokens in the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<u32>,

    /// Tokens in the candidates (responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<u32>,

    /// Total tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<u32>,
}

//
// ─── HELPER IMPLEMENTATIONS ─────────────────────────────────────────────────
//

impl Part {
    /// Create a text part.
    pub fn text(text: impl Into<String>) -> Self {
        Part::Text(TextPart { text: text.into() })
    }

    /// Extract text from a part, if it contains text.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Part::Text(TextPart { text }) => Some(text),
            _ => None,
        }
    }
}

impl ServerMessage {
    /// Check if this is a setup complete message.
    pub fn is_setup_complete(&self) -> bool {
        self.setup_complete.is_some()
    }

    /// Check if this is a server content message.
    ///
    /// Not yet used - reserved for future features.
    #[allow(dead_code)]
    pub fn is_server_content(&self) -> bool {
        self.server_content.is_some()
    }

    /// Check if this is a tool call message.
    ///
    /// Not yet used - reserved for future tool calling feature.
    #[allow(dead_code)]
    pub fn is_tool_call(&self) -> bool {
        self.tool_call.is_some()
    }

    /// Check if this is a go away (disconnect) message.
    pub fn is_go_away(&self) -> bool {
        self.go_away.is_some()
    }

    /// Extract text from server content, if present.
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
    pub fn is_turn_complete(&self) -> bool {
        self.server_content
            .as_ref()
            .map(|content| content.turn_complete)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_message_serialization() {
        let msg = SetupMessage {
            setup: SetupConfig {
                model: "models/gemini-2.0-flash-exp".to_string(),
                generation_config: Some(GenerationConfig {
                    temperature: Some(1.0),
                    max_output_tokens: Some(100),
                    ..Default::default()
                }),
                system_instruction: None,
                tools: None,
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"model\":\"models/gemini-2.0-flash-exp\""));
        assert!(json.contains("\"temperature\":1.0"));
    }

    #[test]
    fn test_client_content_message_serialization() {
        let msg = ClientContentMessage {
            client_content: ClientContent {
                turns: vec![Turn {
                    role: "user".to_string(),
                    parts: vec![Part::text("Hello, how are you?")],
                }],
                turn_complete: true,
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Hello, how are you?\""));
        assert!(json.contains("\"turnComplete\":true"));
    }

    #[test]
    fn test_server_message_deserialization_setup_complete() {
        let json = r#"{"setupComplete": {}}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_setup_complete());
        assert!(!msg.is_server_content());
    }

    #[test]
    fn test_server_message_deserialization_content() {
        let json = r#"{
            "serverContent": {
                "modelTurn": {
                    "parts": [{"text": "I'm doing well, thank you!"}]
                },
                "turnComplete": true
            },
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 20,
                "totalTokenCount": 30
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_server_content());
        assert!(msg.is_turn_complete());
        assert_eq!(msg.extract_text().unwrap(), "I'm doing well, thank you!");

        let usage = msg.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, Some(10));
        assert_eq!(usage.candidates_token_count, Some(20));
        assert_eq!(usage.total_token_count, Some(30));
    }

    #[test]
    fn test_part_text_helper() {
        let part = Part::text("Hello");
        assert_eq!(part.as_text(), Some("Hello"));
    }
}
