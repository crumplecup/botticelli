// Chat tab implementation

use botticelli_core::{Input, Message, Role};
use chrono::{DateTime, Utc};

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

#[cfg(feature = "tui")]
use crate::tui::widgets::ChatInput;

/// A displayed message with metadata
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    /// The message role
    pub role: Role,
    /// The message content (simplified text representation)
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl DisplayMessage {
    /// Creates a new display message
    pub fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: Utc::now(),
        }
    }

    /// Creates from a Message
    pub fn from_message(message: &Message) -> Self {
        let content = message
            .content()
            .iter()
            .map(|input| match input {
                Input::Text(text) => text.clone(),
                Input::Image { .. } => "[Image]".to_string(),
                Input::Audio { .. } => "[Audio]".to_string(),
                Input::Video { .. } => "[Video]".to_string(),
                Input::Document { .. } => "[Document]".to_string(),
                Input::Table { .. } => "[Table]".to_string(),
                Input::BotCommand { .. } => "[BotCommand]".to_string(),
                Input::Narrative { .. } => "[Narrative]".to_string(),
                Input::ToolCall { .. } => "[ToolCall]".to_string(),
                Input::ToolResult { .. } => "[ToolResult]".to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        Self::new(*message.role(), content)
    }
}

/// State for the Chat tab
#[derive(Debug)]
pub struct ChatTab {
    /// Message history
    messages: Vec<DisplayMessage>,

    /// List state for scrolling
    #[cfg(feature = "tui")]
    list_state: ListState,

    /// Chat input widget
    #[cfg(feature = "tui")]
    input: ChatInput,

    /// Whether input is focused
    input_focused: bool,
}

impl ChatTab {
    /// Creates a new chat tab
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            #[cfg(feature = "tui")]
            list_state: ListState::default(),
            #[cfg(feature = "tui")]
            input: ChatInput::new(),
            input_focused: true,
        }
    }

    /// Adds a message to the history
    pub fn add_message(&mut self, message: DisplayMessage) {
        self.messages.push(message);

        #[cfg(feature = "tui")]
        {
            // Scroll to bottom
            if !self.messages.is_empty() {
                self.list_state.select(Some(self.messages.len() - 1));
            }
        }
    }

    /// Sends a user message
    pub fn send_message(&mut self, content: String) {
        if content.trim().is_empty() {
            return;
        }

        let message = DisplayMessage::new(Role::User, content);
        self.add_message(message);

        // TODO: Trigger LLM response
        // For now, just add a placeholder response
        let response = DisplayMessage::new(
            Role::Assistant,
            "Chat integration coming soon...".to_string(),
        );
        self.add_message(response);
    }

    /// Clears the conversation
    pub fn clear_conversation(&mut self) {
        self.messages.clear();
        #[cfg(feature = "tui")]
        {
            self.list_state.select(None);
        }
    }

    /// Scrolls up in the message list
    #[cfg(feature = "tui")]
    pub fn scroll_up(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        let selected = self
            .list_state
            .selected()
            .unwrap_or(self.messages.len() - 1);
        if selected > 0 {
            self.list_state.select(Some(selected - 1));
        }
    }

    /// Scrolls down in the message list
    #[cfg(feature = "tui")]
    pub fn scroll_down(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        if selected < self.messages.len() - 1 {
            self.list_state.select(Some(selected + 1));
        }
    }

    /// Handles keyboard input
    #[cfg(feature = "tui")]
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;

        if self.input_focused {
            use crate::tui::widgets::chat_input::InputResult;

            match self.input.handle_input(key) {
                InputResult::SendMessage => {
                    let content = self.input.take_content();
                    self.send_message(content);
                    true
                }
                InputResult::Continue => true,
            }
        } else {
            // Navigation mode
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_up();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_down();
                    true
                }
                KeyCode::Char('i') => {
                    self.input_focused = true;
                    true
                }
                _ => false,
            }
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the chat tab
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Split into message history and input areas
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Message history
                Constraint::Length(5), // Input area
            ])
            .split(area);

        // Render message history
        self.render_messages(chunks[0], buf);

        // Render input
        self.input.render(chunks[1], buf);
    }

    #[cfg(feature = "tui")]
    /// Renders the message history
    fn render_messages(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::widgets::{StatefulWidget, Widget};

        if self.messages.is_empty() {
            let empty_text = Paragraph::new(vec![
                Line::from(""),
                Line::from("No messages yet"),
                Line::from(""),
                Line::from("Type a message below and press Ctrl+Enter to send"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Chat"))
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: false });

            Widget::render(empty_text, area, buf);
            return;
        }

        // Create list items from messages
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .map(|msg| {
                let timestamp = msg.timestamp.format("%H:%M:%S");

                let (role_str, role_color) = match msg.role {
                    Role::User => ("You", Color::Cyan),
                    Role::Assistant => ("Assistant", Color::Green),
                    Role::System => ("System", Color::Yellow),
                };

                let header = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        role_str,
                        Style::default().fg(role_color).add_modifier(Modifier::BOLD),
                    ),
                ]);

                // Wrap content to available width (accounting for borders and padding)
                let max_width = area.width.saturating_sub(4) as usize;
                let content_lines: Vec<Line> = msg
                    .content
                    .lines()
                    .flat_map(|line| {
                        if line.is_empty() {
                            vec![Line::from("")]
                        } else {
                            wrap_text(line, max_width)
                        }
                    })
                    .collect();

                let mut lines = vec![header];
                lines.extend(content_lines);
                lines.push(Line::from(""));

                ListItem::new(Text::from(lines))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Chat ({} messages)", self.messages.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}

impl Default for ChatTab {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tui")]
/// Wraps text to fit within a maximum width, breaking on word boundaries
fn wrap_text(text: &str, max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 {
        return vec![Line::from(text.to_string())];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();

        // If adding this word would exceed max_width, start a new line
        if current_width + word_len + 1 > max_width && !current_line.is_empty() {
            lines.push(Line::from(current_line.clone()));
            current_line.clear();
            current_width = 0;
        }

        // Add word to current line
        if !current_line.is_empty() {
            current_line.push(' ');
            current_width += 1;
        }
        current_line.push_str(word);
        current_width += word_len;
    }

    // Add the last line if not empty
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    // Return at least one line
    if lines.is_empty() {
        vec![Line::from("")]
    } else {
        lines
    }
}
