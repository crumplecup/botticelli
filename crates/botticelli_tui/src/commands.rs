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
    /// Navigate up in lists.
    NavigateUp,
    /// Navigate down in lists.
    NavigateDown,
    /// Select the current item in a list.
    SelectNarrative,
    /// Quit the application.
    Quit,
}
