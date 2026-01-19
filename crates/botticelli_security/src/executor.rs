//! Secure command executor with multi-layer security pipeline.

use crate::{
    ApprovalWorkflow, CommandValidator, ContentFilter, PermissionChecker, RateLimiter,
    SecurityResult,
};
use rmcp::tool;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

/// Secure executor that wraps a command executor with security checks.
///
/// This executor implements a 5-layer security pipeline:
/// 1. Permission check - Verify command and resource permissions
/// 2. Input validation - Validate command parameters
/// 3. Content filtering - Filter AI-generated content
/// 4. Rate limiting - Check rate limits
/// 5. Approval workflow - Check if approval required/granted
///
/// After passing all checks, the command is executed and logged.
///
/// Note: SecureExecutor cannot be cloned because it contains a stateful
/// RateLimiter. Create a new instance if you need separate security contexts.
#[derive(derive_getters::Getters)]
pub struct SecureExecutor<V: CommandValidator> {
    /// Permission checker for command and resource permissions.
    permission_checker: PermissionChecker,

    /// Command validator for input validation.
    validator: V,

    /// Content filter for AI-generated content.
    content_filter: ContentFilter,

    /// Rate limiter for command rate limiting.
    rate_limiter: RateLimiter,

    /// Approval workflow for command approvals.
    approval_workflow: ApprovalWorkflow,
}

impl<V: CommandValidator> SecureExecutor<V> {
    /// Create a new secure executor.
    #[tool]
    #[tracing::instrument(skip_all)]
    pub fn new(
        permission_checker: PermissionChecker,
        validator: V,
        content_filter: ContentFilter,
        rate_limiter: RateLimiter,
        approval_workflow: ApprovalWorkflow,
    ) -> Self {
        tracing::debug!("Creating secure executor");
        Self {
            permission_checker,
            validator,
            content_filter,
            rate_limiter,
            approval_workflow,
        }
    }

    /// Execute a command through the security pipeline.
    ///
    /// Returns Ok(()) if the command passes all security checks and is ready to execute.
    /// Returns Err with specific security error if any check fails.
    #[tool]
    #[instrument(skip(self, params), fields(command, narrative_id))]
    pub fn check_security(
        &mut self,
        narrative_id: &str,
        command: &str,
        params: &HashMap<String, String>,
    ) -> SecurityResult<Option<String>> {
        info!("Starting security pipeline");

        // Layer 1: Permission check
        debug!("Layer 1: Checking permissions");
        self.permission_checker.check_command(command)?;

        // Check resource permissions if applicable
        if let Some(channel_id) = params.get("channel_id") {
            self.permission_checker
                .check_resource("channel", channel_id)?;
        }
        if let Some(user_id) = params.get("user_id") {
            self.permission_checker.check_user_protected(user_id)?;
        }
        if let Some(role_id) = params.get("role_id") {
            self.permission_checker.check_role_protected(role_id)?;
        }

        // Layer 2: Input validation
        debug!("Layer 2: Validating input");
        self.validator.validate(command, params)?;

        // Layer 3: Content filtering
        debug!("Layer 3: Filtering content");
        if let Some(content) = params.get("content") {
            self.content_filter.filter(content)?;
        }

        // Layer 4: Rate limiting
        debug!("Layer 4: Checking rate limits");
        self.rate_limiter.check(command)?;

        // Layer 5: Approval workflow
        debug!("Layer 5: Checking approval requirements");
        if self.approval_workflow.requires_approval(command) {
            // Check if there's an existing approved action
            if let Some(action_id) = params.get("_approval_action_id") {
                self.approval_workflow.check_approval(action_id)?;
                info!(action_id, "Action approved, ready to execute");
                return Ok(None);
            } else {
                // Create pending action
                let action_id = self.approval_workflow.create_pending_action(
                    narrative_id,
                    command,
                    params.clone(),
                    params.get("_approval_reason").cloned(),
                )?;
                warn!(action_id, "Approval required for command");
                return Ok(Some(action_id));
            }
        }

        info!("All security checks passed");
        Ok(None)
    }
}
