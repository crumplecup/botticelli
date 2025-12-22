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
    /// Clear the current conversation.
    ClearConversation,
    /// Start interactive narrative elicitation.
    StartNarrativeElicitation,
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
    /// Append character to input buffer.
    AppendChar(char),
    /// Delete last character from input buffer.
    DeleteChar,
    /// Quit the application.
    Quit,
    /// Show database tables view.
    DatabaseShowTables,
    /// Show database schema view.
    DatabaseShowSchema,
    /// Show database content view.
    DatabaseShowContent,
    /// Cycle database content filter.
    DatabaseCycleFilter,
    /// Load tables from database.
    DatabaseLoadTables,
    /// Select current database table.
    DatabaseSelectTable,
}
