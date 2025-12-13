//! Terminal UI implementation for chat interface.

use crate::{
    parse_intent, ChatError, ChatErrorKind, ChatInterface, ChatResult, CommandExecutor, Message,
    Response, UserInput,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use std::io::{self, Stdout};
use tracing::instrument;

use crate::ServiceContainer;
use std::sync::Arc;

/// Terminal user interface for interactive chat.
pub struct TuiInterface {
    messages: Vec<RenderedMessage>,
    input_buffer: String,
    scroll_offset: usize,
    executor: CommandExecutor,
}

#[derive(Debug, Clone)]
struct RenderedMessage {
    role: MessageRole,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageRole {
    System,
    User,
    Assistant,
}

impl TuiInterface {
    /// Create a new TUI interface.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            executor: CommandExecutor::new(),
        }
    }

    /// Create a new TUI interface with services.
    pub fn with_services(services: Arc<ServiceContainer>) -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            executor: CommandExecutor::with_services(services),
        }
    }

    /// Run the interactive TUI loop.
    #[instrument(skip(self, terminal))]
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> ChatResult<()> {
        self.add_welcome_message();

        loop {
            terminal
                .draw(|f| self.render(f))
                .map_err(|e| ChatError::new(ChatErrorKind::IoError(format!("Render failed: {}", e))))?;

            if let Event::Key(key) = event::read()
                .map_err(|e| ChatError::new(ChatErrorKind::IoError(format!("Event read failed: {}", e))))?
            {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        break;
                    }
                    (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                        self.clear()?;
                        self.add_welcome_message();
                    }
                    (KeyCode::Enter, _) => {
                        if !self.input_buffer.is_empty() {
                            let input = self.input_buffer.clone();
                            self.input_buffer.clear();
                            self.handle_user_input(input).await?;
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        self.input_buffer.pop();
                    }
                    (KeyCode::Char(c), _) => {
                        self.input_buffer.push(c);
                    }
                    (KeyCode::Up, _) => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                    (KeyCode::Down, _) => {
                        self.scroll_offset += 1;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(f.area());

        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .skip(self.scroll_offset)
            .map(|msg| {
                let style = match msg.role {
                    MessageRole::System => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    MessageRole::User => Style::default().fg(Color::Green),
                    MessageRole::Assistant => Style::default().fg(Color::Yellow),
                };

                let prefix = match msg.role {
                    MessageRole::System => "System: ",
                    MessageRole::User => "You: ",
                    MessageRole::Assistant => "Assistant: ",
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::raw(&msg.content),
                ]))
            })
            .collect();

        let messages_widget = List::new(messages).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Botticelli Interactive Console")
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
        );

        f.render_widget(messages_widget, chunks[0]);

        let input_widget = Paragraph::new(self.input_buffer.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Input (Ctrl+C to quit, Ctrl+L to clear)")
                    .title_style(Style::default().fg(Color::Gray)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(input_widget, chunks[1]);
    }

    async fn handle_user_input(&mut self, input: String) -> ChatResult<()> {
        self.add_message(MessageRole::User, input.clone());

        match parse_intent(&input) {
            Ok(command) => {
                if command.is_exit() {
                    self.add_message(MessageRole::System, "Exiting...".to_string());
                    return Err(ChatError::new(ChatErrorKind::InvalidState("Exit requested".to_string())));
                }

                match self.executor.execute(command).await {
                    Ok(response) => {
                        self.add_message(MessageRole::Assistant, response.as_text());
                    }
                    Err(e) => {
                        self.add_message(
                            MessageRole::System,
                            format!("Error: {}", e.kind),
                        );
                    }
                }
            }
            Err(e) => {
                self.add_message(
                    MessageRole::System,
                    format!("Could not understand command: {}", e.kind),
                );
            }
        }

        Ok(())
    }

    fn add_message(&mut self, role: MessageRole, content: String) {
        self.messages.push(RenderedMessage { role, content });
    }

    fn add_welcome_message(&mut self) {
        let welcome = vec![
            "Welcome! I can help you:",
            "• Generate narratives",
            "• Assign narratives to bots",
            "• Schedule social media posts",
            "• Monitor system status",
            "",
            "Try: 'Create a narrative' or 'Help'",
        ];

        for line in welcome {
            self.add_message(MessageRole::System, line.to_string());
        }
    }
}

impl Default for TuiInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatInterface for TuiInterface {
    #[instrument(skip(self))]
    fn send_message(&mut self, message: Message) -> ChatResult<()> {
        self.add_message(MessageRole::Assistant, message.content().to_string());
        Ok(())
    }

    #[instrument(skip(self))]
    fn receive_input(&mut self) -> ChatResult<UserInput> {
        Ok(UserInput::Text(self.input_buffer.clone()))
    }

    #[instrument(skip(self))]
    fn send_response(&mut self, response: Response) -> ChatResult<()> {
        self.add_message(MessageRole::Assistant, format!("{:?}", response));
        Ok(())
    }

    #[instrument(skip(self))]
    fn clear(&mut self) -> ChatResult<()> {
        self.messages.clear();
        Ok(())
    }
}

/// Setup terminal for TUI mode.
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore terminal to normal mode.
pub fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
