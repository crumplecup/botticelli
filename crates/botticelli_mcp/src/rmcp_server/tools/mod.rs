//! MCP tool implementations organized by category.

mod core;
mod discord;
mod elicitation;
mod execution;
mod library;
#[cfg(any(feature = "gemini", feature = "anthropic", feature = "groq", feature = "huggingface", feature = "ollama"))]
mod models;
mod narrative;
mod rate_limit;
mod scene;
mod storage;

use crate::rmcp_server::BotticelliServer;
use rmcp::tool_router;

// Empty impl block required for #[tool_router] macro
impl BotticelliServer {}

// This macro gathers all tool methods from the separate module files
#[tool_router]
impl BotticelliServer {}
