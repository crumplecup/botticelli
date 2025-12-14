// Chat input widget using tui-textarea

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
};

#[cfg(feature = "tui")]
use tui_textarea::TextArea;

#[cfg(feature = "tui")]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(feature = "tui")]
use ratatui::widgets::{Block, Borders};

/// Rich text input widget for chat
pub struct ChatInput {
    #[cfg(feature = "tui")]
    textarea: TextArea<'static>,
}

/// Result of handling input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputResult {
    /// Message should be sent
    SendMessage,
    /// Continue editing
    Continue,
}

impl ChatInput {
    /// Creates a new chat input widget
    pub fn new() -> Self {
        #[cfg(feature = "tui")]
        {
            let mut textarea = TextArea::default();
            textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Input (Ctrl+Enter to send, Ctrl+Z to undo)"),
            );
            textarea.set_placeholder_text("Type your message...");

            Self { textarea }
        }

        #[cfg(not(feature = "tui"))]
        {
            Self {}
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the chat input widget
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::Widget;
        Widget::render(&self.textarea, area, buf);
    }

    #[cfg(feature = "tui")]
    /// Handles keyboard input
    pub fn handle_input(&mut self, event: KeyEvent) -> InputResult {
        // Ctrl+Enter sends message
        if event.code == KeyCode::Enter && event.modifiers.contains(KeyModifiers::CONTROL) {
            return InputResult::SendMessage;
        }

        // Regular input
        self.textarea.input(event);
        InputResult::Continue
    }

    #[cfg(feature = "tui")]
    /// Takes the current content and clears the input
    pub fn take_content(&mut self) -> String {
        let content = self.textarea.lines().join("\n");
        self.textarea.select_all();
        self.textarea.cut();
        content
    }

    #[cfg(feature = "tui")]
    /// Checks if the input is empty
    pub fn is_empty(&self) -> bool {
        self.textarea.lines().iter().all(|l| l.trim().is_empty())
    }
}

impl Default for ChatInput {
    fn default() -> Self {
        Self::new()
    }
}
