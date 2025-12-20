// TUI Module - Modular redesign
//
// This module contains the new modular TUI architecture.
// The old implementation is in legacy.rs for backwards compatibility.

mod app;
mod commands;
mod events;
mod state;

pub mod tabs;
pub mod widgets;

// Legacy TUI (single-file implementation - backwards compatible)
mod legacy;

// New modular API
pub use app::TuiApp;
pub use commands::TuiCommand;
pub use events::{EventHandler, EventResult};
pub use state::AppState;

// Re-export legacy API for backwards compatibility
pub use legacy::TuiInterface;
