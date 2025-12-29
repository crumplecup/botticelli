//! Echo tool types.
//!
//! The echo tool provides a simple test endpoint that returns
//! the input message with a timestamp, useful for testing MCP connectivity.

use chrono::Utc;
use derive_getters::Getters;
use derive_new::new;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for the echo tool.
///
/// This tool echoes back the provided message with a timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct EchoParams {
    /// The message to echo back.
    ///
    /// This can be any UTF-8 string. The server will return it
    /// unchanged along with a timestamp.
    pub message: String,
}

/// Result from the echo tool.
///
/// Contains the echoed message and the timestamp when it was processed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct EchoResult {
    /// The echoed message (same as input).
    pub echo: String,
    
    /// ISO 8601 timestamp when the echo was processed.
    pub timestamp: String,
}

impl EchoResult {
    /// Create a new echo result with current timestamp.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to include in the result
    ///
    /// # Returns
    ///
    /// An `EchoResult` with the message and current timestamp.
    pub fn new(message: String) -> Self {
        Self {
            echo: message,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}
