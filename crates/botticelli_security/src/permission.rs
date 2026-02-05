//! Permission model for command execution.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use rmcp::tool;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, instrument};

/// Permission configuration for a narrative.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    Default,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_new::new,
    elicitation::Elicit,
)]
#[setters(prefix = "with_")]
pub struct PermissionConfig {
    /// Commands explicitly allowed
    #[serde(default)]
    #[new(default)]
    allowed_commands: HashSet<String>,

    /// Commands explicitly denied (takes precedence)
    #[serde(default)]
    #[new(default)]
    denied_commands: HashSet<String>,

    /// Resource-level permissions
    #[serde(default)]
    #[new(default)]
    resources: HashMap<String, ResourcePermission>,

    /// Protected users that cannot be targeted
    #[serde(default)]
    #[new(default)]
    protected_users: HashSet<String>,

    /// Protected roles that cannot be modified
    #[serde(default)]
    #[new(default)]
    protected_roles: HashSet<String>,

    /// Whether to allow all commands by default
    #[serde(default)]
    #[new(default)]
    allow_all_by_default: bool,
}

/// Resource-level permission configuration.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    Default,
    derive_getters::Getters,
    derive_setters::Setters,
    derive_new::new,
    elicitation::Elicit,
)]
#[setters(prefix = "with_")]
pub struct ResourcePermission {
    /// Specific resource IDs allowed (e.g., channel IDs)
    #[serde(default)]
    #[new(default)]
    allowed_ids: HashSet<String>,

    /// Specific resource IDs denied (takes precedence)
    #[serde(default)]
    #[new(default)]
    denied_ids: HashSet<String>,

    /// Whether to allow all resources by default
    #[serde(default)]
    #[new(default)]
    allow_all_by_default: bool,
}

/// Command permission information.
#[derive(
    Debug,
    Clone,
    schemars::JsonSchema,
    derive_getters::Getters,
    derive_new::new,
    elicitation::Elicit,
)]
pub struct CommandPermission {
    /// Command name
    command: String,
    /// Whether command is allowed
    allowed: bool,
    /// Reason if denied
    reason: Option<String>,
}

/// Permission checker for validating command execution.
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    derive_getters::Getters,
    derive_new::new,
    elicitation::Elicit,
)]
pub struct PermissionChecker {
    config: PermissionConfig,
}

impl PermissionChecker {
    /// Check if a command is allowed.
    #[tool]
    #[instrument(skip(self), fields(command))]
    pub fn check_command(&self, command: &str) -> SecurityResult<()> {
        debug!("Checking command permission");

        // Deny list takes precedence
        if self.config.denied_commands.contains(command) {
            debug!("Command explicitly denied");
            return Err(SecurityError::new(SecurityErrorKind::PermissionDenied {
                command: command.to_string(),
                reason: "Command is in deny list".to_string(),
            }));
        }

        // Check allow list or default policy
        let allowed =
            self.config.allowed_commands.contains(command) || self.config.allow_all_by_default;

        if !allowed {
            debug!("Command not in allow list");
            return Err(SecurityError::new(SecurityErrorKind::PermissionDenied {
                command: command.to_string(),
                reason: "Command not in allow list".to_string(),
            }));
        }

        debug!("Command permitted");
        Ok(())
    }

    /// Check if a resource is accessible.
    #[tool]
    #[instrument(skip(self), fields(resource_type, resource_id))]
    pub fn check_resource(&self, resource_type: &str, resource_id: &str) -> SecurityResult<()> {
        debug!("Checking resource permission");

        let resource_perm = self
            .config
            .resources
            .get(resource_type)
            .cloned()
            .unwrap_or_default();

        // Deny list takes precedence
        if resource_perm.denied_ids.contains(resource_id) {
            debug!("Resource explicitly denied");
            return Err(SecurityError::new(
                SecurityErrorKind::ResourceAccessDenied {
                    resource: format!("{}:{}", resource_type, resource_id),
                    reason: "Resource is in deny list".to_string(),
                },
            ));
        }

        // Check allow list or default policy
        let allowed =
            resource_perm.allowed_ids.contains(resource_id) || resource_perm.allow_all_by_default;

        if !allowed {
            debug!("Resource not in allow list");
            return Err(SecurityError::new(
                SecurityErrorKind::ResourceAccessDenied {
                    resource: format!("{}:{}", resource_type, resource_id),
                    reason: "Resource not in allow list".to_string(),
                },
            ));
        }

        debug!("Resource access permitted");
        Ok(())
    }

    /// Check if a user is protected.
    #[tool]
    #[instrument(skip(self), fields(user_id))]
    pub fn check_user_protected(&self, user_id: &str) -> SecurityResult<()> {
        if self.config.protected_users.contains(user_id) {
            debug!("User is protected");
            return Err(SecurityError::new(
                SecurityErrorKind::ResourceAccessDenied {
                    resource: format!("user:{}", user_id),
                    reason: "User is protected and cannot be targeted".to_string(),
                },
            ));
        }
        debug!("User is not protected");
        Ok(())
    }

    /// Check if a role is protected.
    #[tool]
    #[instrument(skip(self), fields(role_id))]
    pub fn check_role_protected(&self, role_id: &str) -> SecurityResult<()> {
        if self.config.protected_roles.contains(role_id) {
            debug!("Role is protected");
            return Err(SecurityError::new(
                SecurityErrorKind::ResourceAccessDenied {
                    resource: format!("role:{}", role_id),
                    reason: "Role is protected and cannot be modified".to_string(),
                },
            ));
        }
        debug!("Role is not protected");
        Ok(())
    }
}
