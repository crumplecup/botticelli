//! MCP tool implementations organized by category.

mod core;
mod discord;
mod elicitation;
mod execution;
mod narrative;
mod scene;

use crate::rmcp_server::BotticelliServer;
use rmcp::tool_router;

// Empty impl block required for #[tool_router] macro
impl BotticelliServer {}

// This macro gathers all tool methods from the separate module files
#[tool_router]
impl BotticelliServer {}
