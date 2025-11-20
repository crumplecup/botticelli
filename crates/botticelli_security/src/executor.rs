//! Secure command executor with multi-layer security pipeline.

use crate::{
    ApprovalWorkflow, CommandValidator, ContentFilter, PermissionChecker, RateLimiter,
    SecurityResult,
};
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
pub struct SecureExecutor<V: CommandValidator> {
    permission_checker: PermissionChecker,
    validator: V,
    content_filter: ContentFilter,
    rate_limiter: RateLimiter,
    approval_workflow: ApprovalWorkflow,
}

impl<V: CommandValidator> SecureExecutor<V> {
    /// Create a new secure executor.
    pub fn new(
        permission_checker: PermissionChecker,
        validator: V,
        content_filter: ContentFilter,
        rate_limiter: RateLimiter,
        approval_workflow: ApprovalWorkflow,
    ) -> Self {
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

    /// Get the permission checker.
    pub fn permission_checker(&self) -> &PermissionChecker {
        &self.permission_checker
    }

    /// Get the validator.
    pub fn validator(&self) -> &V {
        &self.validator
    }

    /// Get the content filter.
    pub fn content_filter(&self) -> &ContentFilter {
        &self.content_filter
    }

    /// Get the rate limiter.
    pub fn rate_limiter(&mut self) -> &mut RateLimiter {
        &mut self.rate_limiter
    }

    /// Get the approval workflow.
    pub fn approval_workflow(&mut self) -> &mut ApprovalWorkflow {
        &mut self.approval_workflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        validation::DiscordValidator, ContentFilterConfig, PermissionConfig, RateLimit,
        ResourcePermission,
    };

    fn create_test_executor() -> SecureExecutor<DiscordValidator> {
        let mut perm_config = PermissionConfig::default();
        perm_config
            .allowed_commands
            .insert("messages.send".to_string());

        let mut resource_perm = ResourcePermission::default();
        resource_perm
            .allowed_ids
            .insert("123456789012345678".to_string());
        perm_config
            .resources
            .insert("channel".to_string(), resource_perm);

        let permission_checker = PermissionChecker::new(perm_config);
        let validator = DiscordValidator::new();
        let content_filter = ContentFilter::new(ContentFilterConfig::default()).unwrap();
        let mut rate_limiter = RateLimiter::new();
        rate_limiter.add_limit("messages.send", RateLimit::strict(10, 60));
        let approval_workflow = ApprovalWorkflow::new();

        SecureExecutor::new(
            permission_checker,
            validator,
            content_filter,
            rate_limiter,
            approval_workflow,
        )
    }

    #[test]
    fn test_security_pipeline_success() {
        let mut executor = create_test_executor();
        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "123456789012345678".to_string());
        params.insert("content".to_string(), "Hello, world!".to_string());

        let result = executor.check_security("narrative1", "messages.send", &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_permission_denied() {
        let mut executor = create_test_executor();
        let params = HashMap::new();

        let result = executor.check_security("narrative1", "forbidden.command", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_failed() {
        let mut executor = create_test_executor();
        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "invalid".to_string());
        params.insert("content".to_string(), "Hello".to_string());

        let result = executor.check_security("narrative1", "messages.send", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_content_filter_violation() {
        let mut executor = create_test_executor();
        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "123456789012345678".to_string());
        params.insert("content".to_string(), "Hello @everyone".to_string());

        let result = executor.check_security("narrative1", "messages.send", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let mut executor = create_test_executor();
        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "123456789012345678".to_string());
        params.insert("content".to_string(), "Hello".to_string());

        // Exhaust rate limit
        for _ in 0..10 {
            executor
                .check_security("narrative1", "messages.send", &params)
                .unwrap();
        }

        // Should fail on 11th attempt
        let result = executor.check_security("narrative1", "messages.send", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_approval_required() {
        let mut executor = create_test_executor();
        executor
            .approval_workflow
            .set_requires_approval("messages.send", true);

        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "123456789012345678".to_string());
        params.insert("content".to_string(), "Hello".to_string());

        let result = executor.check_security("narrative1", "messages.send", &params);
        assert!(result.is_ok());
        
        // Should return action ID
        let action_id = result.unwrap();
        assert!(action_id.is_some());
    }

    #[test]
    fn test_approved_action() {
        let mut executor = create_test_executor();
        executor
            .approval_workflow
            .set_requires_approval("messages.send", true);

        let mut params = HashMap::new();
        params.insert("channel_id".to_string(), "123456789012345678".to_string());
        params.insert("content".to_string(), "Hello".to_string());

        // Create pending action
        let result = executor.check_security("narrative1", "messages.send", &params);
        let action_id = result.unwrap().unwrap();

        // Approve action
        executor
            .approval_workflow
            .approve_action(&action_id, "admin", None)
            .unwrap();

        // Try again with approval ID
        params.insert("_approval_action_id".to_string(), action_id.clone());
        let result = executor.check_security("narrative1", "messages.send", &params);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // No new action ID
    }
}
