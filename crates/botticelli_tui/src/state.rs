use std::collections::HashMap;

use uuid::Uuid;

/// Application state for the TUI.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Active view mode.
    mode: ViewMode,
    /// Current conversation (if any).
    current_conversation: Option<ConversationId>,
    /// Current narrative (if any).
    current_narrative: Option<NarrativeId>,
    /// Conversation history cache.
    conversations: HashMap<ConversationId, Vec<ChatMessage>>,
    /// Input buffer for current view.
    input_buffer: String,
}

impl AppState {
    /// Gets the current view mode.
    pub fn mode(&self) -> ViewMode {
        self.mode
    }

    /// Sets the view mode.
    pub fn set_mode(&mut self, mode: ViewMode) {
        self.mode = mode;
    }

    /// Gets the current conversation ID.
    pub fn current_conversation(&self) -> Option<ConversationId> {
        self.current_conversation
    }

    /// Sets the current conversation.
    pub fn set_current_conversation(&mut self, id: Option<ConversationId>) {
        self.current_conversation = id;
    }

    /// Gets the current narrative ID.
    pub fn current_narrative(&self) -> Option<NarrativeId> {
        self.current_narrative
    }

    /// Sets the current narrative.
    pub fn set_current_narrative(&mut self, id: Option<NarrativeId>) {
        self.current_narrative = id;
    }

    /// Gets messages for a conversation.
    pub fn conversation_messages(&self, id: &ConversationId) -> Option<&Vec<ChatMessage>> {
        self.conversations.get(id)
    }

    /// Updates messages for a conversation.
    pub fn update_conversation(&mut self, id: ConversationId, messages: Vec<ChatMessage>) {
        self.conversations.insert(id, messages);
    }

    /// Gets the input buffer.
    pub fn input_buffer(&self) -> &str {
        &self.input_buffer
    }

    /// Sets the input buffer.
    pub fn set_input_buffer(&mut self, buffer: String) {
        self.input_buffer = buffer;
    }

    /// Appends to the input buffer.
    pub fn append_input(&mut self, text: &str) {
        self.input_buffer.push_str(text);
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
    }
}

/// View mode for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    /// Chat interface.
    Chat,
    /// Narrative browser.
    NarrativeBrowser,
    /// Narrative editor.
    NarrativeEditor,
    /// Settings.
    Settings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: ViewMode::Chat,
            current_conversation: None,
            current_narrative: None,
            conversations: HashMap::new(),
            input_buffer: String::new(),
        }
    }
}

/// A chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Message content.
    pub content: String,
    /// Whether this is from the user.
    pub is_user: bool,
}

/// Conversation identifier.
pub type ConversationId = Uuid;

/// Narrative identifier.
pub type NarrativeId = Uuid;
