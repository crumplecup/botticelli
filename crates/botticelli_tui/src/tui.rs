//! Main TUI entry point and coordinator.

use crate::{AppState, Event, EventHandler, TuiResult};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

/// Main TUI coordinator.
///
/// Manages the terminal, event handling, and rendering loop.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    events: EventHandler,
    state: AppState,
}

impl Tui {
    /// Create a new TUI instance.
    pub fn new() -> TuiResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(std::time::Duration::from_millis(250));
        let state = AppState::default();

        Ok(Self {
            terminal,
            events,
            state,
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

            // Handle events
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
        }
        Ok(true)
    }
}
