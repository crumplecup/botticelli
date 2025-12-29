//! Server info tool types.
//!
//! Provides server metadata and version information.

use chrono::Utc;
use rmcp::schemars::JsonSchema;
use serde::Serialize;

/// Result from the server_info tool.
///
/// Contains server metadata including name, version, and timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ServerInfoResult {
    /// Server name.
    pub name: String,
    
    /// Server version.
    pub version: String,
    
    /// ISO 8601 timestamp when info was retrieved.
    pub timestamp: String,
    
    /// Number of tools available.
    pub tool_count: usize,
}

impl ServerInfoResult {
    /// Create a new server info result with current timestamp.
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    /// * `version` - Server version
    /// * `tool_count` - Number of available tools
    ///
    /// # Returns
    ///
    /// A `ServerInfoResult` with the provided data and current timestamp.
    pub fn new(name: String, version: String, tool_count: usize) -> Self {
        Self {
            name,
            version,
            timestamp: Utc::now().to_rfc3339(),
            tool_count,
        }
    }
}
