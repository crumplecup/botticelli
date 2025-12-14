//! Terminal User Interface for Botticelli.
//!
//! Provides an interactive TUI for chat, narrative management, and settings.
//! Built with ratatui for terminal rendering.

mod app;
mod commands;
mod error;
mod events;
mod state;
mod view;

pub use app::App;
pub use commands::Command;
pub use error::{TuiError, TuiErrorKind, TuiResult};
pub use events::{Event, EventHandler};
pub use state::{AppState, ChatMessage, ConversationId, NarrativeId, ViewMode};
pub use view::{ChatView, NarrativeBrowserView, NarrativeEditorView, View};
