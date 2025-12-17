//! TUI application coordinator.
//!
//! Wires together views, commands, state, and provides clean library entry points.

use crate::{AppState, Command, Event, EventHandler, McpUpdate, TuiResult, ViewMode};
use botticelli_interface::BotticelliDriver;
use crossterm::event::KeyEvent;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::Arc};
use tokio::sync::mpsc;
use tracing::debug;

/// Main TUI application.
///
/// Coordinates views, commands, state, and event handling.
/// This is the primary library interface - binaries should use this.
pub struct TuiApp {
    /// Terminal backend.
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    /// Event handler.
    events: EventHandler,
    /// Application state.
    state: AppState,
    /// MCP update channel receiver.
    mcp_rx: mpsc::UnboundedReceiver<McpUpdate>,
}

impl TuiApp {
    /// Create a new TUI application with MCP integration.
    ///
    /// This is the main entry point for library usage.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use botticelli_models::AnthropicClient;
    /// use botticelli_tui::TuiApp;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    /// let driver = Arc::new(AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022"));
    ///
    /// let mut app = TuiApp::new(driver)?;
    /// app.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(driver: Arc<dyn BotticelliDriver>) -> TuiResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(std::time::Duration::from_millis(250));

        // Create channel for MCP updates
        let (mcp_tx, mcp_rx) = mpsc::unbounded_channel();

        // Initialize AppState with MCP integration
        let mut state = AppState::with_mcp_integration(driver);
        state.set_mcp_channel(mcp_tx);

        Ok(Self {
            terminal,
            events,
            state,
            mcp_rx,
        })
    }

    /// Run the TUI application.
    ///
    /// This enters the main event loop and blocks until the user quits.
    pub async fn run(&mut self) -> TuiResult<()> {
        // Setup terminal
        self.setup_terminal()?;

        // Main event loop
        loop {
            // Render current view
            self.render()?;

            // Check for MCP updates (non-blocking)
            while let Ok(update) = self.mcp_rx.try_recv() {
                self.handle_event(Event::McpUpdate(update)).await?;
            }

            // Handle terminal events
            if let Some(event) = self.events.next().await?
                && !self.handle_event(event).await?
            {
                break;
            }
        }

        // Cleanup terminal
        self.cleanup_terminal()?;

        Ok(())
    }

    /// Setup terminal for TUI.
    fn setup_terminal(&mut self) -> TuiResult<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;

        self.terminal.clear()?;

        Ok(())
    }

    /// Cleanup terminal after TUI.
    fn cleanup_terminal(&mut self) -> TuiResult<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;

        Ok(())
    }

    /// Render the current view.
    fn render(&mut self) -> TuiResult<()> {
        let state = &self.state;
        self.terminal.draw(|frame| {
            // Render current view
            state.current_view().render(frame, state).ok();
        })?;
        Ok(())
    }

    /// Handle an event and return false if should quit.
    async fn handle_event(&mut self, event: Event) -> TuiResult<bool> {
        match event {
            Event::Quit => return Ok(false),
            Event::Key(key_event) => {
                // First, let the current view handle the key
                if let Some(command) = self
                    .state
                    .current_view()
                    .handle_input(key_event, &self.state)?
                {
                    return self.handle_command(command).await;
                }

                // If view didn't handle it, check for global keybindings
                if let Some(command) = self.handle_global_keys(key_event) {
                    return self.handle_command(command).await;
                }
            }
            Event::Mouse(mouse_event) => {
                self.state.handle_mouse(mouse_event)?;
            }
            Event::Resize(width, height) => {
                self.state.handle_resize(width, height)?;
            }
            Event::Tick => {
                self.state.update()?;
            }
            Event::McpUpdate(update) => {
                self.state.handle_mcp_update(update)?;
            }
        }
        Ok(true)
    }

    /// Handle global keybindings (Tab for view switching, etc.).
    fn handle_global_keys(&self, key: KeyEvent) -> Option<Command> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.code, key.modifiers) {
            // Tab cycles through views
            (KeyCode::Tab, KeyModifiers::NONE) => {
                let next_mode = match self.state.mode() {
                    ViewMode::Chat => ViewMode::NarrativeBrowser,
                    ViewMode::NarrativeBrowser => ViewMode::NarrativeEditor,
                    ViewMode::NarrativeEditor => ViewMode::Settings,
                    ViewMode::Settings => ViewMode::Chat,
                };
                Some(Command::SwitchMode(next_mode))
            }
            // Shift+Tab cycles backwards
            (KeyCode::BackTab, _) => {
                let prev_mode = match self.state.mode() {
                    ViewMode::Chat => ViewMode::Settings,
                    ViewMode::Settings => ViewMode::NarrativeEditor,
                    ViewMode::NarrativeEditor => ViewMode::NarrativeBrowser,
                    ViewMode::NarrativeBrowser => ViewMode::Chat,
                };
                Some(Command::SwitchMode(prev_mode))
            }
            // Ctrl+Q always quits
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Some(Command::Quit),
            _ => None,
        }
    }

    /// Handle a command and return false if should quit.
    async fn handle_command(&mut self, command: Command) -> TuiResult<bool> {
        debug!(?command, "Handling command");

        match command {
            Command::Quit => return Ok(false),
            Command::SwitchMode(mode) => {
                debug!(?mode, "Switching to view mode");
                self.state.set_mode(mode);
            }
            Command::SendMessage(message) => {
                // Send message with orchestration (tool calling support)
                self.state.send_message_with_orchestration(message)?;
            }
            Command::NavigateUp => {
                if let Some(idx) = self.state.selected_narrative()
                    && idx > 0
                {
                    self.state.set_selected_narrative(Some(idx - 1));
                }
            }
            Command::NavigateDown => {
                if let Some(idx) = self.state.selected_narrative() {
                    let max = self.state.narrative_list().len().saturating_sub(1);
                    if idx < max {
                        self.state.set_selected_narrative(Some(idx + 1));
                    }
                } else if !self.state.narrative_list().is_empty() {
                    self.state.set_selected_narrative(Some(0));
                }
            }
            Command::AppendChar(c) => {
                self.state.append_input(&c.to_string());
            }
            Command::DeleteChar => {
                self.state.delete_char();
            }
            _ => {
                // Other commands not yet implemented
                debug!(?command, "Command not yet implemented");
            }
        }

        Ok(true)
    }
}
