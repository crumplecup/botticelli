use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, Input as CoreInput, Message as CoreMessage, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_mcp_client::{
    LlmBackend, ToolDefinition, ToolHandler, UnifiedMcpClient,
    tools::{CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool},
};
use botticelli_narrative::FilesystemNarrativeStorage;
use pmcp::{Content, ToolInfo};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Simple LlmBackend adapter for BotticelliDriver.
pub struct TuiLlmBackend {
    driver: Arc<dyn BotticelliDriver>,
}

impl TuiLlmBackend {
    /// Create a new TUI LLM backend.
    pub fn new(driver: Arc<dyn BotticelliDriver>) -> Self {
        Self { driver }
    }
}

impl std::fmt::Debug for TuiLlmBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiLlmBackend")
            .field("driver", &self.driver.model_name())
            .finish()
    }
}

#[async_trait]
impl LlmBackend for TuiLlmBackend {
    async fn generate_with_tools(
        &self,
        messages: &[botticelli_core::Message],
        _tools: &[ToolDefinition],
    ) -> Result<String, Box<dyn std::error::Error>> {
        use botticelli_core::Output;
        use serde_json::json;

        // NOTE: Current architecture limitation - BotticelliDriver trait doesn't expose tool definitions
        // We'll need to refactor this to properly support tool calling
        // For now, we convert the response format so execute_with_tracking can extract tool calls

        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .build()?;

        let response = self.driver.generate(&request).await?;

        // Convert response to format expected by extract_tool_calls
        // The extract_tool_calls function expects JSON with "content" array containing
        // objects with type="tool_use", name, and input fields

        let mut has_tool_calls = false;
        let mut content_array = Vec::new();

        for output in response.outputs() {
            match output {
                Output::Text(text) => {
                    // Include text content
                    content_array.push(json!({
                        "type": "text",
                        "text": text
                    }));
                }
                Output::ToolCalls(calls) => {
                    // Convert to Anthropic tool_use format
                    has_tool_calls = true;
                    for call in calls {
                        content_array.push(json!({
                            "type": "tool_use",
                            "id": call.id(),
                            "name": call.name(),
                            "input": call.arguments()
                        }));
                    }
                }
                _ => {
                    // Other output types not relevant for tool calling
                    tracing::debug!("Skipping non-text/non-tool output");
                }
            }
        }

        if has_tool_calls {
            // Return structured JSON for tool calls
            let result = json!({
                "content": content_array
            });
            Ok(serde_json::to_string(&result)?)
        } else {
            // No tool calls - return text only
            let text = response
                .outputs()
                .iter()
                .find_map(|output| match output {
                    Output::Text(t) => Some(t.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "No response from model".to_string());
            Ok(text)
        }
    }
}

/// Application state for the TUI.
#[derive(Clone)]
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
    /// List of narrative names.
    narrative_list: Vec<String>,
    /// Selected narrative index in browser.
    selected_narrative: Option<usize>,
    /// Selected conversation index in history browser.
    selected_conversation_history: Option<usize>,
    /// Editor content buffer.
    editor_content: String,
    /// MCP client for tool execution (optional).
    mcp_client: Option<Arc<tokio::sync::Mutex<UnifiedMcpClient>>>,
    /// LLM backend for generation (optional).
    llm_backend: Option<Arc<TuiLlmBackend>>,
    /// Channel to send MCP updates to UI thread.
    mcp_channel: Option<tokio::sync::mpsc::UnboundedSender<crate::McpMessage>>,
    /// Conversation storage for persistence.
    storage: crate::storage::ConversationStorage,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("mode", &self.mode)
            .field("current_conversation", &self.current_conversation)
            .field("conversations_count", &self.conversations.len())
            .field("has_mcp_client", &self.mcp_client.is_some())
            .field("has_llm_backend", &self.llm_backend.is_some())
            .finish()
    }
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
    ///
    /// Automatically saves the conversation to disk.
    pub fn update_conversation(&mut self, id: ConversationId, messages: Vec<ChatMessage>) {
        self.conversations.insert(id, messages.clone());

        // Auto-save to disk
        if let Err(e) = self.storage.save(&id, &messages) {
            error!(conversation_id = %id, error = %e, "Failed to save conversation");
        }
    }

    /// Clears the current conversation.
    ///
    /// This removes the conversation from the conversations map and resets the current
    /// conversation ID to None. The input buffer is also cleared. The conversation is also
    /// deleted from disk.
    pub fn clear_conversation(&mut self) {
        if let Some(conv_id) = self.current_conversation {
            self.conversations.remove(&conv_id);

            // Delete from disk
            if let Err(e) = self.storage.delete(&conv_id) {
                error!(conversation_id = %conv_id, error = %e, "Failed to delete conversation from disk");
            }

            info!(conversation_id = %conv_id, "Cleared conversation");
        }
        self.current_conversation = None;
        self.clear_input();
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

    /// Deletes the last character from the input buffer.
    pub fn delete_char(&mut self) {
        self.input_buffer.pop();
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
    }

    /// Gets the list of narratives.
    pub fn narrative_list(&self) -> &[String] {
        &self.narrative_list
    }

    /// Sets the narrative list.
    pub fn set_narrative_list(&mut self, list: Vec<String>) {
        self.narrative_list = list;
    }

    /// Gets the selected narrative index.
    pub fn selected_narrative(&self) -> Option<usize> {
        self.selected_narrative
    }

    /// Sets the selected narrative index.
    pub fn set_selected_narrative(&mut self, idx: Option<usize>) {
        self.selected_narrative = idx;
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

    /// Gets a sorted list of conversation IDs.
    pub fn conversation_ids(&self) -> Vec<ConversationId> {
        let mut ids: Vec<_> = self.conversations.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Gets the selected conversation index in history browser.
    pub fn selected_conversation_history(&self) -> Option<usize> {
        self.selected_conversation_history
    }

    /// Sets the selected conversation index in history browser.
    pub fn set_selected_conversation_history(&mut self, idx: Option<usize>) {
        self.selected_conversation_history = idx;
    }

    /// Moves selection up in conversation history browser.
    pub fn select_previous_conversation_history(&mut self) {
        if let Some(idx) = self.selected_conversation_history {
            if idx > 0 {
                self.selected_conversation_history = Some(idx - 1);
            }
        } else {
            let count = self.conversations.len();
            if count > 0 {
                self.selected_conversation_history = Some(0);
            }
        }
    }

    /// Moves selection down in conversation history browser.
    pub fn select_next_conversation_history(&mut self) {
        let count = self.conversations.len();
        if let Some(idx) = self.selected_conversation_history {
            if idx < count.saturating_sub(1) {
                self.selected_conversation_history = Some(idx + 1);
            }
        } else if count > 0 {
            self.selected_conversation_history = Some(0);
        }
    }

    /// Gets the editor content.
    pub fn editor_content(&self) -> &str {
        &self.editor_content
    }

    /// Sets the editor content.
    pub fn set_editor_content(&mut self, content: String) {
        self.editor_content = content;
    }

    /// Clears the editor content.
    pub fn clear_editor_content(&mut self) {
        self.editor_content.clear();
    }

    /// Gets the current view based on mode.
    pub fn current_view(&self) -> &dyn crate::View {
        match self.mode {
            ViewMode::Chat => &crate::ChatView,
            ViewMode::ConversationHistory => &crate::ConversationHistoryView,
            ViewMode::NarrativeBrowser => &crate::NarrativeBrowserView,
            ViewMode::NarrativeEditor => &crate::NarrativeEditorView,
            ViewMode::Settings => &crate::ChatView, // Placeholder
        }
    }

    /// Handle key event.
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
            warn!(error = %e, "Failed to load conversations, starting fresh");
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
        }
    }
}

/// Simple echo tool for testing.
struct EchoTool;

#[async_trait]
impl ToolHandler for EchoTool {
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> botticelli_mcp_client::McpClientResult<Vec<Content>> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");

        Ok(vec![Content::Text {
            text: format!("Echo: {}", message),
        }])
    }

    fn tool_info(&self) -> ToolInfo {
        ToolInfo::new(
            "echo",
            Some("Echoes back the message you send. Useful for testing.".to_string()),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                },
                "required": ["message"]
            }),
        )
    }
}

impl AppState {
    /// Create AppState with MCP integration enabled.
    ///
    /// Initializes:
    /// - LLM backend with provided driver
    /// - Tool registry with basic tools
    /// - UnifiedMcpClient for orchestration
    pub fn with_mcp_integration(driver: Arc<dyn BotticelliDriver>) -> Self {
        info!("Initializing AppState with MCP integration");

        // Create LLM backend
        let llm_backend = TuiLlmBackend::new(driver);

        // Create MCP client first (so we can populate its internal registry)
        let mut mcp_client = UnifiedMcpClient::builder().max_iterations(10).build();

        // Get mutable reference to internal tool registry
        let registry = mcp_client.internal_registry_mut();

        // Register echo tool for testing
        registry
            .register("echo".to_string(), Arc::new(EchoTool))
            .expect("Failed to register echo tool");

        // Register narrative tools
        let narratives_dir = "narratives";
        let storage = FilesystemNarrativeStorage::new(narratives_dir.into());

        registry
            .register(
                "create_narrative".to_string(),
                Arc::new(CreateNarrativeTool::new(storage.clone())),
            )
            .expect("Failed to register create_narrative tool");

        registry
            .register(
                "validate_narrative".to_string(),
                Arc::new(ValidateNarrativeTool::new(storage.clone())),
            )
            .expect("Failed to register validate_narrative tool");

        registry
            .register(
                "list_narratives".to_string(),
                Arc::new(ListNarrativesTool::new(storage.clone())),
            )
            .expect("Failed to register list_narratives tool");

        registry
            .register(
                "load_narrative".to_string(),
                Arc::new(LoadNarrativeTool::new(storage)),
            )
            .expect("Failed to register load_narrative tool");

        info!(
            tool_count = mcp_client.internal_registry().tool_count(),
            "Internal tools registered in UnifiedMcpClient"
        );

        let mut state = Self::default();
        state.set_llm_backend(llm_backend);
        state.set_mcp_client(mcp_client);

        state
    }

    /// Set the MCP client for tool execution.
    pub fn set_mcp_client(&mut self, client: UnifiedMcpClient) {
        self.mcp_client = Some(Arc::new(tokio::sync::Mutex::new(client)));
    }

    /// Set the LLM backend for generation.
    pub fn set_llm_backend(&mut self, backend: TuiLlmBackend) {
        self.llm_backend = Some(Arc::new(backend));
    }

    /// Set the channel for sending MCP updates.
    pub fn set_mcp_channel(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::McpMessage>) {
        self.mcp_channel = Some(tx);
    }

    /// Check if MCP integration is enabled.
    pub fn has_mcp_integration(&self) -> bool {
        self.mcp_client.is_some() && self.llm_backend.is_some()
    }

    /// Handle MCP execution update from async task.
    pub fn handle_mcp_update(&mut self, update: crate::McpUpdate) -> crate::TuiResult<()> {
        use crate::ChatMessage;

        info!(
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
            info!("Removed thinking indicator");
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

        error!(
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
            info!("Removed thinking indicator");
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
            // Convert existing ChatMessages to core Messages, then add new user message
            let mut core_messages: Vec<CoreMessage> = messages
                .iter()
                .filter_map(|msg| match msg {
                    ChatMessage::User { content } => Some(
                        CoreMessage::builder()
                            .role(Role::User)
                            .content(vec![CoreInput::Text(content.clone())])
                            .build()
                            .ok()?,
                    ),
                    ChatMessage::Assistant { content } => Some(
                        CoreMessage::builder()
                            .role(Role::Assistant)
                            .content(vec![CoreInput::Text(content.clone())])
                            .build()
                            .ok()?,
                    ),
                    _ => None, // Skip tool calls/results/thinking for now
                })
                .collect();

            // Add the new user message to core_messages
            core_messages.push(
                CoreMessage::builder()
                    .role(Role::User)
                    .content(vec![CoreInput::Text(user_message.clone())])
                    .build()
                    .expect("Valid user message"),
            );

            // Show user message + thinking indicator immediately for UI feedback
            let mut updated_messages = messages.clone();
            updated_messages.push(ChatMessage::user(user_message.clone()));
            updated_messages.push(ChatMessage::thinking("Thinking...".to_string()));
            self.update_conversation(conv_id, updated_messages);

            // Execute with MCP client
            if let (Some(mcp_client), Some(llm_backend), Some(tx)) =
                (&self.mcp_client, &self.llm_backend, &self.mcp_channel)
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
                            info!(
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
                                error!(error = %e, "Failed to send MCP update to UI");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "MCP execution failed");

                            // Send error to UI thread
                            if let Err(send_err) =
                                tx.send(crate::McpMessage::Error(crate::McpConversationError {
                                    conversation_id: conv_id,
                                    user_message: user_msg,
                                    error: format!("{}", e),
                                }))
                            {
                                error!(error = %send_err, "Failed to send MCP error to UI");
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
