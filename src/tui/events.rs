//! Event handling for TUI.

use crate::{BoticelliError, BoticelliResult, ConfigError};
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Event types for the TUI.
#[derive(Debug)]
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
    pub fn next(&self) -> BoticelliResult<Option<Event>> {
        if event::poll(self.tick_rate).map_err(|e| {
            BoticelliError::from(ConfigError::new(format!(
                "Failed to poll for events: {}",
                e
            )))
        })? {
            match event::read().map_err(|e| {
                BoticelliError::from(ConfigError::new(format!(
                    "Failed to read event: {}",
                    e
                )))
            })? {
                CrosstermEvent::Key(key) => Ok(Some(Event::Key(key))),
                _ => Ok(None),
            }
        } else {
            Ok(Some(Event::Tick))
        }
    }
}
