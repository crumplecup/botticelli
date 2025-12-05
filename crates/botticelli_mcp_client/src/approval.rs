//! Approval workflow for sensitive tool operations.

use crate::{McpClientError, McpClientErrorKind, McpClientResult};
use serde_json::Value;
use std::collections::HashSet;
use std::io::Write;
use tracing::{debug, warn};

/// Tool approval policy.
#[derive(Debug, Clone)]
pub enum ApprovalPolicy {
    /// All tools require approval.
    AllToolsRequireApproval,
    /// Specific tools require approval.
    SpecificTools(HashSet<String>),
    /// No approval required (auto-approve all).
    AutoApprove,
}

/// Approval handler trait.
pub trait ApprovalHandler: Send + Sync {
    /// Requests approval for a tool call.
    ///
    /// Returns `true` if approved, `false` if denied.
    fn request_approval(&self, tool_name: &str, args: &Value) -> McpClientResult<bool>;
}

/// Console-based approval handler (prompts user via stdin).
#[derive(Debug)]
pub struct ConsoleApprovalHandler;

impl ApprovalHandler for ConsoleApprovalHandler {
    fn request_approval(&self, tool_name: &str, args: &Value) -> McpClientResult<bool> {
        println!("\n🔒 Approval Required");
        println!("Tool: {tool_name}");
        println!("Arguments: {}", serde_json::to_string_pretty(args).unwrap_or_default());
        print!("Approve? (y/n): ");
        std::io::stdout().flush().ok();

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| McpClientError::new(McpClientErrorKind::ConnectionError(format!("Failed to read input: {e}"))))?;

        Ok(input.trim().eq_ignore_ascii_case("y"))
    }
}

/// Approval manager for MCP client.
pub struct ApprovalManager {
    /// Approval policy.
    policy: ApprovalPolicy,
    /// Approval handler.
    handler: Box<dyn ApprovalHandler>,
}

impl ApprovalManager {
    /// Creates a new approval manager.
    #[must_use]
    pub fn new(policy: ApprovalPolicy, handler: Box<dyn ApprovalHandler>) -> Self {
        Self { policy, handler }
    }

    /// Creates an auto-approve manager (for testing/development).
    #[must_use]
    pub fn auto_approve() -> Self {
        Self {
            policy: ApprovalPolicy::AutoApprove,
            handler: Box::new(ConsoleApprovalHandler),
        }
    }

    /// Checks if a tool call requires approval.
    #[must_use]
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        match &self.policy {
            ApprovalPolicy::AllToolsRequireApproval => true,
            ApprovalPolicy::SpecificTools(tools) => tools.contains(tool_name),
            ApprovalPolicy::AutoApprove => false,
        }
    }

    /// Requests approval for a tool call.
    pub fn request_approval(&self, tool_name: &str, args: &Value) -> McpClientResult<bool> {
        if !self.requires_approval(tool_name) {
            debug!(tool = tool_name, "Tool auto-approved");
            return Ok(true);
        }

        debug!(tool = tool_name, "Requesting approval");
        let approved = self.handler.request_approval(tool_name, args)?;

        if approved {
            debug!(tool = tool_name, "Tool call approved");
        } else {
            warn!(tool = tool_name, "Tool call denied");
        }

        Ok(approved)
    }
}
