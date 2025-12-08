//! User input types.

use serde::{Deserialize, Serialize};

/// User input in the chat interface.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserInput {
    /// Text command or message.
    Text(String),

    /// Confirmation response (yes/no).
    Confirmation(bool),

    /// File path input.
    FilePath(String),

    /// Exit/quit command.
    Exit,
}

impl UserInput {
    /// Create a text input.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Create a confirmation input.
    pub fn confirmation(value: bool) -> Self {
        Self::Confirmation(value)
    }

    /// Create a file path input.
    pub fn file_path(path: impl Into<String>) -> Self {
        Self::FilePath(path.into())
    }

    /// Create an exit input.
    pub fn exit() -> Self {
        Self::Exit
    }

    /// Check if input is text.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Check if input is confirmation.
    pub fn is_confirmation(&self) -> bool {
        matches!(self, Self::Confirmation(_))
    }

    /// Check if input is file path.
    pub fn is_file_path(&self) -> bool {
        matches!(self, Self::FilePath(_))
    }

    /// Check if input is exit.
    pub fn is_exit(&self) -> bool {
        matches!(self, Self::Exit)
    }

    /// Get text content if available.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get confirmation value if available.
    pub fn as_confirmation(&self) -> Option<bool> {
        match self {
            Self::Confirmation(b) => Some(*b),
            _ => None,
        }
    }

    /// Get file path if available.
    pub fn as_file_path(&self) -> Option<&str> {
        match self {
            Self::FilePath(p) => Some(p),
            _ => None,
        }
    }
}

impl std::fmt::Display for UserInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(s) => write!(f, "{}", s),
            Self::Confirmation(b) => write!(f, "{}", if *b { "yes" } else { "no" }),
            Self::FilePath(p) => write!(f, "{}", p),
            Self::Exit => write!(f, "exit"),
        }
    }
}
