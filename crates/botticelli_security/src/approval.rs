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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    /// Unique action ID
    pub id: String,
    /// Narrative ID that requested the action
    pub narrative_id: String,
    /// Command to execute
    pub command: String,
    /// Command parameters
    pub params: HashMap<String, String>,
    /// Reason for the action (from AI)
    pub reason: Option<String>,
    /// Timestamp when action was created
    pub created_at: u64,
    /// Timestamp when action expires (24 hours default)
    pub expires_at: u64,
    /// Current decision
    pub decision: ApprovalDecision,
    /// Reason for approval/denial
    pub decision_reason: Option<String>,
    /// User who made the decision
    pub decided_by: Option<String>,
}

impl PendingAction {
    /// Create a new pending action.
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

        Self {
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
        }
    }

    /// Check if the action has expired.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }

    /// Approve the action.
    pub fn approve(&mut self, approved_by: impl Into<String>, reason: Option<String>) {
        self.decision = ApprovalDecision::Approved;
        self.decided_by = Some(approved_by.into());
        self.decision_reason = reason;
    }

    /// Deny the action.
    pub fn deny(&mut self, denied_by: impl Into<String>, reason: Option<String>) {
        self.decision = ApprovalDecision::Denied;
        self.decided_by = Some(denied_by.into());
        self.decision_reason = reason;
    }
}

/// Approval workflow manager.
pub struct ApprovalWorkflow {
    /// Pending actions by ID
    pending: HashMap<String, PendingAction>,
    /// Commands that require approval
    requires_approval: HashMap<String, bool>,
}

impl ApprovalWorkflow {
    /// Create a new approval workflow.
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            requires_approval: HashMap::new(),
        }
    }

    /// Configure whether a command requires approval.
    pub fn set_requires_approval(&mut self, command: impl Into<String>, required: bool) {
        self.requires_approval.insert(command.into(), required);
    }

    /// Check if a command requires approval.
    pub fn requires_approval(&self, command: &str) -> bool {
        self.requires_approval.get(command).copied().unwrap_or(false)
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
    pub fn get_pending_action(&self, id: &str) -> Option<&PendingAction> {
        self.pending.get(id)
    }

    /// Get a mutable pending action by ID.
    pub fn get_pending_action_mut(&mut self, id: &str) -> Option<&mut PendingAction> {
        self.pending.get_mut(id)
    }

    /// List all pending actions for a narrative.
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
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.pending.len();
        self.pending.retain(|_, action| !action.is_expired());
        let removed = before - self.pending.len();
        if removed > 0 {
            debug!(removed, "Cleaned up expired actions");
        }
        removed
    }
}

impl Default for ApprovalWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pending_action() {
        let mut workflow = ApprovalWorkflow::new();
        let params = HashMap::new();

        let action_id = workflow
            .create_pending_action("narrative1", "test.command", params, None)
            .unwrap();

        let action = workflow.get_pending_action(&action_id).unwrap();
        assert_eq!(action.narrative_id, "narrative1");
        assert_eq!(action.command, "test.command");
        assert_eq!(action.decision, ApprovalDecision::Pending);
    }

    #[test]
    fn test_approve_action() {
        let mut workflow = ApprovalWorkflow::new();
        let params = HashMap::new();

        let action_id = workflow
            .create_pending_action("narrative1", "test.command", params, None)
            .unwrap();

        workflow
            .approve_action(&action_id, "admin", Some("Looks good".to_string()))
            .unwrap();

        let action = workflow.get_pending_action(&action_id).unwrap();
        assert_eq!(action.decision, ApprovalDecision::Approved);
        assert_eq!(action.decided_by, Some("admin".to_string()));
    }

    #[test]
    fn test_deny_action() {
        let mut workflow = ApprovalWorkflow::new();
        let params = HashMap::new();

        let action_id = workflow
            .create_pending_action("narrative1", "test.command", params, None)
            .unwrap();

        workflow
            .deny_action(&action_id, "admin", Some("Not allowed".to_string()))
            .unwrap();

        let action = workflow.get_pending_action(&action_id).unwrap();
        assert_eq!(action.decision, ApprovalDecision::Denied);
    }

    #[test]
    fn test_check_approval_pending() {
        let mut workflow = ApprovalWorkflow::new();
        let params = HashMap::new();

        let action_id = workflow
            .create_pending_action("narrative1", "test.command", params, None)
            .unwrap();

        let result = workflow.check_approval(&action_id);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(
                e.kind,
                SecurityErrorKind::ApprovalRequired { .. }
            ));
        }
    }

    #[test]
    fn test_check_approval_approved() {
        let mut workflow = ApprovalWorkflow::new();
        let params = HashMap::new();

        let action_id = workflow
            .create_pending_action("narrative1", "test.command", params, None)
            .unwrap();

        workflow.approve_action(&action_id, "admin", None).unwrap();

        assert!(workflow.check_approval(&action_id).is_ok());
    }

    #[test]
    fn test_list_pending_actions() {
        let mut workflow = ApprovalWorkflow::new();

        let id1 = workflow
            .create_pending_action("narrative1", "cmd1", HashMap::new(), None)
            .unwrap();
        workflow
            .create_pending_action("narrative1", "cmd2", HashMap::new(), None)
            .unwrap();
        workflow
            .create_pending_action("narrative2", "cmd3", HashMap::new(), None)
            .unwrap();

        // Approve one
        workflow.approve_action(&id1, "admin", None).unwrap();

        let pending = workflow.list_pending_actions("narrative1");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].command, "cmd2");
    }

    #[test]
    fn test_requires_approval() {
        let mut workflow = ApprovalWorkflow::new();

        workflow.set_requires_approval("dangerous.command", true);

        assert!(workflow.requires_approval("dangerous.command"));
        assert!(!workflow.requires_approval("safe.command"));
    }
}
