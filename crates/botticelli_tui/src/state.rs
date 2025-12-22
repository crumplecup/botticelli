use std::sync::Arc;

use botticelli_interface::ChatHost;
use derive_getters::Getters;
use derive_setters::Setters;
use uuid::Uuid;

/// Conversation identifier.
pub type ConversationId = Uuid;

/// Narrative identifier.
pub type NarrativeId = Uuid;

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
