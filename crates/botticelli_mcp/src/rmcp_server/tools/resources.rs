//! MCP resource tools generated from McpResource trait.
//!
//! This module uses `#[elicit_trait_tools_router]` to automatically generate
//! MCP tools from the McpResource trait implementation on ResourceRegistry.

use crate::rmcp_server::BotticelliServer;
use botticelli_interface::{McpResource, ReadParams, ReadResult};
use elicitation_macros::elicit_trait_tools_router;
use rmcp::tool_router;

/// MCP tools for resource operations.
///
/// Generated tool router function: `resources_tool_router()`
///
/// Tools:
/// - `read` - Read any resource by URI (routes to appropriate resource internally)
///
/// The ResourceRegistry implements McpResource and routes requests to registered
/// resources based on URI pattern matching. This provides a unified interface
/// for all resources through a single MCP tool.
///
/// Note: The #[tool_router] macro generates the `resources_tool_router()` function
/// without documentation attributes. Since we cannot modify the macro-generated
/// code, we use #[allow(missing_docs)] as an explicit exception to the crate's
/// missing_docs lint. This is acceptable for macro-generated code where
/// documentation would need to be added by the macro itself (upstream fix).
#[allow(missing_docs)]
#[elicit_trait_tools_router(McpResource, resource_registry, [read])]
#[tool_router(router = resources_tool_router, vis = "pub")]
impl BotticelliServer {}
