//! Conversation modeling for multi-turn LLM interactions.
//!
//! This module provides clean abstractions for managing multi-turn conversations
//! with LLMs, including tool calls and results.

use botticelli_core::ToolCall;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A multi-turn conversation session.
#[derive(Debug, Clone)]
pub struct ConversationSession {
    /// Unique session ID
    pub id: String,

    /// System prompt (context for all turns)
    pub system_prompt: String,

    /// Conversation turns in order
    pub turns: Vec<ConversationTurn>,

    /// Current session state
    pub state: SessionState,

    /// Maximum turns allowed (prevent infinite loops)
    pub max_turns: usize,
}

impl ConversationSession {
    /// Create a new conversation session.
    pub fn new(system_prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            system_prompt: system_prompt.into(),
            turns: Vec::new(),
            state: SessionState::Active,
            max_turns: 50,
        }
    }

    /// Add a turn to the conversation.
    pub fn add_turn(&mut self, turn: ConversationTurn) {
        self.turns.push(turn);

        if self.turns.len() >= self.max_turns {
            self.state = SessionState::MaxTurnsExceeded;
        }
    }

    /// Get the number of turns.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Check if session is still active.
    pub fn is_active(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
}

/// A single turn in the conversation.
///
/// Each turn represents one discrete action:
/// - User sends a message
/// - Assistant responds with text
/// - Assistant calls tools
/// - Tool results are provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationTurn {
    /// User sent a message
    UserMessage {
        /// Message content
        content: String,
        /// Optional attachments (images, files, etc.)
        #[serde(skip_serializing_if = "Option::is_none")]
        attachments: Option<Vec<Attachment>>,
    },

    /// Assistant responded with text content
    AssistantMessage {
        /// Response content
        content: String,
    },

    /// Assistant called one or more tools
    AssistantToolCalls {
        /// The tool calls being made
        calls: Vec<ToolCall>,

        /// Optional thinking/reasoning text before tool use
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },

    /// Results from tool execution
    ToolResults {
        /// Results indexed by tool_call_id
        results: Vec<ToolResult>,
    },
}

/// Result from executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this responds to
    pub tool_call_id: String,

    /// Output from the tool
    pub output: serde_json::Value,

    /// Whether this was an error
    pub is_error: bool,

    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Attachment to a user message (image, file, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// MIME type
    pub mime_type: String,
    /// Binary data
    pub data: Vec<u8>,
}

/// State of a conversation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is active and accepting turns
    Active,

    /// Session completed successfully
    Completed,

    /// Session failed with error
    Failed,

    /// Session exceeded maximum turns
    MaxTurnsExceeded,
}
