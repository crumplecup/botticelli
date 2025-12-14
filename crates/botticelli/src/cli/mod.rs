//! Command-line interface module.
//!
//! This module provides the CLI structure and command handlers for the botticelli binary.

mod commands;
mod content;
#[cfg(feature = "mcp")]
mod mcp;
mod run;
#[cfg(feature = "bots")]
mod server;
mod tui_handler;
mod validate;

#[cfg(feature = "mcp")]
pub use commands::McpCommandArgs;
pub use commands::{Cli, Commands, ValidationOutputFormat};
pub use content::handle_content_command;
#[cfg(feature = "mcp")]
pub use mcp::handle_mcp_command;
#[cfg(not(feature = "gemini"))]
pub use run::run_narrative;
#[cfg(feature = "gemini")]
pub use run::{ExecutionOptions, NarrativeSource, run_narrative};
#[cfg(feature = "bots")]
pub use server::handle_server_command;
pub use tui_handler::launch_tui;
pub use validate::handle_validate_command;
