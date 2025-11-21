//! Security-aware bot command executor.
//!
//! This module integrates the security framework with bot command execution,
//! providing a secure wrapper around platform-specific executors.

use crate::{BotCommandError, BotCommandErrorKind, BotCommandExecutor, BotCommandResult};
use async_trait::async_trait;
use botticelli_security::{
    ApprovalWorkflow, CommandValidator, ContentFilter, PermissionChecker, RateLimiter,
    SecureExecutor, SecurityError,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument};

/// Security-aware bot command executor.
///
/// Wraps a platform-specific executor with the security framework's 5-layer pipeline:
/// 1. Permission checking
/// 2. Input validation
/// 3. Content filtering
/// 4. Rate limiting
/// 5. Approval workflow
pub struct SecureBotExecutor<E, V>
where
    E: BotCommandExecutor,
    V: CommandValidator,
{
    inner: E,
    secure_executor: Arc<Mutex<SecureExecutor<V>>>,
    narrative_id: String,
}

impl<E, V> SecureBotExecutor<E, V>
where
    E: BotCommandExecutor,
    V: CommandValidator,
{
    /// Create a new secure bot executor.
    pub fn new(
        inner: E,
        permission_checker: PermissionChecker,
        validator: V,
        content_filter: ContentFilter,
        rate_limiter: RateLimiter,
        approval_workflow: ApprovalWorkflow,
        narrative_id: String,
    ) -> Self {
        let secure_executor = SecureExecutor::new(
            permission_checker,
            validator,
            content_filter,
            rate_limiter,
            approval_workflow,
        );

        Self {
            inner,
            secure_executor: Arc::new(Mutex::new(secure_executor)),
            narrative_id,
        }
    }

    /// Get reference to inner executor.
    pub fn inner(&self) -> &E {
        &self.inner
    }

    /// Get mutable reference to inner executor.
    pub fn inner_mut(&mut self) -> &mut E {
        &mut self.inner
    }
}

#[async_trait]
impl<E, V> BotCommandExecutor for SecureBotExecutor<E, V>
where
    E: BotCommandExecutor + Send + Sync,
    V: CommandValidator + Send + Sync,
{
    #[instrument(skip(self, args), fields(platform = self.inner.platform(), command, narrative_id = %self.narrative_id))]
    async fn execute(&self, command: &str, args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
        info!("Executing command through security pipeline");

        // Convert HashMap args to String params for security checks
        let params = hashmap_to_params(args)?;

        // Run security checks
        debug!("Running security checks");
        let mut secure_executor = self.secure_executor.lock().await;
        let approval_id = secure_executor
            .check_security(&self.narrative_id, command, &params)
            .map_err(|e| {
                error!("Security check failed: {}", e);
                security_error_to_bot_error(command, e)
            })?;

        // If approval required, return pending status
        if let Some(approval_id) = approval_id {
            info!(approval_id = %approval_id, "Command requires approval");
            return Ok(serde_json::json!({
                "status": "pending_approval",
                "approval_id": approval_id,
                "message": "Command requires approval before execution"
            }));
        }

        // Security checks passed, execute the command
        debug!("Security checks passed, executing command");
        drop(secure_executor); // Release lock before executing

        let result = self.inner.execute(command, args).await?;

        info!("Command executed successfully");
        Ok(result)
    }

    fn platform(&self) -> &str {
        self.inner.platform()
    }

    fn supported_commands(&self) -> Vec<String> {
        self.inner.supported_commands()
    }

    fn supports_command(&self, command: &str) -> bool {
        self.inner.supports_command(command)
    }

    fn command_help(&self, command: &str) -> Option<String> {
        self.inner.command_help(command)
    }
}

/// Convert HashMap<String, JsonValue> to HashMap<String, String> for security checks.
fn hashmap_to_params(args: &HashMap<String, JsonValue>) -> BotCommandResult<HashMap<String, String>> {
    let mut params = HashMap::new();

    for (key, value) in args {
        let value_str = match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => continue,
            _ => serde_json::to_string(value).map_err(|e| {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: "security_check".to_string(),
                    arg_name: key.clone(),
                    reason: format!("Failed to serialize argument: {}", e),
                })
            })?,
        };
        params.insert(key.clone(), value_str);
    }

    Ok(params)
}

/// Convert security error to bot command error.
fn security_error_to_bot_error(command: &str, error: SecurityError) -> BotCommandError {
    let kind = match error.kind() {
        botticelli_security::SecurityErrorKind::PermissionDenied { command: _, reason } => {
            BotCommandErrorKind::PermissionDenied {
                command: command.to_string(),
                reason: reason.clone(),
            }
        }
        botticelli_security::SecurityErrorKind::ResourceAccessDenied { resource, reason } => {
            BotCommandErrorKind::PermissionDenied {
                command: command.to_string(),
                reason: format!("Resource '{}': {}", resource, reason),
            }
        }
        botticelli_security::SecurityErrorKind::ValidationFailed { field, reason } => {
            BotCommandErrorKind::InvalidArgument {
                command: command.to_string(),
                arg_name: field.clone(),
                reason: reason.clone(),
            }
        }
        botticelli_security::SecurityErrorKind::ContentViolation { reason } => {
            BotCommandErrorKind::ContentFiltered {
                command: command.to_string(),
                reason: reason.clone(),
            }
        }
        botticelli_security::SecurityErrorKind::RateLimitExceeded {
            operation: _,
            reason: _,
            limit: _,
            window_secs,
        } => BotCommandErrorKind::RateLimitExceeded {
            command: command.to_string(),
            retry_after: *window_secs,
        },
        _ => BotCommandErrorKind::ApiError {
            command: command.to_string(),
            reason: error.to_string(),
        },
    };

    BotCommandError::new(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use botticelli_security::{DiscordValidator, PermissionConfig};

    /// Mock executor for testing.
    struct MockExecutor {
        platform: String,
        commands: Vec<String>,
    }

    #[async_trait]
    impl BotCommandExecutor for MockExecutor {
        async fn execute(&self, _command: &str, _args: &HashMap<String, JsonValue>) -> BotCommandResult<JsonValue> {
            Ok(serde_json::json!({"status": "success"}))
        }

        fn platform(&self) -> &str {
            &self.platform
        }

        fn supported_commands(&self) -> Vec<String> {
            self.commands.clone()
        }

        fn supports_command(&self, command: &str) -> bool {
            self.commands.contains(&command.to_string())
        }

        fn command_help(&self, _command: &str) -> Option<String> {
            None
        }
    }

    #[tokio::test]
    async fn test_secure_executor_wraps_inner() {
        let mock = MockExecutor {
            platform: "test".to_string(),
            commands: vec!["test.command".to_string()],
        };

        let permission_config = PermissionConfig::new()
            .with_allowed_commands(vec!["test.command".to_string()].into_iter().collect());

        let content_filter = ContentFilter::new(Default::default()).unwrap();

        let secure = SecureBotExecutor::new(
            mock,
            PermissionChecker::new(permission_config),
            DiscordValidator,
            content_filter,
            RateLimiter::new(),
            ApprovalWorkflow::new(),
            "test_narrative".to_string(),
        );

        assert_eq!(secure.platform(), "test");
        assert_eq!(secure.supported_commands(), vec!["test.command"]);
    }

    #[tokio::test]
    async fn test_security_check_blocks_disallowed_command() {
        let mock = MockExecutor {
            platform: "test".to_string(),
            commands: vec!["test.command".to_string()],
        };

        let permission_config = PermissionConfig::new()
            .with_allowed_commands(vec!["allowed.command".to_string()].into_iter().collect());

        let content_filter = ContentFilter::new(Default::default()).unwrap();

        let secure = SecureBotExecutor::new(
            mock,
            PermissionChecker::new(permission_config),
            DiscordValidator,
            content_filter,
            RateLimiter::new(),
            ApprovalWorkflow::new(),
            "test_narrative".to_string(),
        );

        let args = HashMap::new();
        let result = secure
            .execute("test.command", &args)
            .await;

        assert!(result.is_err());
        match result.unwrap_err().kind() {
            BotCommandErrorKind::PermissionDenied { .. } => {}
            other => panic!("Expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_security_check_allows_allowed_command() {
        let mock = MockExecutor {
            platform: "test".to_string(),
            commands: vec!["test.command".to_string()],
        };

        let permission_config = PermissionConfig::new()
            .with_allowed_commands(vec!["test.command".to_string()].into_iter().collect());

        let content_filter = ContentFilter::new(Default::default()).unwrap();

        let secure = SecureBotExecutor::new(
            mock,
            PermissionChecker::new(permission_config),
            DiscordValidator,
            content_filter,
            RateLimiter::new(),
            ApprovalWorkflow::new(),
            "test_narrative".to_string(),
        );

        let args = HashMap::new();
        let result = secure
            .execute("test.command", &args)
            .await;

        assert!(result.is_ok());
    }
}
