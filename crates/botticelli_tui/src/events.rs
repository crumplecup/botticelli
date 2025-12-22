use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use tokio::sync::mpsc;

use crate::TuiResult;

/// Update from MCP execution.
#[derive(Debug, Clone)]
pub struct McpUpdate {
    /// Conversation ID this update belongs to
    pub conversation_id: uuid::Uuid,
    /// Original user message that triggered this execution
    pub user_message: String,
    /// Assistant's response
    pub assistant_message: String,
}

/// Error from MCP execution in a conversation.
#[derive(Debug, Clone)]
pub struct McpConversationError {
    /// Conversation ID this error belongs to
    pub conversation_id: uuid::Uuid,
    /// Original user message that triggered this execution
    pub user_message: String,
    /// Error message
    pub error: String,
}

/// Message from MCP execution (success or error).
#[derive(Debug, Clone)]
pub enum McpMessage {
    /// Execution completed successfully.
    Update(McpUpdate),
    /// Execution failed with error.
    Error(McpConversationError),
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
    /// MCP execution completed successfully.
    McpUpdate(McpUpdate),
    /// MCP execution failed.
    McpError(McpConversationError),
}

/// Event handler for the TUI.
pub struct EventHandler {
    /// Event receiver channel.
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Creates a new event handler with event-driven input.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Spawn blocking task for crossterm events (immediate, no polling delay)
        let event_tx = tx.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(true) = event::poll(Duration::from_millis(10)) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            // Check for quit (Ctrl+C or 'q')
                            let evt = if key.code == crossterm::event::KeyCode::Char('q')
                                || (key.code == crossterm::event::KeyCode::Char('c')
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL))
                            {
                                Event::Quit
                            } else {
                                Event::Key(key)
                            };
                            if event_tx.send(evt).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            if event_tx.send(Event::Mouse(mouse)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if event_tx.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                        _ => {}
                    }
                }
            }
        });

        // Spawn tick task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tx.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        Self { rx }
    }

    /// Gets the next event (non-blocking with async).
    pub async fn next(&mut self) -> TuiResult<Option<Event>> {
        Ok(self.rx.recv().await)
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
