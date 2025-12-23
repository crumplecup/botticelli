use std::sync::Arc;

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
    chat_host: Option<Arc<tokio::sync::Mutex<dyn ChatHost>>>,
    /// Channel to send MCP updates to UI thread.
    mcp_channel: Option<tokio::sync::mpsc::UnboundedSender<crate::McpMessage>>,
    /// Current conversation ID.
    current_conversation: Option<Uuid>,
    /// All conversations (keyed by ID).
    conversations: std::collections::HashMap<Uuid, Vec<ChatMessage>>,
    /// Bot list for bot management.
    bots: Vec<crate::view::BotInfo>,
    /// Selected bot index.
    selected_bot: Option<usize>,
    /// Database view mode.
    database_view_mode: crate::view::DatabaseViewMode,
    /// Database tables list.
    database_tables: Vec<crate::view::TableInfo>,
    /// Selected table index.
    selected_database_table: Option<usize>,
    /// Database schema for selected table.
    database_schema: Vec<crate::view::ColumnDisplay>,
    /// Database content rows.
    database_content: Vec<crate::view::ContentRow>,
    /// Selected content row.
    selected_database_content_row: Option<usize>,
    /// Database content filter.
    database_filter: crate::view::ContentFilter,
    /// Database connection status.
    database_connected: bool,
    /// Scheduled tasks list.
    schedule_tasks: Vec<crate::view::ScheduledTask>,
    /// Selected task index.
    selected_schedule_task: Option<usize>,
    /// Detail scroll offset for schedule view.
    schedule_detail_scroll: usize,
    /// Dirty flag for rendering optimization.
    /// Set to true when state changes, cleared after render.
    dirty: bool,
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
            bots: Vec::new(),
            selected_bot: None,
            database_view_mode: crate::view::DatabaseViewMode::Tables,
            database_tables: Vec::new(),
            selected_database_table: None,
            database_schema: Vec::new(),
            database_content: Vec::new(),
            selected_database_content_row: None,
            database_filter: crate::view::ContentFilter::default(),
            database_connected: false,
            schedule_tasks: Vec::new(),
            selected_schedule_task: None,
            schedule_detail_scroll: 0,
            dirty: true, // Initial render needed
        }
    }
}

impl AppState {
    /// Creates a new AppState with the given chat host.
    pub fn new(chat_host: Arc<tokio::sync::Mutex<dyn ChatHost>>) -> Self {
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
            bots: Vec::new(),
            selected_bot: None,
            database_view_mode: crate::view::DatabaseViewMode::Tables,
            database_tables: Vec::new(),
            selected_database_table: None,
            database_schema: Vec::new(),
            database_content: Vec::new(),
            selected_database_content_row: None,
            database_filter: crate::view::ContentFilter::default(),
            database_connected: false,
            schedule_tasks: Vec::new(),
            selected_schedule_task: None,
            schedule_detail_scroll: 0,
            dirty: true, // Initial render needed
        }
    }

    /// Checks if the state needs to be rendered.
    pub fn needs_render(&self) -> bool {
        self.dirty
    }

    /// Clears the dirty flag after rendering.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Marks state as dirty (needs render).
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Adds a chat response to the current conversation.
    #[tracing::instrument(skip(self), fields(conversation_id = ?self.current_conversation))]
    /// Adds a user message to the current conversation.
    pub fn add_user_message(&mut self, content: String, conversation_id: Option<Uuid>) {
        tracing::debug!(content = %content, "Adding user message");
        
        // Use provided or create new conversation
        let conversation_id = conversation_id
            .or_else(|| self.current_conversation)
            .unwrap_or_else(Uuid::new_v4);
        
        self.current_conversation = Some(conversation_id);
        
        // Add user message to conversation
        let message = ChatMessage {
            role: "user".to_string(),
            content,
        };
        
        self.conversations
            .entry(conversation_id)
            .or_default()
            .push(message);
        
        self.mark_dirty();
        tracing::debug!("User message added successfully");
    }

    /// Adds an assistant response to the current conversation.
    pub fn add_chat_response(&mut self, response: String) {
        tracing::debug!(response = %response, "Adding chat response");
        
        // Create conversation if it doesn't exist
        let conversation_id = self.current_conversation.get_or_insert_with(Uuid::new_v4);
        
        // Add assistant message to conversation
        let message = ChatMessage {
            role: "assistant".to_string(),
            content: response,
        };
        
        self.conversations
            .entry(*conversation_id)
            .or_default()
            .push(message);
        
        self.mark_dirty();
        tracing::debug!("Chat response added successfully");
    }

    /// Creates AppState with MCP integration (compatibility wrapper).
    pub fn with_mcp_integration(
        _driver: impl botticelli_interface::BotticelliDriver,
        mcp_host: Arc<tokio::sync::Mutex<dyn ChatHost>>,
    ) -> Self {
        Self::new(mcp_host)
    }

    /// Appends to the input buffer.
    #[tracing::instrument(skip(self), fields(buffer_len = self.input_buffer.len()))]
    pub fn append_input(&mut self, text: &str) {
        let start = std::time::Instant::now();
        self.input_buffer.push_str(text);
        self.mark_dirty();
        let elapsed = start.elapsed();
        tracing::trace!(new_len = self.input_buffer.len(), elapsed_us = elapsed.as_micros(), "Appended to input buffer");
    }

    /// Deletes the last character from the input buffer.
    #[tracing::instrument(skip(self), fields(buffer_len = self.input_buffer.len()))]
    pub fn delete_char(&mut self) {
        let start = std::time::Instant::now();
        self.input_buffer.pop();
        self.mark_dirty();
        let elapsed = start.elapsed();
        tracing::trace!(new_len = self.input_buffer.len(), elapsed_us = elapsed.as_micros(), "Deleted char from buffer");
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.mark_dirty();
    }

    /// Moves selection up in narrative browser.
    pub fn select_previous_narrative(&mut self) {
        if let Some(idx) = self.selected_narrative {
            if idx > 0 {
                self.selected_narrative = Some(idx - 1);
                self.mark_dirty();
            }
        } else if !self.narrative_list.is_empty() {
            self.selected_narrative = Some(0);
            self.mark_dirty();
        }
    }

    /// Moves selection down in narrative browser.
    pub fn select_next_narrative(&mut self) {
        if let Some(idx) = self.selected_narrative {
            if idx < self.narrative_list.len().saturating_sub(1) {
                self.selected_narrative = Some(idx + 1);
                self.mark_dirty();
            }
        } else if !self.narrative_list.is_empty() {
            self.selected_narrative = Some(0);
            self.mark_dirty();
        }
    }

    /// Clears the editor content.
    pub fn clear_editor_content(&mut self) {
        self.editor_content.clear();
        self.mark_dirty();
    }

    /// Moves selection up in conversation history browser.
    pub fn select_previous_conversation_history(&mut self) {
        if let Some(idx) = self.selected_conversation_history
            && idx > 0
        {
            self.selected_conversation_history = Some(idx - 1);
            self.mark_dirty();
        }
    }

    /// Moves selection down in conversation history browser.
    pub fn select_next_conversation_history(&mut self) {
        let max = self.conversations.len().saturating_sub(1);
        if let Some(idx) = self.selected_conversation_history {
            if idx < max {
                self.selected_conversation_history = Some(idx + 1);
                self.mark_dirty();
            }
        } else if !self.conversations.is_empty() {
            self.selected_conversation_history = Some(0);
            self.mark_dirty();
        }
    }

    /// Gets list of conversation IDs.
    pub fn conversation_ids(&self) -> Vec<Uuid> {
        self.conversations.keys().copied().collect()
    }

    /// Clears the current conversation.
    pub fn clear_conversation(&mut self) {
        self.current_conversation = None;
        self.mark_dirty();
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
        self.mark_dirty();
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
            tracing::info!("Has MCP integration, proceeding with orchestration");
            let chat_host = self.chat_host.as_ref().unwrap().clone();
            let tx = self.mcp_channel.as_ref().unwrap().clone();
            let user_msg = user_message.clone();

            // Add user message to conversation immediately
            let mut updated_messages = messages.clone();
            updated_messages.push(ChatMessage::user(user_message.clone()));
            self.update_conversation(conv_id, updated_messages);
            tracing::debug!("Updated conversation with user message");

            // Spawn async task for chat host (don't block event loop)
            tracing::info!("Spawning async task for chat host");
            tokio::spawn(async move {
                tracing::info!("Inside async task, acquiring chat host lock");
                
                // Use the chat host to send the message
                let mut host = chat_host.lock().await;
                tracing::info!("Acquired lock, calling send_message");
                let response = host.send_message(user_msg.clone()).await;
                drop(host); // Explicitly drop before processing
                
                tracing::info!("send_message returned, processing response");
                match response {
                    Ok(response) => {
                        tracing::info!(response = %response, "Received LLM response");
                        if let Err(e) = tx.send(crate::McpMessage::Update(crate::McpUpdate {
                            conversation_id: conv_id,
                            user_message: user_msg.clone(),
                            assistant_message: response,
                        })) {
                            tracing::error!(error = %e, "Failed to send MCP update to UI");
                        } else {
                            tracing::info!("Successfully sent MCP update to UI");
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
                tracing::info!("Blocking task completed");
            });
            tracing::info!("Spawned blocking task successfully");
        } else {
            tracing::warn!("No MCP integration available");
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
    #[tracing::instrument(skip(self), fields(key_code = ?key.code, modifiers = ?key.modifiers))]
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::TuiResult<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let start = std::time::Instant::now();
        
        let result = match key.code {
            KeyCode::Char(_c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                tracing::trace!("Control key combination");
                // Control key combinations are handled by EventHandler (Ctrl+C for quit)
                Ok(())
            }
            KeyCode::Char(c) => {
                tracing::trace!(char = %c, "Character input");
                // Regular character input
                self.append_input(&c.to_string());
                Ok(())
            }
            KeyCode::Backspace => {
                tracing::trace!("Backspace");
                // Delete last character
                self.delete_char();
                Ok(())
            }
            KeyCode::Enter => {
                tracing::trace!("Enter key");
                // Submit input based on current mode
                let user_message = self.input_buffer.clone();
                if !user_message.is_empty() {
                    self.send_message_with_orchestration(user_message)?;
                }
                Ok(())
            }
            _ => {
                tracing::trace!("Other key ignored");
                // Other keys ignored
                Ok(())
            }
        };
        
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 1 {
            tracing::warn!(elapsed_ms = elapsed.as_millis(), "handle_key SLOW!");
        } else {
            tracing::trace!(elapsed_us = elapsed.as_micros(), "handle_key completed");
        }
        
        result
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
    /// Bots management.
    Bots,
    /// Database browser.
    Database,
    /// Schedule management.
    Schedule,
}

impl ViewMode {
    /// Returns the next view mode in the cycle.
    pub fn next(&self) -> Self {
        match self {
            Self::Chat => Self::ConversationHistory,
            Self::ConversationHistory => Self::NarrativeBrowser,
            Self::NarrativeBrowser => Self::NarrativeEditor,
            Self::NarrativeEditor => Self::Settings,
            Self::Settings => Self::Bots,
            Self::Bots => Self::Database,
            Self::Database => Self::Schedule,
            Self::Schedule => Self::Chat,
        }
    }

    /// Returns the previous view mode in the cycle.
    pub fn previous(&self) -> Self {
        match self {
            Self::Chat => Self::Schedule,
            Self::ConversationHistory => Self::Chat,
            Self::NarrativeBrowser => Self::ConversationHistory,
            Self::NarrativeEditor => Self::NarrativeBrowser,
            Self::Settings => Self::NarrativeEditor,
            Self::Bots => Self::Settings,
            Self::Database => Self::Bots,
            Self::Schedule => Self::Database,
        }
    }
}
