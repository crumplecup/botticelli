//! Terminal user interface module.
//!
//! Multi-pane TUI with tabs for narratives, bots, database, chat, schedule, and settings.

mod app;
mod state;
mod events;
mod commands;

// Tabs
mod tabs;

// Widgets
mod widgets;

// Public API
pub use app::App;
pub use state::{AppState, Tab};
