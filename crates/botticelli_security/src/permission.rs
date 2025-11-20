//! Permission model for command execution.

use crate::{SecurityError, SecurityErrorKind, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, instrument};

/// Permission configuration for a narrative.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionConfig {
    /// Commands explicitly allowed
    #[serde(default)]
    pub allowed_commands: HashSet<String>,

    /// Commands explicitly denied (takes precedence)
    #[serde(default)]
    pub denied_commands: HashSet<String>,

    /// Resource-level permissions
    #[serde(default)]
    pub resources: HashMap<String, ResourcePermission>,

    /// Protected users that cannot be targeted
    #[serde(default)]
    pub protected_users: HashSet<String>,

    /// Protected roles that cannot be modified
    #[serde(default)]
    pub protected_roles: HashSet<String>,

    /// Whether to allow all commands by default
    #[serde(default)]
    pub allow_all_by_default: bool,
}

/// Resource-level permission configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourcePermission {
    /// Specific resource IDs allowed (e.g., channel IDs)
    #[serde(default)]
    pub allowed_ids: HashSet<String>,

    /// Specific resource IDs denied (takes precedence)
    #[serde(default)]
    pub denied_ids: HashSet<String>,

    /// Whether to allow all resources by default
    #[serde(default)]
    pub allow_all_by_default: bool,
}

/// Command permission information.
#[derive(Debug, Clone)]
pub struct CommandPermission {
    /// Command name
    pub command: String,
    /// Whether command is allowed
    pub allowed: bool,
    /// Reason if denied
    pub reason: Option<String>,
}

/// Permission checker for validating command execution.
pub struct PermissionChecker {
    config: PermissionConfig,
}

impl PermissionChecker {
    /// Create a new permission checker with the given configuration.
    pub fn new(config: PermissionConfig) -> Self {
        Self { config }
    }

    /// Check if a command is allowed.
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
        let allowed = self.config.allowed_commands.contains(command)
            || self.config.allow_all_by_default;

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
    #[instrument(skip(self), fields(resource_type, resource_id))]
    pub fn check_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> SecurityResult<()> {
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
        let allowed = resource_perm.allowed_ids.contains(resource_id)
            || resource_perm.allow_all_by_default;

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

    /// Get the permission configuration.
    pub fn config(&self) -> &PermissionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> PermissionConfig {
        let mut config = PermissionConfig::default();
        config.allowed_commands.insert("test.command".to_string());
        config.denied_commands.insert("test.denied".to_string());
        config.protected_users.insert("12345".to_string());
        config
    }

    #[test]
    fn test_allowed_command() {
        let checker = PermissionChecker::new(create_test_config());
        assert!(checker.check_command("test.command").is_ok());
    }

    #[test]
    fn test_denied_command() {
        let checker = PermissionChecker::new(create_test_config());
        let result = checker.check_command("test.denied");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(
                e.kind,
                SecurityErrorKind::PermissionDenied { .. }
            ));
        }
    }

    #[test]
    fn test_unknown_command() {
        let checker = PermissionChecker::new(create_test_config());
        let result = checker.check_command("test.unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_protected_user() {
        let checker = PermissionChecker::new(create_test_config());
        let result = checker.check_user_protected("12345");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(
                e.kind,
                SecurityErrorKind::ResourceAccessDenied { .. }
            ));
        }
    }

    #[test]
    fn test_unprotected_user() {
        let checker = PermissionChecker::new(create_test_config());
        assert!(checker.check_user_protected("67890").is_ok());
    }
}
