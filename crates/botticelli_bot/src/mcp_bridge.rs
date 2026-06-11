//! MCP bridge — stub pending Phase 7 rewrite to use BotticelliClient.

use derive_more::{Display, Error};

/// Bridge between Discord and the Botticelli MCP server.
///
/// Stub: Phase 7 will rewrite this to use `botticelli_mcp_client::BotticelliClient`.
#[derive(Clone)]
pub struct DiscordMcpBridge;

/// Specific error conditions for the Discord MCP bridge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum DiscordMcpBridgeErrorKind {
    /// Connection to server failed.
    #[display("Connection error: {}", _0)]
    ConnectionError(String),
}

/// Error type for Discord MCP bridge operations.
#[derive(Debug, Clone, Display, Error)]
#[display("Discord MCP Bridge: {} at {}:{}", kind, file, line)]
pub struct DiscordMcpBridgeError {
    /// The specific error kind.
    pub kind: DiscordMcpBridgeErrorKind,
    /// Line number where error occurred.
    pub line: u32,
    /// File where error occurred.
    pub file: &'static str,
}
