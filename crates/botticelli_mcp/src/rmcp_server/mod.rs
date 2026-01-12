//! RMCP-based MCP server implementation.
//!
//! The BotticelliServer struct holds all tool implementations and state
//! needed for MCP operations.

mod handler;
mod helpers;
mod server;
mod tools;

pub use server::{BotticelliServer, BotticelliServerBuilder};
