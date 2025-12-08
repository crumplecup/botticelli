//! Response types for chat interactions.

use serde::{Deserialize, Serialize};

/// Response from the chat interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    /// Simple text response.
    Text(String),

    /// Success message with optional data.
    Success {
        /// Success message.
        message: String,
        /// Optional additional data.
        data: Option<String>,
    },

    /// Error message.
    Error {
        /// Error message.
        message: String,
        /// Optional error details.
        details: Option<String>,
    },

    /// Confirmation prompt (yes/no question).
    Confirmation {
        /// Question to confirm.
        question: String,
        /// Default answer if none provided.
        default: bool,
    },

    /// Information message.
    Info(String),

    /// Warning message.
    Warning(String),
}

impl Response {
    /// Create a text response.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Create a success response.
    pub fn success(message: impl Into<String>) -> Self {
        Self::Success {
            message: message.into(),
            data: None,
        }
    }

    /// Create a success response with data.
    pub fn success_with_data(message: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Success {
            message: message.into(),
            data: Some(data.into()),
        }
    }

    /// Create an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            details: None,
        }
    }

    /// Create an error response with details.
    pub fn error_with_details(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    /// Create a confirmation prompt.
    pub fn confirmation(question: impl Into<String>, default: bool) -> Self {
        Self::Confirmation {
            question: question.into(),
            default,
        }
    }

    /// Create an info response.
    pub fn info(content: impl Into<String>) -> Self {
        Self::Info(content.into())
    }

    /// Create a warning response.
    pub fn warning(content: impl Into<String>) -> Self {
        Self::Warning(content.into())
    }

    /// Check if response is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Check if response is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if response is a confirmation prompt.
    pub fn is_confirmation(&self) -> bool {
        matches!(self, Self::Confirmation { .. })
    }

    /// Extract text content from response.
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Success { message, data } => {
                if let Some(d) = data {
                    format!("{}\n{}", message, d)
                } else {
                    message.clone()
                }
            }
            Self::Error { message, details } => {
                if let Some(d) = details {
                    format!("Error: {}\n{}", message, d)
                } else {
                    format!("Error: {}", message)
                }
            }
            Self::Confirmation { question, default } => {
                format!("{} [{}]", question, if *default { "Y/n" } else { "y/N" })
            }
            Self::Info(s) => format!("Info: {}", s),
            Self::Warning(s) => format!("Warning: {}", s),
        }
    }
}
