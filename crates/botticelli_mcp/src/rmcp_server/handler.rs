//! ServerHandler trait implementation.

use super::server::BotticelliServer;
use rmcp::model::ServerCapabilities;
use rmcp::{ServerHandler, tool_handler};

#[tool_handler]
impl ServerHandler for BotticelliServer {
    fn get_info(&self) -> rmcp::model::InitializeResult {
        rmcp::model::InitializeResult {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "botticelli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Botticelli".to_string()),
                website_url: None,
                icons: None,
            },
            instructions: Some("Botticelli MCP server - LLM orchestration tools".to_string()),
        }
    }
}
