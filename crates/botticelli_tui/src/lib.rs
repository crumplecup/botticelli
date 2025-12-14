//! Terminal User Interface for Botticelli.
//!
//! Provides an interactive TUI for chat, narrative management, and settings.
//! Built with ratatui for terminal rendering.

mod commands;
mod error;
mod events;
mod state;
mod tui;
mod view;

pub use commands::Command;
pub use error::{TuiError, TuiErrorKind, TuiResult};
pub use events::{Event, EventHandler};
pub use state::{AppState, ChatMessage, ConversationId, NarrativeId, ViewMode};
pub use tui::Tui;
pub use view::{ChatView, NarrativeBrowserView, NarrativeEditorView, View};
