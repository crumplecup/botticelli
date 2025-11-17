//! Terminal User Interface for content review.
//!
//! Provides an interactive TUI for reviewing, editing, and managing generated content
//! stored in custom tables. Built with ratatui for terminal rendering.

mod app;
mod events;
mod ui;
mod views;

pub use app::{App, AppMode, run_tui};
