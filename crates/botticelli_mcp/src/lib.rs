//! Model Context Protocol (MCP) server for Botticelli.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod server;
mod tools;

pub use error::{McpError, McpResult};
pub use server::{McpServer, McpServerBuilder};
pub use tools::ToolRegistry;
