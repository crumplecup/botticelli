//! In-process transport for MCP client/server communication.
//!
//! ## Migration Status
//!
//! This module is deprecated during the rmcp migration. The rmcp library
//! provides its own transport abstraction via `rmcp::service::Service` and
//! doesn't require a custom in-process transport implementation.
//!
//! The old pmcp-based InProcTransport will be replaced with rmcp's native
//! service model once the migration is complete.

#![allow(dead_code)] // Temporarily allow during migration

use async_trait::async_trait;
use super::{McpTransport, McpTransportError, McpTransportErrorKind};
use botticelli_core::ToolDefinition;
use serde_json::Value;
use tracing::warn;

/// Deprecated in-process transport (pmcp-based).
///
/// This type is no longer functional and exists only for compilation
/// during the rmcp migration. Use `rmcp::service::Service` instead.
#[derive(Debug, Clone)]
pub struct InProcTransport;

impl InProcTransport {
    /// Create a pair of connected in-process transports.
    ///
    /// ## Deprecated
    ///
    /// This function is deprecated. Use rmcp's native service model instead.
    pub fn pair() -> (Self, Self) {
        warn!("InProcTransport::pair() deprecated during rmcp migration");
        (Self, Self)
    }

    /// Spawn a server task.
    ///
    /// ## Deprecated
    ///
    /// This function is deprecated. Use `rmcp::service::ServiceExt::serve()` instead.
    pub fn spawn_server<S>(_server: S, _transport: Self) -> InProcServerHandle {
        warn!("InProcTransport::spawn_server() deprecated during rmcp migration");
        InProcServerHandle
    }
}

#[async_trait]
impl McpTransport for InProcTransport {
    async fn initialize(&mut self) -> Result<(), McpTransportError> {
        Err(McpTransportError::new(McpTransportErrorKind::NotInitialized))
    }

    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpTransportError> {
        Err(McpTransportError::new(McpTransportErrorKind::NotInitialized))
    }

    async fn call_tool(&self, _name: &str, _arguments: Value) -> Result<Value, McpTransportError> {
        Err(McpTransportError::new(McpTransportErrorKind::NotInitialized))
    }

    fn is_connected(&self) -> bool {
        false
    }
}

/// Handle to a spawned in-process server.
///
/// ## Deprecated
///
/// This type is deprecated during the rmcp migration.
#[derive(Debug)]
pub struct InProcServerHandle;
