use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};

use crate::{TuiError, TuiErrorKind, TuiResult};

/// Update from MCP execution.
#[derive(Debug, Clone)]
pub struct McpUpdate {
    /// Conversation ID this update belongs to
    pub conversation_id: uuid::Uuid,
    /// Original user message that triggered this execution
    pub user_message: String,
    /// Execution result with tool calls
    pub result: botticelli_mcp_client::ExecutionResult,
}

/// TUI events.
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input.
    Key(KeyEvent),
    /// Mouse input.
    Mouse(crossterm::event::MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
    /// Tick for periodic updates.
    Tick,
    /// Quit signal.
    Quit,
    /// MCP execution completed.
    McpUpdate(McpUpdate),
}

/// Event handler for the TUI.
#[derive(Debug)]
pub struct EventHandler {
    /// Tick rate for periodic updates.
    tick_rate: Duration,
}

impl EventHandler {
    /// Creates a new event handler.
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Polls for the next event.
    pub async fn next(&self) -> TuiResult<Option<Event>> {
        if event::poll(self.tick_rate).map_err(|e| {
            TuiError::new(TuiErrorKind::EventPoll(format!(
                "Failed to poll events: {}",
                e
            )))
        })? {
            match event::read().map_err(|e| {
                TuiError::new(TuiErrorKind::EventRead(format!(
                    "Failed to read event: {}",
                    e
                )))
            })? {
                CrosstermEvent::Key(key) => {
                    // Check for quit (Ctrl+C or 'q')
                    if key.code == crossterm::event::KeyCode::Char('q')
                        || (key.code == crossterm::event::KeyCode::Char('c')
                            && key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL))
                    {
                        Ok(Some(Event::Quit))
                    } else {
                        Ok(Some(Event::Key(key)))
                    }
                }
                CrosstermEvent::Mouse(mouse) => Ok(Some(Event::Mouse(mouse))),
                CrosstermEvent::Resize(w, h) => Ok(Some(Event::Resize(w, h))),
                _ => Ok(Some(Event::Tick)),
            }
        } else {
            Ok(Some(Event::Tick))
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
