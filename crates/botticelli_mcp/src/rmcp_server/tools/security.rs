//! Security primitive delegation wrappers.

use crate::rmcp_server::BotticelliServer;
use botticelli_security::{
    ApprovalWorkflow, CommandValidator, ContentFilter, ContentFilterConfig, DiscordValidator,
    PendingAction, PermissionChecker, RateLimit, ValidationError,
};
use elicitation::Elicit;
use rmcp::tool;
use rmcp::tool_router;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;

impl BotticelliServer {
    // ========================================================================
    // PendingAction primitives
    // ========================================================================

    /// Create a new pending action.
    #[tool]
    #[instrument(skip(self, params), fields(id = %params.id, command = %params.command))]
    pub fn pending_action_new(&self, params: PendingActionNewParams) -> PendingAction {
        tracing::debug!("Delegating to PendingAction::new");
        PendingAction::new(
            params.id,
            params.narrative_id,
            params.command,
            params.params,
            params.reason,
        )
    }

    /// Check if a pending action has expired.
    #[tool]
    #[instrument(skip(self, params), fields(id = %params.action.id()))]
    pub fn pending_action_is_expired(
        &self,
        params: PendingActionIsExpiredParams,
    ) -> PendingActionIsExpiredResult {
        tracing::debug!("Delegating to PendingAction::is_expired");
        let is_expired = params.action.is_expired();
        tracing::debug!(is_expired, "Checked action expiration");
        PendingActionIsExpiredResult { is_expired }
    }

    /// Approve a pending action.
    #[tool]
    #[instrument(skip(self, params), fields(id = %params.action.id(), approved_by = %params.approved_by))]
    pub fn pending_action_approve(
        &self,
        mut params: PendingActionApproveParams,
    ) -> PendingActionApproveResult {
        tracing::debug!("Delegating to PendingAction::approve");
        params.action.approve(params.approved_by, params.reason);
        tracing::debug!("Action approved");
        PendingActionApproveResult {
            action: params.action,
        }
    }

    /// Deny a pending action.
    #[tool]
    #[instrument(skip(self, params), fields(id = %params.action.id(), denied_by = %params.denied_by))]
    pub fn pending_action_deny(
        &self,
        mut params: PendingActionDenyParams,
    ) -> PendingActionDenyResult {
        tracing::debug!("Delegating to PendingAction::deny");
        params.action.deny(params.denied_by, params.reason);
        tracing::debug!("Action denied");
        PendingActionDenyResult {
            action: params.action,
        }
    }

    // ========================================================================
    // ApprovalWorkflow primitives
    // ========================================================================

    /// Create a new approval workflow.
    #[tool]
    #[instrument(skip(self))]
    pub fn approval_workflow_new(&self) -> ApprovalWorkflow {
        tracing::debug!("Delegating to ApprovalWorkflow::new");
        ApprovalWorkflow::new()
    }

    /// Set whether a command requires approval.
    #[tool]
    #[instrument(skip(self, params), fields(command = %params.command, required = params.required))]
    pub fn approval_workflow_set_requires_approval(
        &self,
        mut params: ApprovalWorkflowSetRequiresApprovalParams,
    ) -> ApprovalWorkflowSetRequiresApprovalResult {
        tracing::debug!("Delegating to ApprovalWorkflow::set_requires_approval");
        params
            .workflow
            .set_requires_approval(params.command, params.required);
        tracing::debug!("Approval requirement set");
        ApprovalWorkflowSetRequiresApprovalResult {
            workflow: params.workflow,
        }
    }

    /// Check if a command requires approval.
    #[tool]
    #[instrument(skip(self, params), fields(command = %params.command))]
    pub fn approval_workflow_requires_approval(
        &self,
        params: ApprovalWorkflowRequiresApprovalParams,
    ) -> ApprovalWorkflowRequiresApprovalResult {
        tracing::debug!("Delegating to ApprovalWorkflow::requires_approval");
        let requires = params.workflow.requires_approval(&params.command);
        tracing::debug!(requires, "Checked approval requirement");
        ApprovalWorkflowRequiresApprovalResult { requires }
    }

    /// Create a pending action in the workflow.
    #[tool]
    #[instrument(skip(self, params), fields(command = %params.command))]
    pub fn approval_workflow_create_pending_action(
        &self,
        mut params: ApprovalWorkflowCreatePendingActionParams,
    ) -> Result<ApprovalWorkflowCreatePendingActionResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to ApprovalWorkflow::create_pending_action");
        
        let result = params.workflow.create_pending_action(
            params.narrative_id,
            params.command,
            params.params,
            params.reason,
        );
        
        match &result {
            Ok(action_id) => {
                tracing::debug!(action_id = %action_id, "Pending action created");
                Ok(ApprovalWorkflowCreatePendingActionResult {
                    workflow: params.workflow,
                    action_id: action_id.clone(),
                })
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to create pending action");
                Err(e.clone())
            }
        }
    }

    /// Get a pending action by ID.
    #[tool]
    #[instrument(skip(self, params), fields(id = %params.id))]
    pub fn approval_workflow_get_pending_action(
        &self,
        params: ApprovalWorkflowGetPendingActionParams,
    ) -> ApprovalWorkflowGetPendingActionResult {
        tracing::debug!("Delegating to ApprovalWorkflow::get_pending_action");
        let action = params.workflow.get_pending_action(&params.id).cloned();
        match &action {
            Some(a) => tracing::debug!(action_id = %a.id(), "Action retrieved"),
            None => tracing::debug!("Action not found"),
        }
        ApprovalWorkflowGetPendingActionResult { action }
    }

    // ========================================================================
    // ContentFilter primitives
    // ========================================================================

    /// Create a new content filter.
    #[tool]
    #[instrument(skip(self, params))]
    pub fn content_filter_new(
        &self,
        params: ContentFilterNewParams,
    ) -> Result<ContentFilter, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to ContentFilter::new");

        let result = ContentFilter::new(params.config);

        match &result {
            Ok(_) => tracing::debug!("Content filter created"),
            Err(e) => tracing::error!(error = ?e, "Failed to create content filter"),
        }

        result
    }

    /// Filter content for violations.
    #[tool]
    #[instrument(skip(self, params), fields(content_len = params.content.len()))]
    pub fn content_filter_filter(
        &self,
        params: ContentFilterFilterParams,
    ) -> Result<ContentFilterFilterResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to ContentFilter::filter");

        // Create the filter from config first
        let filter = ContentFilter::new(params.config)?;
        let result = filter.filter(&params.content);

        match &result {
            Ok(_) => tracing::debug!("Content passed filtering"),
            Err(e) => tracing::error!(error = ?e, "Content failed filtering"),
        }

        result.map(|_| ContentFilterFilterResult {})
    }

    // ========================================================================
    // PermissionChecker primitives
    // ========================================================================

    /// Check if a command is permitted.
    #[tool]
    #[instrument(skip(self, params), fields(command = %params.command))]
    pub fn permission_checker_check_command(
        &self,
        params: PermissionCheckerCheckCommandParams,
    ) -> Result<PermissionCheckerCheckCommandResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to PermissionChecker::check_command");

        let result = params.checker.check_command(&params.command);

        match &result {
            Ok(_) => tracing::debug!("Command permitted"),
            Err(e) => tracing::error!(error = ?e, "Command denied"),
        }

        result.map(|_| PermissionCheckerCheckCommandResult {})
    }

    /// Check if a resource access is permitted.
    #[tool]
    #[instrument(skip(self, params), fields(resource_type = %params.resource_type, resource_id = %params.resource_id))]
    pub fn permission_checker_check_resource(
        &self,
        params: PermissionCheckerCheckResourceParams,
    ) -> Result<PermissionCheckerCheckResourceResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to PermissionChecker::check_resource");

        let result = params
            .checker
            .check_resource(&params.resource_type, &params.resource_id);

        match &result {
            Ok(_) => tracing::debug!("Resource access permitted"),
            Err(e) => tracing::error!(error = ?e, "Resource access denied"),
        }

        result.map(|_| PermissionCheckerCheckResourceResult {})
    }

    /// Check if a user is protected.
    #[tool]
    #[instrument(skip(self, params), fields(user_id = %params.user_id))]
    pub fn permission_checker_check_user_protected(
        &self,
        params: PermissionCheckerCheckUserProtectedParams,
    ) -> Result<PermissionCheckerCheckUserProtectedResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to PermissionChecker::check_user_protected");

        let result = params.checker.check_user_protected(&params.user_id);

        match &result {
            Ok(_) => tracing::debug!("User not protected"),
            Err(e) => tracing::error!(error = ?e, "User is protected"),
        }

        result.map(|_| PermissionCheckerCheckUserProtectedResult {})
    }

    /// Check if a role is protected.
    #[tool]
    #[instrument(skip(self, params), fields(role_id = %params.role_id))]
    pub fn permission_checker_check_role_protected(
        &self,
        params: PermissionCheckerCheckRoleProtectedParams,
    ) -> Result<PermissionCheckerCheckRoleProtectedResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to PermissionChecker::check_role_protected");

        let result = params.checker.check_role_protected(&params.role_id);

        match &result {
            Ok(_) => tracing::debug!("Role not protected"),
            Err(e) => tracing::error!(error = ?e, "Role is protected"),
        }

        result.map(|_| PermissionCheckerCheckRoleProtectedResult {})
    }

    // ========================================================================
    // RateLimit primitives
    // ========================================================================

    /// Create a new rate limit.
    #[tool]
    #[instrument(skip(self, params), fields(max_tokens = params.max_tokens, window_secs = params.window_secs, burst = params.burst))]
    pub fn rate_limit_new(&self, params: RateLimitNewParams) -> RateLimit {
        tracing::debug!("Delegating to RateLimit::new");
        RateLimit::new(params.max_tokens, params.window_secs, params.burst)
    }

    /// Create a strict rate limit (no burst).
    #[tool]
    #[instrument(skip(self, params), fields(max_tokens = params.max_tokens, window_secs = params.window_secs))]
    pub fn rate_limit_strict(&self, params: RateLimitStrictParams) -> RateLimit {
        tracing::debug!("Delegating to RateLimit::strict");
        RateLimit::strict(params.max_tokens, params.window_secs)
    }

    // ========================================================================
    // ValidationError primitives
    // ========================================================================

    /// Create a new validation error.
    #[tool]
    #[instrument(skip(self, params), fields(field = %params.field, reason = %params.reason))]
    pub fn validation_error_new(&self, params: ValidationErrorNewParams) -> ValidationError {
        tracing::debug!("Delegating to ValidationError::new");
        ValidationError::new(params.field, params.reason)
    }

    // ========================================================================
    // DiscordValidator primitives
    // ========================================================================

    /// Create a new Discord validator.
    #[tool]
    #[instrument(skip(self))]
    pub fn discord_validator_new(&self) -> DiscordValidator {
        tracing::debug!("Delegating to DiscordValidator::new");
        DiscordValidator::new()
    }

    /// Validate Discord command parameters using CommandValidator trait.
    #[tool]
    #[instrument(skip(self, params), fields(command = %params.command, param_count = params.params.len()))]
    pub fn validate_discord_command(
        &self,
        params: ValidateDiscordParams,
    ) -> Result<ValidateDiscordResult, botticelli_error::SecurityError> {
        tracing::debug!("Delegating to DiscordValidator::validate");
        let validator = DiscordValidator::new();
        
        let result = validator.validate(&params.command, &params.params);
        
        match &result {
            Ok(_) => tracing::debug!("Validation passed"),
            Err(e) => tracing::error!(error = ?e, "Validation failed"),
        }
        
        result.map(|_| ValidateDiscordResult {})
    }
}

// ============================================================================
// DTOs
// ============================================================================

// PendingAction DTOs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct PendingActionNewParams {
    pub id: String,
    pub narrative_id: String,
    pub command: String,
    pub params: HashMap<String, String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActionIsExpiredParams {
    pub action: PendingAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActionIsExpiredResult {
    pub is_expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActionApproveParams {
    pub action: PendingAction,
    pub approved_by: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActionApproveResult {
    pub action: PendingAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActionDenyParams {
    pub action: PendingAction,
    pub denied_by: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActionDenyResult {
    pub action: PendingAction,
}

// ApprovalWorkflow DTOs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowSetRequiresApprovalParams {
    pub workflow: ApprovalWorkflow,
    pub command: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowSetRequiresApprovalResult {
    pub workflow: ApprovalWorkflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowRequiresApprovalParams {
    pub workflow: ApprovalWorkflow,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowRequiresApprovalResult {
    pub requires: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ApprovalWorkflowCreatePendingActionParams {
    pub workflow: ApprovalWorkflow,
    pub narrative_id: String,
    pub command: String,
    pub params: HashMap<String, String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowCreatePendingActionResult {
    pub workflow: ApprovalWorkflow,
    pub action_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowGetPendingActionParams {
    pub workflow: ApprovalWorkflow,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalWorkflowGetPendingActionResult {
    pub action: Option<PendingAction>,
}

// ContentFilter DTOs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ContentFilterNewParams {
    pub config: ContentFilterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ContentFilterFilterParams {
    pub config: ContentFilterConfig,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContentFilterFilterResult {}

// PermissionChecker DTOs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckCommandParams {
    pub checker: PermissionChecker,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckCommandResult {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckResourceParams {
    pub checker: PermissionChecker,
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckResourceResult {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckUserProtectedParams {
    pub checker: PermissionChecker,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckUserProtectedResult {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckRoleProtectedParams {
    pub checker: PermissionChecker,
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PermissionCheckerCheckRoleProtectedResult {}

// RateLimit DTOs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitNewParams {
    pub max_tokens: u32,
    pub window_secs: u64,
    pub burst: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct RateLimitStrictParams {
    pub max_tokens: u32,
    pub window_secs: u64,
}

// ValidationError DTOs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ValidationErrorNewParams {
    pub field: String,
    pub reason: String,
}

// CommandValidator trait wrapper DTOs
/// Parameters for Discord command validation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Elicit)]
pub struct ValidateDiscordParams {
    /// Command to validate
    pub command: String,
    /// Command parameters
    pub params: HashMap<String, String>,
}

/// Result of Discord command validation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateDiscordResult {}
