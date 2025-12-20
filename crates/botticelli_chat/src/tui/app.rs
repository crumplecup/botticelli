// Main TUI application

use crate::tui::{AppState, EventHandler, EventResult};

#[cfg(feature = "tui")]
use crossterm::event;
#[cfg(feature = "tui")]
use ratatui::{backend::Backend, Terminal};

/// Main TUI application
pub struct TuiApp {
    /// Application state
    pub state: AppState,
    /// Event handler
    pub event_handler: EventHandler,
}

impl TuiApp {
    /// Creates a new TUI application
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            event_handler: EventHandler::new(),
        }
    }

    #[cfg(feature = "tui")]
    /// Runs the application main loop
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> {
        loop {
            // Render the UI
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            // Handle events
            if event::poll(std::time::Duration::from_millis(100))? {
                let event = event::read()?;
                let result = self.event_handler.handle_event(event, &mut self.state);

                if result == EventResult::ShouldQuit || self.state.should_quit {
                    break;
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "tui")]
    /// Renders the application UI
    fn render(&mut self, frame: &mut ratatui::Frame) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            widgets::{Block, Borders, Paragraph, Tabs},
        };

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
                Constraint::Length(1), // Shortcut bar
            ])
            .split(frame.area());

        // Render tab bar
        let tab_titles = vec![
            "Narratives",
            "Bots",
            "Database",
            "Chat",
            "Schedule",
            "Settings",
        ];
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Botticelli"))
            .select(self.state.active_tab as usize)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(tabs, chunks[0]);

        // Render active tab content
        self.render_active_tab(frame, chunks[1]);

        // Render status bar
        let status_text = format!(
            "Status: {} | Provider: {} | DB: {}",
            if self.state.status.db_connected {
                "Connected"
            } else {
                "Disconnected"
            },
            self.state.status.active_provider,
            if self.state.status.db_connected {
                "✓"
            } else {
                "✗"
            }
        );
        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, chunks[2]);

        // Render shortcut bar
        let shortcuts = Paragraph::new("? Help | Tab: Next Tab | 1-6: Direct Tab | q: Quit");
        frame.render_widget(shortcuts, chunks[3]);

        // Render modal if active
        if let Some(_modal) = &self.state.modal {
            // TODO: Render modal overlay
        }
    }

    #[cfg(feature = "tui")]
    /// Renders the currently active tab
    fn render_active_tab(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use crate::tui::state::Tab;

        match self.state.active_tab {
            Tab::Narratives => {
                // Render narratives tab
                let buf = frame.buffer_mut();
                self.state.narratives_state.render(area, buf);
            }
            Tab::Chat => {
                // Render chat tab
                let buf = frame.buffer_mut();
                self.state.chat_state.render(area, buf);
            }
            Tab::Bots => {
                // Render bots tab
                let buf = frame.buffer_mut();
                self.state.bots_state.render(area, buf);
            }
            Tab::Database => {
                // Render database tab
                let buf = frame.buffer_mut();
                self.state.database_state.render(area, buf);
            }
            Tab::Schedule => {
                // Render schedule tab
                let buf = frame.buffer_mut();
                self.state.schedule_state.render(area, buf);
            }
            Tab::Settings => {
                // Render settings tab
                let buf = frame.buffer_mut();
                self.state.settings_state.render(area, buf);
            }
        }
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
