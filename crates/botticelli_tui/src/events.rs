//! Event types and handling for the TUI.

/// Update from MCP execution.
#[derive(Debug, Clone)]
pub struct McpUpdate {
    /// Conversation ID this update belongs to
    pub conversation_id: uuid::Uuid,
    /// Original user message that triggered this execution
    pub user_message: String,
    /// Assistant's response
    pub assistant_message: String,
}

/// Error from MCP execution in a conversation.
#[derive(Debug, Clone)]
pub struct McpConversationError {
    /// Conversation ID this error belongs to
    pub conversation_id: uuid::Uuid,
    /// Original user message that triggered this execution
    pub user_message: String,
    /// Error message
    pub error: String,
}

/// Message from MCP execution (success or error).
#[derive(Debug, Clone)]
pub enum McpMessage {
    /// Execution completed successfully.
    Update(McpUpdate),
    /// Execution failed with error.
    Error(McpConversationError),
}

/// TUI events.
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input.
    Key(crossterm::event::KeyEvent),
    /// Mouse input.
    Mouse(crossterm::event::MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
    /// Tick for periodic updates.
    Tick,
    /// Quit signal.
    Quit,
    /// MCP execution completed successfully.
    McpUpdate(McpUpdate),
    /// MCP execution failed.
    McpError(McpConversationError),
}
