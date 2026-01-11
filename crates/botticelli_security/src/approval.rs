//! Approval workflows for dangerous operations.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, instrument};

/// Approval decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Action approved
    Approved,
    /// Action denied
    Denied,
    /// Pending approval
    Pending,
}

/// Pending action awaiting approval.
#[derive(Debug, Clone, Serialize, Deserialize, derive_getters::Getters)]
pub struct PendingAction {
    /// Unique action ID
    id: String,
    /// Narrative ID that requested the action
    narrative_id: String,
    /// Command to execute
    command: String,
    /// Command parameters
    params: HashMap<String, String>,
    /// Reason for the action (from AI)
    reason: Option<String>,
    /// Timestamp when action was created
    created_at: u64,
    /// Timestamp when action expires (24 hours default)
    expires_at: u64,
    /// Current decision
    decision: ApprovalDecision,
    /// Reason for approval/denial
    decision_reason: Option<String>,
    /// User who made the decision
    decided_by: Option<String>,
}

impl PendingAction {
    /// Create a new pending action.
    #[tracing::instrument(fields(id, narrative_id, command, param_count = params.len()))]
    pub fn new(
        id: impl Into<String>,
        narrative_id: impl Into<String>,
        command: impl Into<String>,
        params: HashMap<String, String>,
        reason: Option<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = now + 24 * 60 * 60; // 24 hours

        let action = Self {
            id: id.into(),
            narrative_id: narrative_id.into(),
            command: command.into(),
            params,
            reason,
            created_at: now,
            expires_at,
            decision: ApprovalDecision::Pending,
            decision_reason: None,
            decided_by: None,
        };
        tracing::debug!(action_id = %action.id, expires_at, "Pending action created");
        action
    }

    /// Check if the action has expired.
    #[tracing::instrument(skip(self), fields(action_id = %self.id))]
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }

    /// Approve the action.
    #[tracing::instrument(skip(self, approved_by), fields(action_id = %self.id))]
    pub fn approve(&mut self, approved_by: impl Into<String>, reason: Option<String>) {
        let approver = approved_by.into();
        tracing::info!(approved_by = %approver, "Action approved");
        self.decision = ApprovalDecision::Approved;
        self.decided_by = Some(approver);
        self.decision_reason = reason;
    }

    /// Deny the action.
    #[tracing::instrument(skip(self, denied_by), fields(action_id = %self.id))]
    pub fn deny(&mut self, denied_by: impl Into<String>, reason: Option<String>) {
        let denier = denied_by.into();
        tracing::info!(denied_by = %denier, "Action denied");
        self.decision = ApprovalDecision::Denied;
        self.decided_by = Some(denier);
        self.decision_reason = reason;
    }
}

/// Approval workflow manager.
#[derive(Debug, Clone, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct ApprovalWorkflow {
    /// Pending actions by ID
    #[setters(doc = "Sets the pending actions")]
    pending: HashMap<String, PendingAction>,
    /// Commands that require approval
    #[setters(doc = "Sets the commands that require approval")]
    requires_approval: HashMap<String, bool>,
}

impl ApprovalWorkflow {
    /// Create a new approval workflow.
    #[tracing::instrument]
    pub fn new() -> Self {
        tracing::debug!("Creating approval workflow");
        Self {
            pending: HashMap::new(),
            requires_approval: HashMap::new(),
        }
    }

    /// Configure whether a command requires approval.
    #[tracing::instrument(skip(self), fields(command, required))]
    pub fn set_requires_approval(&mut self, command: impl Into<String>, required: bool) {
        let cmd = command.into();
        tracing::debug!(command = %cmd, required, "Setting approval requirement");
        self.requires_approval.insert(cmd, required);
    }

    /// Check if a command requires approval.
    #[tracing::instrument(skip(self), fields(command))]
    pub fn requires_approval(&self, command: &str) -> bool {
        self.requires_approval
            .get(command)
            .copied()
            .unwrap_or(false)
    }

    /// Create a pending action and return its ID.
    #[instrument(skip(self, params), fields(narrative_id, command))]
    pub fn create_pending_action(
        &mut self,
        narrative_id: impl Into<String>,
        command: impl Into<String>,
        params: HashMap<String, String>,
        reason: Option<String>,
    ) -> SecurityResult<String> {
        let narrative_id = narrative_id.into();
        let command = command.into();

        // Generate unique ID
        let id = format!(
            "{}-{}-{}",
            narrative_id,
            command,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        debug!(action_id = %id, "Creating pending action");

        let action = PendingAction::new(id.clone(), narrative_id, command, params, reason);
        self.pending.insert(id.clone(), action);

        Ok(id)
    }

    /// Get a pending action by ID.
    #[tracing::instrument(skip(self), fields(action_id = id))]
    pub fn get_pending_action(&self, id: &str) -> Option<&PendingAction> {
        self.pending.get(id)
    }

    /// Get a mutable pending action by ID.
    #[tracing::instrument(skip(self), fields(action_id = id))]
    pub fn get_pending_action_mut(&mut self, id: &str) -> Option<&mut PendingAction> {
        self.pending.get_mut(id)
    }

    /// List all pending actions for a narrative.
    #[tracing::instrument(skip(self), fields(narrative_id))]
    pub fn list_pending_actions(&self, narrative_id: &str) -> Vec<&PendingAction> {
        self.pending
            .values()
            .filter(|a| a.narrative_id == narrative_id && a.decision == ApprovalDecision::Pending)
            .collect()
    }

    /// Approve a pending action.
    #[instrument(skip(self), fields(action_id, approved_by))]
    pub fn approve_action(
        &mut self,
        action_id: &str,
        approved_by: impl Into<String>,
        reason: Option<String>,
    ) -> SecurityResult<()> {
        let action = self.pending.get_mut(action_id).ok_or_else(|| {
            SecurityError::new(SecurityErrorKind::Configuration(format!(
                "Action '{}' not found",
                action_id
            )))
        })?;

        if action.is_expired() {
            debug!("Action has expired");
            return Err(SecurityError::new(SecurityErrorKind::ApprovalDenied {
                action_id: action_id.to_string(),
                reason: "Action has expired".to_string(),
            }));
        }

        debug!("Approving action");
        action.approve(approved_by, reason);
        Ok(())
    }

    /// Deny a pending action.
    #[instrument(skip(self), fields(action_id, denied_by))]
    pub fn deny_action(
        &mut self,
        action_id: &str,
        denied_by: impl Into<String>,
        reason: Option<String>,
    ) -> SecurityResult<()> {
        let action = self.pending.get_mut(action_id).ok_or_else(|| {
            SecurityError::new(SecurityErrorKind::Configuration(format!(
                "Action '{}' not found",
                action_id
            )))
        })?;

        debug!("Denying action");
        action.deny(denied_by, reason);
        Ok(())
    }

    /// Check if an action is approved and ready to execute.
    #[instrument(skip(self), fields(action_id))]
    pub fn check_approval(&self, action_id: &str) -> SecurityResult<()> {
        let action = self.pending.get(action_id).ok_or_else(|| {
            SecurityError::new(SecurityErrorKind::Configuration(format!(
                "Action '{}' not found",
                action_id
            )))
        })?;

        if action.is_expired() {
            debug!("Action has expired");
            return Err(SecurityError::new(SecurityErrorKind::ApprovalDenied {
                action_id: action_id.to_string(),
                reason: "Action has expired".to_string(),
            }));
        }

        match action.decision {
            ApprovalDecision::Approved => {
                debug!("Action is approved");
                Ok(())
            }
            ApprovalDecision::Denied => {
                debug!("Action was denied");
                Err(SecurityError::new(SecurityErrorKind::ApprovalDenied {
                    action_id: action_id.to_string(),
                    reason: action
                        .decision_reason
                        .clone()
                        .unwrap_or_else(|| "Action denied".to_string()),
                }))
            }
            ApprovalDecision::Pending => {
                debug!("Action is still pending");
                Err(SecurityError::new(SecurityErrorKind::ApprovalRequired {
                    operation: action.command.clone(),
                    reason: "Action is pending approval".to_string(),
                    action_id: Some(action_id.to_string()),
                }))
            }
        }
    }

    /// Clean up expired actions.
    #[tracing::instrument(skip(self), fields(pending_count = self.pending.len()))]
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.pending.len();
        self.pending.retain(|_, action| !action.is_expired());
        let removed = before - self.pending.len();
        if removed > 0 {
            tracing::info!(
                removed,
                remaining = self.pending.len(),
                "Cleaned up expired actions"
            );
        }
        removed
    }
}

impl Default for ApprovalWorkflow {
    fn default() -> Self {
        Self::new()
    }
}
