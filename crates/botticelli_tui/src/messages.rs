use crossterm::event::KeyEvent;

use crate::{AppState, ViewMode};

/// User actions from UI thread to state manager
#[derive(Debug, Clone)]
pub enum UserAction {
    /// Key press event
    KeyPress(KeyEvent),
    
    /// Send current message
    SendMessage,
    
    /// Change view mode
    ChangeView(ViewMode),
    
    /// Quit application
    Quit,
}

/// State updates from state manager to UI thread
#[derive(Debug, Clone)]
pub struct StateUpdate {
    /// New application state
    pub state: AppState,
}

/// Background tick events
#[derive(Debug, Clone, Copy)]
pub enum TickEvent {
    /// Regular tick for animations
    Tick,
    
    /// Refresh data from server
    Refresh,
}
