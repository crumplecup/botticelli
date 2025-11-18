//! Command-line interface module.
//!
//! This module provides the CLI structure and command handlers for the botticelli binary.

mod commands;
mod run;
mod content;
mod tui_handler;

pub use commands::{Cli, Commands};
pub use run::run_narrative;
pub use content::handle_content_command;
pub use tui_handler::launch_tui;
