//! Command-line interface module.
//!
//! This module provides the CLI structure and command handlers for the botticelli binary.

mod commands;
mod content;
mod run;
#[cfg(feature = "server")]
mod server;
mod tui_handler;

pub use commands::{Cli, Commands};
#[cfg(feature = "server")]
pub use commands::ServerCommands;
pub use content::handle_content_command;
pub use run::run_narrative;
#[cfg(feature = "server")]
pub use server::handle_server_command;
pub use tui_handler::launch_tui;
#[cfg(all(feature = "tui", feature = "server"))]
pub use tui_handler::launch_server_tui;
