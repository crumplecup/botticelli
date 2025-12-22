// Chat tab implementation

use botticelli_core::{Input, Message, Role};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::services::ServiceContainer;

#[cfg(feature = "tui")]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListState, Paragraph, Wrap},
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
    
    /// Services for LLM and tool execution
    services: Option<Arc<ServiceContainer>>,
    
    /// Channel for receiving LLM responses
    response_rx: Option<mpsc::UnboundedReceiver<String>>,
    
    /// Channel for sending LLM responses
    response_tx: Option<mpsc::UnboundedSender<String>>,
}

impl ChatTab {
    /// Creates a new chat tab
    pub fn new() -> Self {
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        
        Self {
            messages: Vec::new(),
            #[cfg(feature = "tui")]
            list_state: ListState::default(),
            #[cfg(feature = "tui")]
            input: ChatInput::new(),
            input_focused: true,
            services: None,
            response_rx: Some(response_rx),
            response_tx: Some(response_tx),
        }
    }
    
    /// Sets the services for LLM integration
    pub fn set_services(&mut self, services: Arc<ServiceContainer>) {
        self.services = Some(services);
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

    /// Sends a user message and triggers LLM response
    #[tracing::instrument(skip(self))]
    pub fn send_message(&mut self, content: String) {
        if content.trim().is_empty() {
            return;
        }

        tracing::info!(content_len = content.len(), "Sending user message");
        let message = DisplayMessage::new(Role::User, content.clone());
        self.add_message(message);

        // Get services if available
        let Some(services) = self.services.clone() else {
            tracing::warn!("Services not configured, cannot send message to LLM");
            let error_msg = DisplayMessage::new(Role::Assistant, "LLM not configured".to_string());
            self.add_message(error_msg);
            return;
        };
        
        // Get response sender
        let Some(response_tx) = self.response_tx.clone() else {
            tracing::error!("Response channel not configured");
            return;
        };

        tracing::debug!("Spawning async LLM response handler");
        // Clone messages for async task
        let messages_for_task = self.messages.clone();
        
        tokio::spawn(async move {
            tracing::info!("LLM response handler started");
            match Self::handle_llm_response(content, messages_for_task, services).await {
                Ok(response) => {
                    tracing::info!(response_len = response.len(), "Got LLM response");
                    if let Err(e) = response_tx.send(response) {
                        tracing::error!(error = ?e, "Failed to send response to UI");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get LLM response");
                    let _ = response_tx.send(format!("Error: {}", e));
                }
            }
            tracing::info!("LLM response handler completed");
        });
    }
    
    /// Poll for LLM responses and add to message history
    #[tracing::instrument(skip(self))]
    pub fn poll_responses(&mut self) {
        if let Some(rx) = &mut self.response_rx {
            while let Ok(response) = rx.try_recv() {
                tracing::info!(response_len = response.len(), "Received LLM response");
                let message = DisplayMessage::new(Role::Assistant, response);
                self.add_message(message);
            }
        }
    }
    
    #[tracing::instrument(skip(_messages, services))]
    async fn handle_llm_response(
        user_content: String,
        _messages: Vec<DisplayMessage>,
        services: Arc<ServiceContainer>,
    ) -> ChatResult<()> {
        tracing::info!("Starting LLM response generation");
        
        // Build conversation message
        let user_message = Message::builder()
            .role(Role::User)
            .content(vec![Input::Text(user_content.clone())])
            .build()
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to build user message");
                ChatError::new(ChatErrorKind::InvalidInput(format!("Failed to build message: {}", e)))
            })?;
        
        // Build generate request
        let request = botticelli_core::GenerateRequest::builder()
            .messages(vec![user_message])
            .build()
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to build generate request");
                ChatError::new(ChatErrorKind::InvalidInput(format!("Failed to build request: {}", e)))
            })?;
        
        tracing::debug!("Getting LLM provider with tools from services");
        // Get LLM provider with tool calling support
        let llm = services.llm_provider_with_tools().await?;
        
        tracing::debug!("Getting MCP client from services");
        // Get MCP client for tool definitions
        let mcp = services.mcp_client().await?;
        let tools = mcp.list_all_tools();
        
        tracing::info!(tool_count = tools.len(), "Retrieved MCP tools");
        
        // Call LLM with tools
        tracing::info!("Calling LLM generate_with_tools");
        let response = llm.generate_with_tools(&request, &tools).await
            .map_err(|e| {
                tracing::error!(error = ?e, "LLM generate failed");
                ChatError::new(ChatErrorKind::ExecutionFailed(format!("LLM error: {}", e)))
            })?;
        
        tracing::info!(output_count = response.outputs().len(), "Received LLM response");
        
        // TODO: Send response back to UI
        // This requires a channel or callback mechanism
        tracing::warn!("Need to implement response callback to UI");
        
        Ok(())
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
        use ratatui::widgets::Widget;

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

        // Build all message lines with proper wrapping
        let mut all_lines: Vec<Line> = Vec::new();
        
        for msg in &self.messages {
            let timestamp = msg.timestamp.format("%H:%M:%S");

            let (role_str, role_color) = match msg.role {
                Role::User => ("You", Color::Cyan),
                Role::Assistant => ("Assistant", Color::Green),
                Role::System => ("System", Color::Yellow),
            };

            // Add header line
            all_lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    role_str,
                    Style::default().fg(role_color).add_modifier(Modifier::BOLD),
                ),
            ]));

            // Add content as a single line - Paragraph will wrap it
            all_lines.push(Line::from(msg.content.clone()));
            
            // Add blank line between messages
            all_lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(all_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Chat ({} messages)", self.messages.len())),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.list_state.selected().unwrap_or(0) as u16, 0));

        Widget::render(paragraph, area, buf);
    }
}

impl Default for ChatTab {
    fn default() -> Self {
        Self::new()
    }
}
