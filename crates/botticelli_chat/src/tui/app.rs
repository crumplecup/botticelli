//! Main TUI application.

use crate::{
    tui::{
        state::{AppState, Modal, Tab},
        events::{poll_event, is_quit_event},
        commands::TuiCommand,
    },
    ServiceContainer,
};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::{
    io::{self, Stdout},
    sync::Arc,
    time::Duration,
};
use tracing::instrument;

/// Main TUI application.
pub struct App {
    /// Application state
    state: AppState,
    /// Service container
    services: Arc<ServiceContainer>,
    /// Terminal backend
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Should quit flag
    should_quit: bool,
}

impl App {
    /// Create a new TUI application.
    #[instrument(skip(services))]
    pub fn new(services: Arc<ServiceContainer>) -> ChatResult<Self> {
        enable_raw_mode().map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!("Failed to enable raw mode: {}", e)))
        })?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to enter alternate screen: {}",
                e
            )))
        })?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to create terminal: {}",
                e
            )))
        })?;

        Ok(Self {
            state: AppState::new(),
            services,
            terminal,
            should_quit: false,
        })
    }

    /// Run the TUI application.
    #[instrument(skip(self))]
    pub fn run(&mut self) -> ChatResult<()> {
        loop {
            // Render
            let state = &mut self.state;
            self.terminal
                .draw(|f| Self::render_frame(f, state))
                .map_err(|e| {
                    ChatError::new(ChatErrorKind::IoError(format!("Failed to draw: {}", e)))
                })?;

            // Handle events
            if let Some(event) = poll_event(Duration::from_millis(100))
                .map_err(|e| {
                    ChatError::new(ChatErrorKind::IoError(format!("Failed to poll event: {}", e)))
                })?
            {
                self.handle_event(event)?;
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Render the TUI frame (static method to avoid borrow issues).
    fn render_frame(frame: &mut Frame, state: &mut AppState) {
        let size = frame.area();

        // Main layout: Tab bar + content + status bar + shortcuts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Tab bar
                Constraint::Min(0),     // Content
                Constraint::Length(1),  // Status bar
                Constraint::Length(1),  // Shortcuts
            ])
            .split(size);

        // Render tab bar
        Self::render_tab_bar(frame, chunks[0], state);

        // Render active tab content
        Self::render_active_tab(frame, chunks[1], state);

        // Render status bar
        Self::render_status_bar(frame, chunks[2], state);

        // Render shortcuts
        Self::render_shortcuts(frame, chunks[3]);

        // Render modal if any
        if let Some(ref modal) = state.modal {
            Self::render_modal(frame, size, modal);
        }
    }

    /// Render tab bar.
    fn render_tab_bar(frame: &mut Frame, area: Rect, state: &AppState) {
        let titles = vec![
            "Narratives",
            "Bots",
            "Database",
            "Chat",
            "Schedule",
            "Settings",
        ];

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Botticelli"))
            .select(state.active_tab.index() - 1)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(tabs, area);
    }

    /// Render active tab content.
    fn render_active_tab(frame: &mut Frame, area: Rect, state: &mut AppState) {
        match state.active_tab {
            Tab::Narratives => Self::render_narratives_tab(frame, area),
            Tab::Bots => Self::render_bots_tab(frame, area),
            Tab::Database => Self::render_database_tab(frame, area),
            Tab::Chat => Self::render_chat_tab(frame, area),
            Tab::Schedule => Self::render_schedule_tab(frame, area),
            Tab::Settings => Self::render_settings_tab(frame, area),
        }
    }

    /// Render status bar.
    fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
        let status_text = format!(
            "Status: Ready | Provider: {} | DB: {} | MCP: {}",
            state.status.active_provider,
            if state.status.db_connected {
                "Connected"
            } else {
                "Disconnected"
            },
            if state.status.mcp_connected {
                "Connected"
            } else {
                "Disconnected"
            }
        );

        let status = Paragraph::new(status_text).style(Style::default().fg(Color::Gray));

        frame.render_widget(status, area);
    }

    /// Render keyboard shortcuts.
    fn render_shortcuts(frame: &mut Frame, area: Rect) {
        let shortcuts = Paragraph::new("? Help | Ctrl+C Quit | Tab Switch | / Search")
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(shortcuts, area);
    }

    /// Render modal overlay.
    fn render_modal(frame: &mut Frame, area: Rect, modal: &Modal) {
        match modal {
            Modal::Help => Self::render_help_modal(frame, area),
            Modal::Error { message } => Self::render_error_modal(frame, area, message),
            Modal::Confirm { message, .. } => Self::render_confirm_modal(frame, area, message),
            Modal::Search { query, results } => {
                Self::render_search_modal(frame, area, query, results)
            }
        }
    }

    /// Render help modal.
    fn render_help_modal(frame: &mut Frame, area: Rect) {
        let help_text = vec![
            "Keyboard Shortcuts:",
            "",
            "Tab / Shift+Tab - Switch tabs",
            "1-6 - Jump to tab",
            "? - Show help",
            "/ - Search",
            "Ctrl+C, Ctrl+Q - Quit",
            "",
            "Press Esc to close",
        ]
        .join("\n");

        let block = Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(help_text).block(block);

        // Center modal
        let modal_area = centered_rect(60, 50, area);
        frame.render_widget(paragraph, modal_area);
    }

    /// Render error modal.
    fn render_error_modal(frame: &mut Frame, area: Rect, message: &str) {
        let block = Block::default()
            .title("Error")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::Red));

        let paragraph = Paragraph::new(message).block(block);

        let modal_area = centered_rect(60, 30, area);
        frame.render_widget(paragraph, modal_area);
    }

    /// Render confirmation modal.
    fn render_confirm_modal(frame: &mut Frame, area: Rect, message: &str) {
        let text = format!("{}\n\nPress Y to confirm, N to cancel", message);

        let block = Block::default()
            .title("Confirm")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(text).block(block);

        let modal_area = centered_rect(60, 30, area);
        frame.render_widget(paragraph, modal_area);
    }

    /// Render search modal.
    fn render_search_modal(frame: &mut Frame, area: Rect, query: &str, results: &[String]) {
        let text = format!(
            "Search: {}\n\nResults:\n{}",
            query,
            results.join("\n")
        );

        let block = Block::default()
            .title("Search")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(text).block(block);

        let modal_area = centered_rect(80, 60, area);
        frame.render_widget(paragraph, modal_area);
    }

    // Tab rendering methods (placeholders for now)

    fn render_narratives_tab(frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("Narratives Tab - Coming Soon")
            .block(Block::default().borders(Borders::ALL).title("Narratives"));
        frame.render_widget(placeholder, area);
    }

    fn render_bots_tab(frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("Bots Tab - Coming Soon")
            .block(Block::default().borders(Borders::ALL).title("Bots"));
        frame.render_widget(placeholder, area);
    }

    fn render_database_tab(frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("Database Tab - Coming Soon")
            .block(Block::default().borders(Borders::ALL).title("Database"));
        frame.render_widget(placeholder, area);
    }

    fn render_chat_tab(frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("Chat Tab - Coming Soon")
            .block(Block::default().borders(Borders::ALL).title("Chat"));
        frame.render_widget(placeholder, area);
    }

    fn render_schedule_tab(frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("Schedule Tab - Coming Soon")
            .block(Block::default().borders(Borders::ALL).title("Schedule"));
        frame.render_widget(placeholder, area);
    }

    fn render_settings_tab(frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new("Settings Tab - Coming Soon")
            .block(Block::default().borders(Borders::ALL).title("Settings"));
        frame.render_widget(placeholder, area);
    }

    /// Handle keyboard/mouse events.
    #[instrument(skip(self))]
    fn handle_event(&mut self, event: Event) -> ChatResult<()> {
        // Check for quit
        if is_quit_event(&event) {
            self.should_quit = true;
            return Ok(());
        }

        // Handle modal events first
        if self.state.modal.is_some() {
            return self.handle_modal_event(event);
        }

        // Handle key events
        if let Event::Key(key) = event {
            // Global shortcuts
            match key.code {
                KeyCode::Tab if key.modifiers.is_empty() => {
                    self.state.next_tab();
                }
                KeyCode::BackTab => {
                    self.state.prev_tab();
                }
                KeyCode::Char('?') => {
                    self.state.modal = Some(Modal::Help);
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    if let Some(tab) = Tab::from_index(c.to_digit(10).unwrap() as usize) {
                        self.state.switch_tab(tab);
                    }
                }
                _ => {
                    // Delegate to active tab
                    self.handle_tab_event(key)?;
                }
            }
        }

        Ok(())
    }

    /// Handle events when modal is open.
    fn handle_modal_event(&mut self, event: Event) -> ChatResult<()> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc => {
                    self.state.modal = None;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(Modal::Confirm { action, .. }) = self.state.modal.take() {
                        // Execute confirmed action
                        self.execute_confirm_action(action)?;
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if matches!(self.state.modal, Some(Modal::Confirm { .. })) {
                        self.state.modal = None;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Execute confirmed action.
    fn execute_confirm_action(&mut self, action: crate::tui::state::ConfirmAction) -> ChatResult<()> {
        match action {
            crate::tui::state::ConfirmAction::Quit => {
                self.should_quit = true;
            }
            crate::tui::state::ConfirmAction::DeleteNarrative(_path) => {
                // TODO: Implement narrative deletion
            }
            crate::tui::state::ConfirmAction::StopBot(_id) => {
                // TODO: Implement bot stopping
            }
        }
        Ok(())
    }

    /// Handle events for active tab.
    fn handle_tab_event(&mut self, _key: crossterm::event::KeyEvent) -> ChatResult<()> {
        match self.state.active_tab {
            Tab::Narratives => {
                // TODO: Delegate to narratives tab handler
            }
            Tab::Bots => {
                // TODO: Delegate to bots tab handler
            }
            Tab::Database => {
                // TODO: Delegate to database tab handler
            }
            Tab::Chat => {
                // TODO: Delegate to chat tab handler
            }
            Tab::Schedule => {
                // TODO: Delegate to schedule tab handler
            }
            Tab::Settings => {
                // TODO: Delegate to settings tab handler
            }
        }

        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Helper to create centered rectangle for modals.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
