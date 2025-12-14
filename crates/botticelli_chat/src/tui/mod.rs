// TUI Module - Modular redesign
//
// This module contains the new modular TUI architecture.
// The old implementation is in legacy.rs for backwards compatibility.

mod app;
mod state;
mod events;
mod commands;

pub mod tabs;
pub mod widgets;

// Legacy TUI (single-file implementation - backwards compatible)
mod legacy;

// New modular API
pub use app::TuiApp;
pub use state::{AppState, Tab, StatusInfo, Modal};
pub use commands::{TuiCommand, Action};
pub use events::{EventHandler, EventResult};

// Re-export legacy API for backwards compatibility
pub use legacy::{restore_terminal, setup_terminal, TuiInterface};
