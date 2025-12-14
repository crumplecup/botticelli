//! TUI implementation of ElicitationDialog.

use async_trait::async_trait;
use botticelli_error::{BotticelliResult, TuiError, TuiErrorKind};
use botticelli_mcp::ElicitationDialog;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use tracing::{debug, instrument};

/// TUI implementation of ElicitationDialog using ratatui.
///
/// Provides an interactive terminal interface for narrative elicitation.
pub struct TuiElicitationDialog {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    messages: Vec<DialogMessage>,
}

#[derive(Debug, Clone)]
struct DialogMessage {
    level: MessageLevel,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageLevel {
    Info,
    Warning,
    Error,
    Progress,
    Preview,
}

impl TuiElicitationDialog {
    /// Create a new TUI elicitation dialog.
    ///
    /// Sets up terminal in raw mode with alternate screen.
    #[instrument]
    pub fn new() -> BotticelliResult<Self> {
        enable_raw_mode().map_err(|e| {
            TuiError::new(TuiErrorKind::TerminalSetup(format!(
                "Failed to enable raw mode: {}",
                e
            )))
        })?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
            TuiError::new(TuiErrorKind::TerminalSetup(format!(
                "Failed to setup terminal: {}",
                e
            )))
        })?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(|e| {
            TuiError::new(TuiErrorKind::TerminalSetup(format!(
                "Failed to create terminal: {}",
                e
            )))
        })?;

        debug!("TUI elicitation dialog initialized");

        Ok(Self {
            terminal,
            messages: Vec::new(),
        })
    }

    /// Render the current UI state.
    #[instrument(skip(self))]
    fn render(&mut self) -> BotticelliResult<()> {
        let messages = self.messages.clone();

        self.terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(3),    // Messages area
                        Constraint::Length(3), // Input prompt area
                    ])
                    .split(f.area());

                // Render messages
                Self::render_messages_static(&messages, f, chunks[0]);

                // Render input prompt (placeholder for now)
                let input_block = Block::default()
                    .title("Input")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Cyan));
                f.render_widget(input_block, chunks[1]);
            })
            .map_err(|e| {
                TuiError::new(TuiErrorKind::Rendering(format!(
                    "Failed to render: {}",
                    e
                )))
            })?;

        Ok(())
    }

    /// Render message history (static version for use in closures).
    fn render_messages_static(
        messages: &[DialogMessage],
        f: &mut Frame,
        area: ratatui::layout::Rect,
    ) {
        let items: Vec<ListItem> = messages
            .iter()
            .map(|msg| {
                let (prefix, style) = match msg.level {
                    MessageLevel::Info => ("ℹ", Style::default().fg(Color::Blue)),
                    MessageLevel::Warning => ("⚠", Style::default().fg(Color::Yellow)),
                    MessageLevel::Error => ("✖", Style::default().fg(Color::Red)),
                    MessageLevel::Progress => ("▶", Style::default().fg(Color::Cyan)),
                    MessageLevel::Preview => ("👁", Style::default().fg(Color::Green)),
                };

                let content = Line::from(vec![
                    Span::styled(format!("{} ", prefix), style),
                    Span::raw(&msg.content),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title("Narrative Elicitation")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White)),
        );

        f.render_widget(list, area);
    }

    /// Add a message to the display.
    fn add_message(&mut self, level: MessageLevel, content: String) {
        self.messages.push(DialogMessage { level, content });
    }

    /// Read a line of text input from the user.
    #[instrument(skip(self))]
    fn read_line(&mut self, prompt: &str) -> BotticelliResult<String> {
        self.add_message(MessageLevel::Info, format!("❯ {}", prompt));
        self.render()?;

        let mut input = String::new();

        loop {
            if event::poll(std::time::Duration::from_millis(100)).map_err(|e| {
                TuiError::new(TuiErrorKind::EventRead(format!(
                    "Event poll failed: {}",
                    e
                )))
            })? {
                if let Event::Key(key) = event::read().map_err(|e| {
                    TuiError::new(TuiErrorKind::EventRead(format!(
                        "Key read failed: {}",
                        e
                    )))
                })? {
                    match key.code {
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Char(c) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                                return Err(TuiError::new(TuiErrorKind::EventRead(
                                    "User cancelled".to_string(),
                                ))
                                .into());
                            }
                            input.push(c);
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        _ => {}
                    }
                }
            }
        }

        debug!(input = %input, "User input received");
        Ok(input)
    }
}

impl Drop for TuiElicitationDialog {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }
}

#[async_trait]
impl ElicitationDialog for TuiElicitationDialog {
    #[instrument(skip(self))]
    async fn ask_text(&mut self, prompt: &str) -> BotticelliResult<String> {
        self.read_line(prompt)
    }

    #[instrument(skip(self))]
    async fn ask_confirmation(&mut self, prompt: &str, default: bool) -> BotticelliResult<bool> {
        let default_text = if default { "[Y/n]" } else { "[y/N]" };
        let full_prompt = format!("{} {}", prompt, default_text);
        let response = self.read_line(&full_prompt)?;

        let answer = response.trim().to_lowercase();
        Ok(match answer.as_str() {
            "y" | "yes" => true,
            "n" | "no" => false,
            "" => default,
            _ => {
                self.add_message(
                    MessageLevel::Warning,
                    "Invalid input, using default".to_string(),
                );
                self.render()?;
                default
            }
        })
    }

    #[instrument(skip(self, options))]
    async fn ask_choice(&mut self, prompt: &str, options: &[&str]) -> BotticelliResult<usize> {
        self.add_message(MessageLevel::Info, prompt.to_string());

        for (i, option) in options.iter().enumerate() {
            self.add_message(MessageLevel::Info, format!("  {}. {}", i + 1, option));
        }

        self.render()?;

        loop {
            let input = self.read_line("Enter number:")?;

            if let Ok(choice) = input.trim().parse::<usize>() {
                if choice > 0 && choice <= options.len() {
                    return Ok(choice - 1);
                }
            }

            self.add_message(
                MessageLevel::Error,
                format!("Invalid choice. Enter 1-{}", options.len()),
            );
            self.render()?;
        }
    }

    #[instrument(skip(self))]
    async fn ask_number(&mut self, prompt: &str, min: i64, max: i64) -> BotticelliResult<i64> {
        let full_prompt = format!("{} ({}-{})", prompt, min, max);

        loop {
            let input = self.read_line(&full_prompt)?;

            if let Ok(num) = input.trim().parse::<i64>() {
                if num >= min && num <= max {
                    return Ok(num);
                }
            }

            self.add_message(
                MessageLevel::Error,
                format!("Invalid number. Enter {}-{}", min, max),
            );
            self.render()?;
        }
    }

    #[instrument(skip(self))]
    async fn ask_file_path(&mut self, prompt: &str) -> BotticelliResult<String> {
        self.read_line(prompt)
    }

    #[instrument(skip(self))]
    async fn show_info(&mut self, message: &str) -> BotticelliResult<()> {
        self.add_message(MessageLevel::Info, message.to_string());
        self.render()
    }

    #[instrument(skip(self))]
    async fn show_warning(&mut self, message: &str) -> BotticelliResult<()> {
        self.add_message(MessageLevel::Warning, message.to_string());
        self.render()
    }

    #[instrument(skip(self))]
    async fn show_error(&mut self, message: &str) -> BotticelliResult<()> {
        self.add_message(MessageLevel::Error, message.to_string());
        self.render()
    }

    #[instrument(skip(self, validation_text))]
    async fn show_validation(&mut self, validation_text: &str) -> BotticelliResult<()> {
        self.add_message(MessageLevel::Info, validation_text.to_string());
        self.render()
    }

    #[instrument(skip(self))]
    async fn show_progress(
        &mut self,
        current: usize,
        total: usize,
        description: &str,
    ) -> BotticelliResult<()> {
        self.add_message(
            MessageLevel::Progress,
            format!("[{}/{}] {}", current, total, description),
        );
        self.render()
    }

    #[instrument(skip(self, toml))]
    async fn show_preview(&mut self, toml: &str) -> BotticelliResult<()> {
        self.add_message(MessageLevel::Preview, "=== TOML Preview ===".to_string());
        for line in toml.lines() {
            self.add_message(MessageLevel::Preview, line.to_string());
        }
        self.add_message(MessageLevel::Preview, "=== End Preview ===".to_string());
        self.render()
    }
}
