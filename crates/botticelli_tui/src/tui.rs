//! Main TUI entry point and coordinator.

use crate::{AppState, Event, EventHandler, McpMessage, TuiResult};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::Arc};
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
        Self::with_llm(None)
    }

    /// Create a new TUI instance with an LLM backend.
    ///
    /// The chat_host should implement the ChatHost trait, integrating LLM and MCP tools.
    pub fn with_llm(chat_host: Option<Arc<std::sync::Mutex<dyn botticelli_interface::ChatHost>>>) -> TuiResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(std::time::Duration::from_millis(250));

        // Create channel for MCP updates
        let (mcp_tx, mcp_rx) = mpsc::unbounded_channel();

        let state = if let Some(host) = chat_host {
            let mut state = AppState::new(host);
            state.with_mcp_channel(Some(mcp_tx));
            state
        } else {
            let mut state = AppState::default();
            state.with_mcp_channel(Some(mcp_tx));
            state
        };

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
        let state = &self.state;
        self.terminal.draw(|frame| {
            use crate::view::View;
            use crate::state::ViewMode;
            
            let result = match state.mode() {
                ViewMode::Chat => crate::view::ChatView.render(frame, state),
                ViewMode::NarrativeBrowser => crate::view::NarrativeBrowserView.render(frame, state),
                ViewMode::ConversationHistory => crate::view::ConversationHistoryView.render(frame, state),
                ViewMode::NarrativeEditor => crate::view::NarrativeEditorView.render(frame, state),
                ViewMode::Settings => crate::view::SettingsView.render(frame, state),
                ViewMode::Bots => crate::view::BotsView.render(frame, state),
                ViewMode::Database => crate::view::DatabaseView.render(frame, state),
                ViewMode::Schedule => crate::view::ScheduleView.render(frame, state),
            };
            
            if let Err(e) = result {
                tracing::error!("Render error: {}", e);
            }
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
                self.state.handle_key(key_event)?;
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
