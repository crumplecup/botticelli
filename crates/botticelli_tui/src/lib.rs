//! Terminal User Interface for content review.
//!
//! Provides an interactive TUI for reviewing, editing, and managing generated content
//! stored in custom tables. Built with ratatui for terminal rendering.

mod app;
mod error;
mod events;
mod ui;
mod views;

pub use app::{App, AppMode, ContentRow, EditBuffer, EditField, run_tui};
pub use error::{TuiError, TuiErrorKind};
pub use events::{Event, EventHandler};
