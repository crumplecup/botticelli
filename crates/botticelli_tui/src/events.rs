//! Event handling for TUI.

use crate::{TuiError, TuiErrorKind};
use botticelli_error::{BotticelliError, BotticelliResult};
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Event types for the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Tick event for periodic updates
    Tick,
    /// Key press event
    Key(KeyEvent),
}

/// Event handler that polls for terminal events.
pub struct EventHandler {
    /// Tick rate in milliseconds
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with specified tick rate in milliseconds.
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Get the next event, blocking until an event is available or timeout.
    #[tracing::instrument(skip(self))]
    pub fn next(&self) -> BotticelliResult<Option<Event>> {
        if event::poll(self.tick_rate).map_err(|e| {
            BotticelliError::from(TuiError::new(TuiErrorKind::EventPoll(e.to_string())))
        })? {
            match event::read().map_err(|e| {
                BotticelliError::from(TuiError::new(TuiErrorKind::EventRead(e.to_string())))
            })? {
                CrosstermEvent::Key(key) => Ok(Some(Event::Key(key))),
                _ => Ok(None),
            }
        } else {
            Ok(Some(Event::Tick))
        }
    }
}
