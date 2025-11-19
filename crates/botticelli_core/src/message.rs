//! Message types for conversation history.

use crate::{Input, Role};
use serde::{Deserialize, Serialize};

/// A multimodal message in a conversation.
///
/// # Examples
///
/// ```
/// use botticelli_core::{Message, Role, Input};
///
/// let message = Message {
///     role: Role::User,
///     content: vec![Input::Text("Hello!".to_string())],
/// };
///
/// assert_eq!(message.role, Role::User);
/// assert_eq!(message.content.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender
    pub role: Role,
    /// The content of the message (can be multimodal)
    pub content: Vec<Input>,
}
