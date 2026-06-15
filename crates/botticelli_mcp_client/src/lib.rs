//! Thin rmcp client for the Botticelli MCP server.
//!
//! This crate connects the TUI (or any user-facing frontend) to the
//! `botticelli_mcp` server. The server owns all state, orchestration, and
//! tool implementations. This crate is a view + input relay.
//!
//! # Usage
//!
//! ```no_run
//! use botticelli_mcp_client::BotticelliClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = BotticelliClient::connect_http("http://localhost:3000/mcp").await?;
//! let tools = client.list_tools().await?;
//! println!("Available tools: {}", tools.tools.len());
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod adapter;
mod connection;
mod error;
mod handler;

pub use adapter::ServerDriverAdapter;
pub use connection::BotticelliClient;
pub use error::{McpClientError, McpClientErrorKind, McpClientResult};
pub use handler::TuiHandler;
