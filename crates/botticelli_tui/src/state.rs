use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use botticelli_core::{GenerateRequest, Input as CoreInput, Message as CoreMessage, Role};
use botticelli_interface::BotticelliDriver;
use botticelli_mcp_client::{
    LlmBackend, ToolDefinition, ToolHandler, ToolRegistry, UnifiedMcpClient,
    tools::{CreateNarrativeTool, ListNarrativesTool, LoadNarrativeTool, ValidateNarrativeTool},
};
use pmcp::{Content, ToolInfo};
use tracing::{error, info};
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
        // Simple implementation: just generate without tool support for now
        // TODO: Add proper tool schema conversion
        let request = GenerateRequest::builder()
            .messages(messages.to_vec())
            .build()?;

        let response = self.driver.generate(&request).await?;

        // Extract text from first output
        let text = response
            .outputs()
            .first()
            .map(|output| match output {
                botticelli_core::Output::Text(t) => t.clone(),
                botticelli_core::Output::ToolCalls(_) => {
                    "Tool calls not yet supported in TUI".to_string()
                }
                _ => "Unsupported output type in TUI".to_string(),
            })
            .unwrap_or_else(|| "No response from model".to_string());

        Ok(text)
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
    /// Editor content buffer.
    editor_content: String,
    /// MCP client for tool execution (optional).
    mcp_client: Option<Arc<tokio::sync::Mutex<UnifiedMcpClient>>>,
    /// LLM backend for generation (optional).
    llm_backend: Option<Arc<TuiLlmBackend>>,
    /// Channel to send MCP updates to UI thread.
    mcp_channel: Option<tokio::sync::mpsc::UnboundedSender<crate::McpUpdate>>,
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
                        if !self.input_buffer.is_empty() {
                            let user_message = self.input_buffer.clone();
                            self.clear_input();

                            // Add user message to conversation
                            let conv_id = self.current_conversation.unwrap_or_else(|| {
                                let id = Uuid::new_v4();
                                self.current_conversation = Some(id);
                                id
                            });

                            let mut messages = self
                                .conversation_messages(&conv_id)
                                .cloned()
                                .unwrap_or_default();
                            messages.push(ChatMessage::user(user_message.clone()));

                            // Execute with MCP if available
                            if self.has_mcp_integration() {
                                // Convert ChatMessages to core Messages for LLM
                                let core_messages: Vec<CoreMessage> = messages
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

                                // Execute with MCP client
                                if let (Some(mcp_client), Some(llm_backend), Some(tx)) =
                                    (&self.mcp_client, &self.llm_backend, &self.mcp_channel)
                                {
                                    let client = mcp_client.clone();
                                    let backend = llm_backend.clone();
                                    let tx = tx.clone();
                                    let conv_id = conv_id;

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

                                                // Send result to UI thread
                                                if let Err(e) = tx.send(crate::McpUpdate {
                                                    conversation_id: conv_id,
                                                    result,
                                                }) {
                                                    error!(error = %e, "Failed to send MCP update to UI");
                                                }
                                            }
                                            Err(e) => {
                                                error!(error = %e, "MCP execution failed");
                                            }
                                        }
                                    });
                                } else {
                                    // Fallback: Simple LLM generation without MCP
                                    messages.push(ChatMessage::assistant(
                                        "MCP integration not fully initialized".to_string(),
                                    ));
                                }
                            } else {
                                // No MCP - add placeholder response
                                messages.push(ChatMessage::assistant(
                                    "LLM integration not enabled. Set up Anthropic API key to use chat.".to_string()
                                ));
                            }

                            self.update_conversation(conv_id, messages);
                        }
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
            narrative_list: Vec::new(),
            selected_narrative: None,
            editor_content: String::new(),
            mcp_client: None,
            llm_backend: None,
            mcp_channel: None,
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

        // Create tool registry and register tools
        let mut registry = ToolRegistry::new();

        // Register echo tool for testing
        registry
            .register("echo".to_string(), Arc::new(EchoTool))
            .expect("Failed to register echo tool");

        // Register narrative tools
        let narratives_dir = "narratives".to_string();

        registry
            .register(
                "create_narrative".to_string(),
                Arc::new(CreateNarrativeTool),
            )
            .expect("Failed to register create_narrative tool");

        registry
            .register(
                "validate_narrative".to_string(),
                Arc::new(ValidateNarrativeTool),
            )
            .expect("Failed to register validate_narrative tool");

        registry
            .register(
                "list_narratives".to_string(),
                Arc::new(ListNarrativesTool::new(&narratives_dir)),
            )
            .expect("Failed to register list_narratives tool");

        registry
            .register(
                "load_narrative".to_string(),
                Arc::new(LoadNarrativeTool::new(&narratives_dir)),
            )
            .expect("Failed to register load_narrative tool");

        info!(tool_count = registry.tool_count(), "Tools registered");

        // Create MCP client with registry
        let mcp_client = UnifiedMcpClient::builder().max_iterations(10).build();

        // Note: We can't add the registry to UnifiedMcpClient yet because it only
        // supports external servers. For now, external tools only.

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
    pub fn set_mcp_channel(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::McpUpdate>) {
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
}

/// A chat message for display.
#[derive(Debug, Clone)]
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
