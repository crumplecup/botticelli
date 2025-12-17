//! Main TUI entry point and coordinator.

use crate::{AppState, Event, EventHandler, McpMessage, TuiResult};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::sync::mpsc;

/// Main TUI coordinator.
///
/// Manages the terminal, event handling, and rendering loop.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    events: EventHandler,
    state: AppState,
    /// Receiver for MCP execution updates from async tasks
    mcp_rx: mpsc::UnboundedReceiver<McpMessage>,
}

impl Tui {
    /// Create a new TUI instance.
    pub fn new() -> TuiResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(std::time::Duration::from_millis(250));

        // Create channel for MCP updates
        let (mcp_tx, mcp_rx) = mpsc::unbounded_channel();

        let mut state = AppState::default();
        state.set_mcp_channel(mcp_tx);

        Ok(Self {
            terminal,
            events,
            state,
            mcp_rx,
        })
    }

    /// Create TUI with MCP integration.
    ///
    /// Takes an LLM driver (Anthropic, Gemini, etc.) and initializes the full
    /// MCP stack for tool execution.
    pub fn with_mcp(
        driver: std::sync::Arc<dyn botticelli_interface::BotticelliDriver>,
    ) -> TuiResult<Self> {
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

    /// Run the TUI event loop.
    pub async fn run(&mut self) -> TuiResult<()> {
        // Setup terminal
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;

        self.terminal.clear()?;

        // Main event loop
        loop {
            // Render current view
            self.render()?;

            // Check for MCP updates (non-blocking)
            while let Ok(msg) = self.mcp_rx.try_recv() {
                let event = match msg {
                    McpMessage::Update(update) => Event::McpUpdate(update),
                    McpMessage::Error(error) => Event::McpError(error),
                };
                self.handle_event(event).await?;
            }

            // Handle terminal events
            if let Some(event) = self.events.next().await?
                && !self.handle_event(event).await?
            {
                break;
            }
        }

        // Cleanup terminal
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
        self.terminal.draw(|frame| {
            self.state.current_view().render(frame, &self.state).ok();
        })?;
        Ok(())
    }

    /// Handle an event.
    ///
    /// Returns `false` if the application should quit.
    async fn handle_event(&mut self, event: Event) -> TuiResult<bool> {
        match event {
            Event::Quit => return Ok(false),
            Event::Key(key_event) => {
                self.state.handle_key(key_event).await?;
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
            Event::McpError(error) => {
                self.state.handle_mcp_error(error)?;
            }
        }
        Ok(true)
    }
}
