//! Terminal User Interface for content review.
//!
//! Provides an interactive TUI for reviewing, editing, and managing generated content
//! stored in custom tables. Built with ratatui for terminal rendering.

mod app;
mod backend;
#[cfg(feature = "database")]
mod database_backend;
mod error;
mod events;
#[cfg(feature = "database")]
mod runner;
#[cfg(feature = "database")]
mod ui;
mod views;

pub use app::{App, AppMode, ContentRow, EditBuffer, EditField};
pub use backend::TuiBackend;
#[cfg(feature = "database")]
pub use database_backend::DatabaseBackend;
pub use error::{TuiError, TuiErrorKind, TuiResult};
pub use events::{Event, EventHandler};
#[cfg(feature = "database")]
pub use runner::run_tui;
