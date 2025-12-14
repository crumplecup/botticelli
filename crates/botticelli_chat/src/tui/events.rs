//! Event handling.

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

/// Poll for events with timeout.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Check if event is a quit command (Ctrl+C or Ctrl+Q).
pub fn is_quit_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(key) if (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('q'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
    )
}
