use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};

use crate::{TuiError, TuiErrorKind, TuiResult};

/// TUI events.
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input.
    Key(KeyEvent),
    /// Terminal resize.
    Resize(u16, u16),
    /// Tick for periodic updates.
    Tick,
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
    pub fn next(&self) -> TuiResult<Event> {
        if event::poll(self.tick_rate).map_err(|e| {
            TuiError::new(TuiErrorKind::EventPoll(format!("Failed to poll events: {}", e)))
        })? {
            match event::read().map_err(|e| {
                TuiError::new(TuiErrorKind::EventRead(format!("Failed to read event: {}", e)))
            })? {
                CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                CrosstermEvent::Resize(w, h) => Ok(Event::Resize(w, h)),
                _ => Ok(Event::Tick),
            }
        } else {
            Ok(Event::Tick)
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
