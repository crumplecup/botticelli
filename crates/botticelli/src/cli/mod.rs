//! Command-line interface module.
//!
//! This module provides the CLI structure and command handlers for the botticelli binary.

mod commands;
mod content;
mod run;
mod server;
mod tui_handler;

pub use commands::{Cli, Commands};
pub use content::handle_content_command;
#[cfg(not(feature = "gemini"))]
pub use run::run_narrative;
#[cfg(feature = "gemini")]
pub use run::{ExecutionOptions, NarrativeSource, run_narrative};
pub use server::handle_server_command;
pub use tui_handler::launch_tui;
