use crate::{ConversationId, NarrativeId, ViewMode};

/// Commands that can be executed in the TUI.
#[derive(Debug, Clone)]
pub enum Command {
    /// Send a message in the current conversation.
    SendMessage(String),
    /// Switch to a different view mode.
    SwitchMode(ViewMode),
    /// Create a new conversation.
    NewConversation,
    /// Load an existing conversation.
    LoadConversation(ConversationId),
    /// Create a new narrative.
    NewNarrative,
    /// Load an existing narrative.
    LoadNarrative(NarrativeId),
    /// Save current narrative.
    SaveNarrative,
    /// Quit the application.
    Quit,
}
