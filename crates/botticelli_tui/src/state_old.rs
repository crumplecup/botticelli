use std::sync::Arc;

use botticelli_interface::ChatHost;
use derive_getters::Getters;
use derive_setters::Setters;

/// Application state for the TUI - thin UI layer over ChatHost trait.
#[derive(Clone, Getters, Setters)]
#[setters(prefix = "with_")]
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
    chat_host: Option<Arc<dyn ChatHost>>,
    /// Channel to send MCP updates to UI thread.
    mcp_channel: Option<tokio::sync::mpsc::UnboundedSender<crate::McpMessage>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("mode", &self.mode)
            .field("has_chat_host", &self.chat_host.is_some())
            .finish()
    }
}

impl AppState {
    /// Creates a new AppState with the given chat host.
    pub fn new(chat_host: Arc<dyn ChatHost>) -> Self {
        Self {
            mode: ViewMode::Chat,
            input_buffer: String::new(),
            narrative_list: Vec::new(),
            selected_narrative: None,
            selected_conversation_history: None,
            editor_content: String::new(),
            chat_host: Some(chat_host),
            mcp_channel: None,
        }
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
}

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
                if !self.input_buffer.is_empty() {
                    self.input_buffer.pop();
                }
                Ok(())
            }
            KeyCode::Enter => {
                // Submit input based on current mode
                match self.mode {
                    ViewMode::Chat => {
                        // Use the dedicated orchestration method
                        let user_message = self.input_buffer.clone();
                        self.send_message_with_orchestration(user_message)?;
                    }
                    ViewMode::ConversationHistory => {
                        // In conversation history, Enter loads conversation
                        // This is handled by SelectNarrative command
                    }
                    ViewMode::NarrativeEditor => {
                        // In editor, Enter adds newline
                        self.append_input("\n");
                    }
                    ViewMode::NarrativeBrowser => {
                        // In browser, Enter selects narrative
                        // This is handled by SelectNarrative command
                    }
                    ViewMode::Settings => {
                        // TODO: Handle settings input
                    }
                }
                Ok(())
            }
            KeyCode::Up => {
                if self.mode == ViewMode::NarrativeBrowser {
                    self.select_previous_narrative();
                }
                Ok(())
            }
            KeyCode::Down => {
                if self.mode == ViewMode::NarrativeBrowser {
                    self.select_next_narrative();
                }
                Ok(())
            }
            KeyCode::Esc => {
                // Switch back to chat view
                self.set_mode(ViewMode::Chat);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Handle mouse event.
    pub fn handle_mouse(&mut self, _mouse: crossterm::event::MouseEvent) -> crate::TuiResult<()> {
        // TODO: Implement mouse handling
        Ok(())
    }

    /// Handle resize event.
    pub fn handle_resize(&mut self, _width: u16, _height: u16) -> crate::TuiResult<()> {
        // TODO: Implement resize handling
        Ok(())
    }

    /// Update state on tick.
    pub fn update(&mut self) -> crate::TuiResult<()> {
        // TODO: Implement periodic updates
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

impl Default for AppState {
    fn default() -> Self {
        // Initialize storage and load conversations
        let storage = crate::storage::ConversationStorage::new()
            .expect("Failed to initialize conversation storage");

        let conversations = storage.load_all().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load conversations, starting fresh");
            HashMap::new()
        });

        Self {
            mode: ViewMode::Chat,
            current_conversation: None,
            current_narrative: None,
            conversations,
            input_buffer: String::new(),
            narrative_list: Vec::new(),
            selected_narrative: None,
            selected_conversation_history: None,
            editor_content: String::new(),
            mcp_client: None,
            llm_backend: None,
            mcp_channel: None,
            storage,
            available_tools: Vec::new(),
        }
    }
}

impl AppState {
    /// Create AppState with MCP integration enabled.
    ///
    /// Initializes:
    /// - LLM backend with provided driver
    /// - Tool registry from provided McpHost
    /// - MCP client for orchestration
    pub fn with_mcp_integration(
        driver: Arc<dyn ToolCalling>,
        mcp_host: botticelli_mcp_client::McpHost,
    ) -> Self {
        tracing::info!("Initializing AppState with MCP integration");

        // Create LLM backend
        let llm_backend = TuiLlmBackend::new(driver);

        // Get tools from the provided MCP host
        let tools = mcp_host.list_all_tools();
        tracing::info!(
            tool_count = tools.len(),
            tools = ?tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
            "Tools available from MCP host"
        );

        let mut state = Self::default();
        state.set_llm_backend(llm_backend);
        state.set_mcp_client(mcp_host);
        state.set_available_tools(tools);

        state
    }

    /// Set the MCP client for tool execution.
    pub fn set_mcp_client(&mut self, client: McpHost) {
        self.mcp_client = Some(Arc::new(tokio::sync::Mutex::new(client)));
    }

    /// Set the channel for sending MCP updates.
    pub fn set_mcp_channel(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::McpMessage>) {
        self.mcp_channel = Some(tx);
    }

    /// Set available tools from MCP.
    pub fn set_available_tools(&mut self, tools: Vec<botticelli_core::ToolDefinition>) {
        tracing::info!(
            tool_count = tools.len(),
            tools = ?tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
            "Setting available tools in AppState"
        );
        self.available_tools = tools;
    }

    /// Get available tools.
    pub fn available_tools(&self) -> &[botticelli_core::ToolDefinition] {
        &self.available_tools
    }

    /// Check if MCP integration is enabled.
    pub fn has_mcp_integration(&self) -> bool {
        self.mcp_client.is_some() && self.llm_backend.is_some()
    }

    /// Handle MCP execution update from async task.
    pub fn handle_mcp_update(&mut self, update: crate::McpUpdate) -> crate::TuiResult<()> {
        use crate::ChatMessage;

        tracing::info!(
            conversation_id = %update.conversation_id,
            iterations = update.result.iterations,
            tool_calls = update.result.tool_calls.len(),
            "Received MCP update"
        );

        // Get or create conversation
        let mut messages = self
            .conversation_messages(&update.conversation_id)
            .cloned()
            .unwrap_or_default();

        // Remove thinking indicator (last message should be "Thinking...")
        if matches!(messages.last(), Some(ChatMessage::Thinking { .. })) {
            messages.pop();
            tracing::info!("Removed thinking indicator");
        }

        // User message should already be there (added by send_message_with_orchestration)
        // If not (e.g., for error recovery), add it
        if !messages.iter().any(
            |msg| matches!(msg, ChatMessage::User { content } if content == &update.user_message),
        ) {
            messages.push(ChatMessage::user(update.user_message));
        }

        // Add tool calls and results
        for tool_call in &update.result.tool_calls {
            messages.push(ChatMessage::tool_call(
                tool_call.tool_name.clone(),
                tool_call.arguments.clone(),
            ));
            messages.push(ChatMessage::tool_result(
                tool_call.tool_name.clone(),
                tool_call.result.clone(),
                tool_call.success,
            ));
        }

        // Add final assistant response
        messages.push(ChatMessage::assistant(update.result.final_response));

        // Update conversation
        self.update_conversation(update.conversation_id, messages);

        Ok(())
    }

    /// Handle MCP execution error.
    ///
    /// Replaces thinking indicator with error message.
    pub fn handle_mcp_error(&mut self, error: crate::McpConversationError) -> crate::TuiResult<()> {
        use crate::ChatMessage;

        tracing::error!(
            conversation_id = %error.conversation_id,
            error = %error.error,
            "Received MCP error"
        );

        // Get or create conversation
        let mut messages = self
            .conversation_messages(&error.conversation_id)
            .cloned()
            .unwrap_or_default();

        // Remove thinking indicator (last message should be "Thinking...")
        if matches!(messages.last(), Some(ChatMessage::Thinking { .. })) {
            messages.pop();
            tracing::info!("Removed thinking indicator");
        }

        // User message should already be there (added by send_message_with_orchestration)
        // If not (e.g., for error recovery), add it
        if !messages.iter().any(
            |msg| matches!(msg, ChatMessage::User { content } if content == &error.user_message),
        ) {
            messages.push(ChatMessage::user(error.user_message));
        }

        // Add error message as assistant response
        messages.push(ChatMessage::assistant(format!(
            "Error during execution: {}",
            error.error
        )));

        // Update conversation
        self.update_conversation(error.conversation_id, messages);

        Ok(())
    }

    /// Send a message with orchestration (tool calling support).
    ///
    /// This method:
    /// 1. Converts conversation history to core Messages
    /// 2. Adds new user message to core Messages
    /// 3. Executes with MCP orchestration (tool calling) in async task
    /// 4. Sends result + user message to UI via mcp_channel
    /// 5. handle_mcp_update adds all messages to conversation atomically
    #[tracing::instrument(skip(self), fields(message_len = user_message.len()))]
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
            // TODO: Implement with ChatSession trait
            // This needs to be rewritten to use session.send_message()
            tracing::warn!("MCP orchestration not yet implemented with trait architecture");
            return Ok(());
        }
            {
                let client = mcp_client.clone();
                let backend = llm_backend.clone();
                let tx = tx.clone();
                let user_msg = user_message.clone();

                // Spawn async task to execute
                tokio::spawn(async move {
                    let mut client_guard = client.lock().await;
                    match client_guard
                        .execute_with_tracking(backend.as_ref(), core_messages)
                        .await
                    {
                        Ok(result) => {
                            tracing::info!(
                                iterations = result.iterations,
                                tool_calls = result.tool_calls.len(),
                                "MCP execution complete - sending to UI"
                            );

                            // Send result to UI thread (including user message)
                            if let Err(e) = tx.send(crate::McpMessage::Update(crate::McpUpdate {
                                conversation_id: conv_id,
                                user_message: user_msg,
                                result,
                            })) {
                                tracing::error!(error = %e, "Failed to send MCP update to UI");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "MCP execution failed");

                            // Send error to UI thread
                            if let Err(send_err) =
                                tx.send(crate::McpMessage::Error(crate::McpConversationError {
                                    conversation_id: conv_id,
                                    user_message: user_msg,
                                    error: format!("{}", e),
                                }))
                            {
                                tracing::error!(error = %send_err, "Failed to send MCP error to UI");
                            }
                        }
                    }
                });
            } else {
                // Fallback: MCP not fully initialized
                // In this case, we need to update conversation immediately
                let mut updated_messages = messages;
                updated_messages.push(ChatMessage::user(user_message));
                updated_messages.push(ChatMessage::assistant(
                    "MCP integration not fully initialized".to_string(),
                ));
                self.update_conversation(conv_id, updated_messages);
            }
        } else {
            // No MCP - add placeholder response immediately
            let mut updated_messages = messages;
            updated_messages.push(ChatMessage::user(user_message));
            updated_messages.push(ChatMessage::assistant(
                "LLM integration not enabled. Set up Anthropic API key to use chat.".to_string(),
            ));
            self.update_conversation(conv_id, updated_messages);
        }

        Ok(())
    }
}

/// A chat message for display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ChatMessage {
    /// User message.
    User {
        /// Message content from the user.
        content: String,
    },
    /// Assistant message.
    Assistant {
        /// Message content from the assistant.
        content: String,
    },
    /// Tool call by the LLM.
    ToolCall {
        /// Name of the tool being called.
        tool_name: String,
        /// JSON arguments for the tool.
        arguments: serde_json::Value,
    },
    /// Tool execution result.
    ToolResult {
        /// Name of the tool that was executed.
        tool_name: String,
        /// Result text from the tool execution.
        result: String,
        /// Whether the tool execution succeeded.
        success: bool,
    },
    /// Thinking/reasoning content.
    Thinking {
        /// Thinking/reasoning text.
        content: String,
    },
}

impl ChatMessage {
    /// Create a new user message.
    pub fn user(content: String) -> Self {
        Self::User { content }
    }

    /// Create a new assistant message.
    pub fn assistant(content: String) -> Self {
        Self::Assistant { content }
    }

    /// Create a new tool call message.
    pub fn tool_call(tool_name: String, arguments: serde_json::Value) -> Self {
        Self::ToolCall {
            tool_name,
            arguments,
        }
    }

    /// Create a new tool result message.
    pub fn tool_result(tool_name: String, result: String, success: bool) -> Self {
        Self::ToolResult {
            tool_name,
            result,
            success,
        }
    }

    /// Create a new thinking message.
    pub fn thinking(content: String) -> Self {
        Self::Thinking { content }
    }

    /// Get message content (for User and Assistant variants).
    pub fn content(&self) -> Option<&str> {
        match self {
            Self::User { content } | Self::Assistant { content } | Self::Thinking { content } => {
                Some(content)
            }
            Self::ToolCall { .. } | Self::ToolResult { .. } => None,
        }
    }

    /// Check if this is a user message.
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User { .. })
    }

    /// Check if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        matches!(self, Self::Assistant { .. })
    }

    /// Check if this is a tool call.
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall { .. })
    }

    /// Check if this is a tool result.
    pub fn is_tool_result(&self) -> bool {
        matches!(self, Self::ToolResult { .. })
    }

    /// Check if this is thinking content.
    pub fn is_thinking(&self) -> bool {
        matches!(self, Self::Thinking { .. })
    }
}

/// Conversation identifier.
pub type ConversationId = Uuid;

/// Narrative identifier.
pub type NarrativeId = Uuid;
