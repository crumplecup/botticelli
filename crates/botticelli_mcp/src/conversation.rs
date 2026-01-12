//! Conversation modeling for multi-turn LLM interactions.
//!
//! This module provides clean abstractions for managing multi-turn conversations
//! with LLMs, including tool calls and results.

use botticelli_core::{ToolCall, ToolResult};
use serde::{Deserialize, Serialize};
use tracing::instrument;
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
    #[instrument(skip(system_prompt), fields(session_id, system_prompt_len))]
    pub fn new(system_prompt: impl Into<String>) -> Self {
        let system_prompt = system_prompt.into();
        let session_id = Uuid::new_v4().to_string();
        tracing::Span::current().record("session_id", &session_id);
        tracing::Span::current().record("system_prompt_len", system_prompt.len());
        
        Self {
            id: session_id,
            system_prompt,
            turns: Vec::new(),
            state: SessionState::Active,
            max_turns: 50,
        }
    }

    /// Add a turn to the conversation.
    #[instrument(skip(self, turn), fields(session_id = %self.id, turn_count = self.turns.len()))]
    pub fn add_turn(&mut self, turn: ConversationTurn) {
        self.turns.push(turn);

        if self.turns.len() >= self.max_turns {
            self.state = SessionState::MaxTurnsExceeded;
        }
    }

    /// Get the number of turns.
    #[instrument(skip(self), fields(session_id = %self.id))]
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Check if session is still active.
    #[instrument(skip(self), fields(session_id = %self.id, state = ?self.state))]
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
///
/// **Note**: This type has been moved to `botticelli_core`.
/// Import from there: `use botticelli_core::ToolResult;`
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
