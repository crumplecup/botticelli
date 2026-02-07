//! Message types for conversation history.

use crate::{Input, Role};
use rmcp::tool;
use serde::{Deserialize, Serialize};

/// A multimodal message in a conversation.
///
/// # Examples
///
/// ```
/// use botticelli_core::{Message, Role, Input};
///
/// let message = Message::new(Role::User, vec![Input::Text("Hello!".to_string())]);
///
/// assert_eq!(*message.role(), Role::User);
/// assert_eq!(message.content().len(), 1);
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    derive_getters::Getters,
    derive_builder::Builder,
    derive_new::new,
    elicitation::Elicit,
)]
pub struct Message {
    /// The role of the message sender
    role: Role,
    /// The content of the message (can be multimodal)
    content: Vec<Input>,
}

impl Message {
    /// Returns a builder for constructing a Message.
    #[tool]
    pub fn builder() -> MessageBuilder {
        MessageBuilder::default()
    }
}
