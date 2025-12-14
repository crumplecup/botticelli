//! Chat tab implementation with rich text input.

use crate::tui::{
    events::EventResult,
    widgets::chat_input::{ChatInput, InputResult},
};
use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

/// A chat message with role, content, and metadata.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    fn style(&self) -> Style {
        match self {
            Self::System => Style::default().fg(Color::Yellow),
            Self::User => Style::default().fg(Color::Cyan),
            Self::Assistant => Style::default().fg(Color::Green),
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::User => "You",
            Self::Assistant => "Assistant",
        }
    }
}

/// Chat tab with message history and rich text input.
pub struct ChatTab {
    messages: Vec<ChatMessage>,
    input: ChatInput,
    scroll_offset: usize,
    scrollbar_state: ScrollbarState,
}

impl ChatTab {
    pub fn new() -> Self {
        let mut messages = Vec::new();
        
        // Add welcome message
        messages.push(ChatMessage {
            role: MessageRole::System,
            content: "Welcome! I can help you create narratives, manage bots, and more.\n\nType your message and press Ctrl+Enter to send.".to_string(),
            timestamp: Utc::now(),
        });

        Self {
            messages,
            input: ChatInput::new(),
            scroll_offset: 0,
            scrollbar_state: ScrollbarState::default(),
        }
    }

    /// Handle keyboard input.
    pub fn handle_key(&mut self, event: KeyEvent) -> EventResult {
        // Handle scrolling
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            match event.code {
                KeyCode::Up => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    return EventResult::Handled;
                }
                KeyCode::Down => {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                    return EventResult::Handled;
                }
                KeyCode::Char('l') => {
                    // Clear chat
                    self.messages.clear();
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: "Chat cleared.".to_string(),
                        timestamp: Utc::now(),
                    });
                    self.scroll_offset = 0;
                    return EventResult::Handled;
                }
                _ => {}
            }
        }

        // Handle input
        match self.input.handle_input(event) {
            InputResult::SendMessage => {
                if !self.input.is_empty() {
                    let content = self.input.take_content();
                    self.add_user_message(content);
                    EventResult::Handled
                } else {
                    EventResult::Handled
                }
            }
            InputResult::Continue => EventResult::Handled,
        }
    }

    /// Add a user message and trigger response.
    fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            timestamp: Utc::now(),
        });

        // TODO: In future, trigger actual LLM response
        // For now, echo back
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: format!("Echo: {}", content),
            timestamp: Utc::now(),
        });

        // Scroll to bottom
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    /// Render the chat tab.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Messages
                Constraint::Length(5),   // Input
            ])
            .split(area);

        // Render messages
        self.render_messages(chunks[0], frame.buffer_mut());

        // Render input
        self.input.render(chunks[1], frame.buffer_mut());
    }

    fn render_messages(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Chat History");

        let inner = block.inner(area);
        block.render(area, buf);

        // Build lines for all messages
        let mut lines = Vec::new();
        for msg in &self.messages {
            // Timestamp and role
            let header = format!(
                "[{}] {}:",
                msg.timestamp.format("%H:%M:%S"),
                msg.role.prefix()
            );
            lines.push(Line::from(Span::styled(header, msg.role.style().add_modifier(Modifier::BOLD))));

            // Message content (wrapped)
            for content_line in msg.content.lines() {
                lines.push(Line::from(content_line.to_string()));
            }

            // Blank line between messages
            lines.push(Line::from(""));
        }

        // Calculate scrollbar
        let total_lines = lines.len();
        let visible_lines = inner.height as usize;
        self.scrollbar_state = self.scrollbar_state
            .content_length(total_lines.saturating_sub(visible_lines));

        // Adjust scroll if too high
        if self.scroll_offset + visible_lines > total_lines {
            self.scroll_offset = total_lines.saturating_sub(visible_lines);
        }

        // Render visible portion
        let visible_lines: Vec<_> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(visible_lines)
            .collect();

        let paragraph = Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false });
        paragraph.render(inner, buf);

        // Render scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(area, buf, &mut self.scrollbar_state);
    }
}
