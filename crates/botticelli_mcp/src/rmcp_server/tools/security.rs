//! MCP tool wrappers for security trait methods.
//!
//! Provides tool access to trait implementations that cannot have #[tool] directly.

use crate::rmcp_server::BotticelliServer;
use anyhow::Result;
use botticelli_security::{CommandValidator, DiscordValidator};
use rmcp::tool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameters for Discord validator validate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateDiscordParams {
    /// Command to validate
    pub command: String,
    /// Command parameters
    pub params: HashMap<String, String>,
}

impl BotticelliServer {
    /// Validate Discord command parameters using CommandValidator trait.
    ///
    /// Wraps DiscordValidator::validate() trait method to provide MCP tool access.
    #[tool]
    #[tracing::instrument(skip(self, params), fields(command = %params.command, param_count = params.params.len()))]
    pub fn validate_discord_command(&self, params: ValidateDiscordParams) -> Result<()> {
        let validator = DiscordValidator::new();
        validator.validate(&params.command, &params.params)?;
        Ok(())
    }
}
