//! Message types for chat conversations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A chat message from user or assistant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Message {
    /// Message from the user.
    User {
        /// Message content.
        content: String,
        /// Timestamp when message was sent.
        timestamp: DateTime<Utc>,
    },
    /// Message from the assistant.
    Assistant {
        /// Response content.
        content: String,
        /// Timestamp when response was generated.
        timestamp: DateTime<Utc>,
    },
    /// System message (info, warnings, errors).
    System {
        /// System message content.
        content: String,
        /// Timestamp when system message was generated.
        timestamp: DateTime<Utc>,
    },
}

impl Message {
    /// Create a user message with current timestamp.
    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create an assistant message with current timestamp.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a system message with current timestamp.
    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        match self {
            Self::User { content, .. } => content,
            Self::Assistant { content, .. } => content,
            Self::System { content, .. } => content,
        }
    }

    /// Get the message timestamp.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::User { timestamp, .. } => *timestamp,
            Self::Assistant { timestamp, .. } => *timestamp,
            Self::System { timestamp, .. } => *timestamp,
        }
    }

    /// Check if message is from user.
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User { .. })
    }

    /// Check if message is from assistant.
    pub fn is_assistant(&self) -> bool {
        matches!(self, Self::Assistant { .. })
    }

    /// Check if message is a system message.
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System { .. })
    }

    /// Get the message role as a string.
    pub fn role(&self) -> &str {
        match self {
            Self::User { .. } => "user",
            Self::Assistant { .. } => "assistant",
            Self::System { .. } => "system",
        }
    }
}
