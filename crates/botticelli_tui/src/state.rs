use std::sync::{Arc, Mutex};

use botticelli_interface::{ChatHost, ChatMessage};
use derive_getters::Getters;
use uuid::Uuid;

/// Conversation identifier.
pub type ConversationId = Uuid;

/// Narrative identifier.
pub type NarrativeId = Uuid;

/// Application state for the TUI - thin UI layer over ChatHost trait.
#[derive(Clone, Getters, derive_setters::Setters)]
#[setters(prefix = "with_", borrow_self)]
pub struct AppState {
    /// Active view mode.
    mode: ViewMode,
    /// Input buffer for current view.
    input_buffer: String,
    /// List of narrative names.
    narrative_list: Vec<String>,
    /// Selected narrative index in browser.
    selected_narrative: Option<usize>,
    /// Selected conversation index in history browser.
    selected_conversation_history: Option<usize>,
    /// Editor content buffer.
    editor_content: String,
    /// Chat host providing LLM and MCP integration.
    #[setters(skip)]
    chat_host: Option<Arc<Mutex<dyn ChatHost>>>,
    /// Channel to send MCP updates to UI thread.
    mcp_channel: Option<tokio::sync::mpsc::UnboundedSender<crate::McpMessage>>,
    /// Current conversation ID.
    current_conversation: Option<Uuid>,
    /// All conversations (keyed by ID).
    conversations: std::collections::HashMap<Uuid, Vec<ChatMessage>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("mode", &self.mode)
            .field("has_chat_host", &self.chat_host.is_some())
            .finish()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: ViewMode::Chat,
            input_buffer: String::new(),
            narrative_list: Vec::new(),
            selected_narrative: None,
            selected_conversation_history: None,
            editor_content: String::new(),
            chat_host: None,
            mcp_channel: None,
            current_conversation: None,
            conversations: std::collections::HashMap::new(),
        }
    }
}

impl AppState {
    /// Creates a new AppState with the given chat host.
    pub fn new(chat_host: Arc<Mutex<dyn ChatHost>>) -> Self {
        Self {
            mode: ViewMode::Chat,
            input_buffer: String::new(),
            narrative_list: Vec::new(),
            selected_narrative: None,
            selected_conversation_history: None,
            editor_content: String::new(),
            chat_host: Some(chat_host),
            mcp_channel: None,
            current_conversation: None,
            conversations: std::collections::HashMap::new(),
        }
    }

    /// Creates AppState with MCP integration (compatibility wrapper).
    pub fn with_mcp_integration(
        _driver: impl botticelli_interface::BotticelliDriver,
        mcp_host: Arc<Mutex<dyn ChatHost>>,
    ) -> Self {
        Self::new(mcp_host)
    }

    /// Appends to the input buffer.
    pub fn append_input(&mut self, text: &str) {
        self.input_buffer.push_str(text);
    }

    /// Deletes the last character from the input buffer.
    pub fn delete_char(&mut self) {
        self.input_buffer.pop();
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
    }

    /// Moves selection up in narrative browser.
    pub fn select_previous_narrative(&mut self) {
        if let Some(idx) = self.selected_narrative {
            if idx > 0 {
                self.selected_narrative = Some(idx - 1);
            }
        } else if !self.narrative_list.is_empty() {
            self.selected_narrative = Some(0);
        }
    }

    /// Moves selection down in narrative browser.
    pub fn select_next_narrative(&mut self) {
        if let Some(idx) = self.selected_narrative {
            if idx < self.narrative_list.len().saturating_sub(1) {
                self.selected_narrative = Some(idx + 1);
            }
        } else if !self.narrative_list.is_empty() {
            self.selected_narrative = Some(0);
        }
    }

    /// Clears the editor content.
    pub fn clear_editor_content(&mut self) {
        self.editor_content.clear();
    }

    /// Moves selection up in conversation history browser.
    pub fn select_previous_conversation_history(&mut self) {
        if let Some(idx) = self.selected_conversation_history {
            if idx > 0 {
                self.selected_conversation_history = Some(idx - 1);
            }
        }
    }

    /// Moves selection down in conversation history browser.
    pub fn select_next_conversation_history(&mut self) {
        let max = self.conversations.len().saturating_sub(1);
        if let Some(idx) = self.selected_conversation_history {
            if idx < max {
                self.selected_conversation_history = Some(idx + 1);
            }
        } else if !self.conversations.is_empty() {
            self.selected_conversation_history = Some(0);
        }
    }

    /// Gets list of conversation IDs.
    pub fn conversation_ids(&self) -> Vec<Uuid> {
        self.conversations.keys().copied().collect()
    }

    /// Clears the current conversation.
    pub fn clear_conversation(&mut self) {
        self.current_conversation = None;
    }

    /// Checks if MCP integration is available.
    pub fn has_mcp_integration(&self) -> bool {
        self.chat_host.is_some() && self.mcp_channel.is_some()
    }

    /// Gets messages for a conversation.
    pub fn conversation_messages(&self, id: &Uuid) -> Option<&Vec<ChatMessage>> {
        self.conversations.get(id)
    }

    /// Updates a conversation with new messages.
    pub fn update_conversation(&mut self, id: Uuid, messages: Vec<ChatMessage>) {
        self.conversations.insert(id, messages);
        self.current_conversation = Some(id);
    }

    /// Gets the current conversation messages.
    pub fn current_messages(&self) -> Vec<ChatMessage> {
        self.current_conversation
            .and_then(|id| self.conversations.get(&id))
            .cloned()
            .unwrap_or_default()
    }

    /// Sends a message with full orchestration (tool calling, etc.).
    #[tracing::instrument(skip(self))]
    pub fn send_message_with_orchestration(
        &mut self,
        user_message: String,
    ) -> crate::TuiResult<()> {
        if user_message.is_empty() {
            return Ok(());
        }

        // Clear input buffer
        self.clear_input();

        // Get or create conversation
        let conv_id = self.current_conversation.unwrap_or_else(|| {
            let id = Uuid::new_v4();
            self.current_conversation = Some(id);
            id
        });

        // Get existing conversation history
        let messages = self
            .conversation_messages(&conv_id)
            .cloned()
            .unwrap_or_default();

        // Execute with MCP if available
        if self.has_mcp_integration() {
            let chat_host = self.chat_host.as_ref().unwrap().clone();
            let tx = self.mcp_channel.as_ref().unwrap().clone();
            let user_msg = user_message.clone();

            // Add user message to conversation immediately
            let mut updated_messages = messages.clone();
            updated_messages.push(ChatMessage::user(user_message.clone()));
            self.update_conversation(conv_id, updated_messages);

            // Spawn blocking task to execute via chat host (sync trait)
            tokio::task::spawn_blocking(move || {
                tracing::info!("Processing message with MCP orchestration");
                
                // Use the chat host to send the message
                let response = match chat_host.lock() {
                    Ok(mut host) => host.send_message(user_msg.clone()),
                    Err(e) => {
                        tracing::error!("Failed to lock chat host: {}", e);
                        return;
                    }
                };
                
                match response {
                    Ok(response) => {
                        tracing::debug!(response = %response, "Received LLM response");
                        if let Err(e) = tx.send(crate::McpMessage::Update(crate::McpUpdate {
                            conversation_id: conv_id,
                            user_message: user_msg.clone(),
                            assistant_message: response,
                        })) {
                            tracing::error!(error = %e, "Failed to send MCP update to UI");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, "Failed to process message");
                        if let Err(e) = tx.send(crate::McpMessage::Update(crate::McpUpdate {
                            conversation_id: conv_id,
                            user_message: user_msg.clone(),
                            assistant_message: format!("Error: {}", e),
                        })) {
                            tracing::error!(error = %e, "Failed to send error to UI");
                        }
                    }
                }
            });
        } else {
            // No MCP - add placeholder response immediately
            let mut updated_messages = messages;
            updated_messages.push(ChatMessage::user(user_message));
            updated_messages.push(ChatMessage::assistant(
                "LLM integration not enabled. Set up API key to use chat.".to_string(),
            ));
            self.update_conversation(conv_id, updated_messages);
        }

        Ok(())
    }

    /// Handles keyboard input events.
    #[tracing::instrument(skip(self))]
    pub async fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::TuiResult<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Char(_c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Control key combinations are handled by EventHandler (Ctrl+C for quit)
                Ok(())
            }
            KeyCode::Char(c) => {
                // Regular character input
                self.append_input(&c.to_string());
                Ok(())
            }
            KeyCode::Backspace => {
                // Delete last character
                self.delete_char();
                Ok(())
            }
            KeyCode::Enter => {
                // Submit input based on current mode
                let user_message = self.input_buffer.clone();
                if !user_message.is_empty() {
                    self.send_message_with_orchestration(user_message)?;
                }
                Ok(())
            }
            _ => {
                // Other keys ignored
                Ok(())
            }
        }
    }

    /// Returns the current view command (for command palette).
    pub fn current_view(&self) -> Option<String> {
        // Return None - TUI doesn't use command palette
        None
    }

    /// Handles mouse events.
    #[tracing::instrument(skip(self))]
    pub fn handle_mouse(&mut self, _event: crossterm::event::MouseEvent) -> Result<(), std::io::Error> {
        // Mouse handling can be implemented later
        Ok(())
    }

    /// Handles terminal resize.
    #[tracing::instrument(skip(self))]
    pub fn handle_resize(&mut self, _width: u16, _height: u16) -> Result<(), std::io::Error> {
        // Resize handling can be implemented later
        Ok(())
    }

    /// Updates the state (called periodically).
    #[tracing::instrument(skip(self))]
    pub fn update(&mut self) -> Result<(), std::io::Error> {
        // Periodic updates can be implemented later
        Ok(())
    }

    /// Handles MCP updates.
    #[tracing::instrument(skip(self))]
    pub fn handle_mcp_update(&mut self, _update: crate::McpUpdate) -> Result<(), std::io::Error> {
        // MCP update handling can be implemented later
        Ok(())
    }

    /// Handles MCP errors.
    #[tracing::instrument(skip(self))]
    pub fn handle_mcp_error(&mut self, error: crate::McpConversationError) -> Result<(), std::io::Error> {
        tracing::error!("MCP error: {:?}", error);
        // Could display error in UI
        Ok(())
    }
}

/// View mode for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    /// Chat interface.
    Chat,
    /// Conversation history browser.
    ConversationHistory,
    /// Narrative browser.
    NarrativeBrowser,
    /// Narrative editor.
    NarrativeEditor,
    /// Settings.
    Settings,
}
