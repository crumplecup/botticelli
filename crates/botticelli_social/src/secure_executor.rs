//! Secure bot command execution with integrated security framework.
//!
//! This module wraps the bot command registry with the security framework
//! to provide permission checking, input validation, content filtering,
//! rate limiting, and approval workflows.

use crate::{BotCommandError, BotCommandErrorKind, BotCommandRegistryImpl, BotCommandResult};
use async_trait::async_trait;
use botticelli_narrative::BotCommandRegistry;
use botticelli_security::{
    CommandValidator, ContentFilter, PermissionChecker, RateLimiter, SecureExecutor,
    ApprovalWorkflow, SecurityError, SecurityErrorKind,
};
use derive_getters::Getters;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tracing::{debug, error, info, instrument, warn};

/// Secure bot command executor with 5-layer security pipeline.
///
/// Wraps a `BotCommandRegistryImpl` with security checks:
/// 1. Permission verification
/// 2. Input validation
/// 3. Content filtering
/// 4. Rate limiting
/// 5. Approval workflow
#[derive(Getters)]
pub struct SecureBotCommandExecutor<V: CommandValidator> {
    registry: BotCommandRegistryImpl,
    security: SecureExecutor<V>,
}

impl<V: CommandValidator> SecureBotCommandExecutor<V> {
    /// Create a new secure bot command executor.
    pub fn new(
        registry: BotCommandRegistryImpl,
        permission_checker: PermissionChecker,
        validator: V,
        content_filter: ContentFilter,
        rate_limiter: RateLimiter,
        approval_workflow: ApprovalWorkflow,
    ) -> Self {
        Self {
            registry,
            security: SecureExecutor::new(
                permission_checker,
                validator,
                content_filter,
                rate_limiter,
                approval_workflow,
            ),
        }
    }

    /// Execute a bot command through the security pipeline.
    ///
    /// Returns:
    /// - `Ok(ExecutionResult::Success(json))` - Command executed successfully
    /// - `Ok(ExecutionResult::ApprovalRequired(action_id))` - Command requires approval
    /// - `Err(error)` - Security check failed or command execution failed
    #[instrument(
        skip(self, args),
        fields(
            narrative_id,
            platform,
            command,
            arg_count = args.len()
        )
    )]
    pub async fn execute_secure(
        &mut self,
        narrative_id: &str,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<ExecutionResult> {
        info!("Starting secure bot command execution");

        // Convert JSON args to string args for security pipeline
        let string_args = Self::convert_args_to_strings(args)?;

        // Combine platform and command for security checks
        let full_command = format!("{}.{}", platform, command);

        // Run security pipeline
        match self
            .security
            .check_security(narrative_id, &full_command, &string_args)
        {
            Ok(None) => {
                debug!("Security checks passed, executing command");
                // Execute the command
                let result = self.registry.execute(platform, command, args).await?;
                Ok(ExecutionResult::Success(result))
            }
            Ok(Some(action_id)) => {
                warn!(action_id, "Command requires approval");
                Ok(ExecutionResult::ApprovalRequired(action_id))
            }
            Err(security_error) => {
                error!(error = %security_error, "Security check failed");
                Err(Self::convert_security_error(security_error, &full_command))
            }
        }
    }

    /// Get mutable access to the approval workflow for manual approval operations.
    pub fn approval_workflow(&mut self) -> &mut ApprovalWorkflow {
        self.security.approval_workflow()
    }

    /// Get mutable access to the rate limiter for configuration.
    pub fn rate_limiter(&mut self) -> &mut RateLimiter {
        self.security.rate_limiter()
    }

    /// Convert JSON arguments to string arguments for security pipeline.
    fn convert_args_to_strings(
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<HashMap<String, String>> {
        let mut string_args = HashMap::new();
        for (key, value) in args {
            let string_value = match value {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Null => "null".to_string(),
                other => serde_json::to_string(other).map_err(|e| {
                    BotCommandError::new(BotCommandErrorKind::SerializationError {
                        command: "convert_args".to_string(),
                        reason: format!("Failed to serialize argument '{}': {}", key, e),
                    })
                })?,
            };
            string_args.insert(key.clone(), string_value);
        }
        Ok(string_args)
    }

    /// Convert security error to bot command error.
    #[allow(unreachable_patterns)]
    fn convert_security_error(error: SecurityError, command_name: &str) -> BotCommandError {
        match error.kind {
            SecurityErrorKind::PermissionDenied { command, reason } => {
                BotCommandError::new(BotCommandErrorKind::PermissionDenied {
                    command,
                    reason,
                })
            }
            SecurityErrorKind::ResourceAccessDenied { resource, reason } => {
                BotCommandError::new(BotCommandErrorKind::PermissionDenied {
                    command: command_name.to_string(),
                    reason: format!("Resource '{}' access denied: {}", resource, reason),
                })
            }
            SecurityErrorKind::ValidationFailed { field, reason } => {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: command_name.to_string(),
                    arg_name: field,
                    reason,
                })
            }
            SecurityErrorKind::ContentViolation { reason } => {
                BotCommandError::new(BotCommandErrorKind::InvalidArgument {
                    command: command_name.to_string(),
                    arg_name: "content".to_string(),
                    reason,
                })
            }
            SecurityErrorKind::RateLimitExceeded {
                operation,
                window_secs,
                ..
            } => BotCommandError::new(BotCommandErrorKind::RateLimitExceeded {
                command: operation,
                retry_after: window_secs,
            }),
            SecurityErrorKind::ApprovalRequired {
                operation, reason, ..
            } => BotCommandError::new(BotCommandErrorKind::PermissionDenied {
                command: operation,
                reason: format!("Approval required: {}", reason),
            }),
            SecurityErrorKind::ApprovalDenied { action_id, reason } => {
                BotCommandError::new(BotCommandErrorKind::PermissionDenied {
                    command: action_id,
                    reason: format!("Approval denied: {}", reason),
                })
            }
            SecurityErrorKind::Configuration(msg) => {
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: command_name.to_string(),
                    reason: format!("Configuration error: {}", msg),
                })
            }
            // Catch-all for feature-gated variants
            _ => {
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: command_name.to_string(),
                    reason: format!("Security error: {}", error.kind),
                })
            }
        }
    }
}

/// Result of executing a bot command through the security pipeline.
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Command executed successfully with result.
    Success(JsonValue),
    /// Command requires approval with action ID.
    ApprovalRequired(String),
}

// Implement BotCommandRegistry trait for narrative integration
#[async_trait]
impl<V: CommandValidator + Send + Sync> BotCommandRegistry for SecureBotCommandExecutor<V> {
    async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
        // Note: This implementation bypasses security for backward compatibility
        // Use execute_secure() for secured execution
        self.registry
            .execute(platform, command, args)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use botticelli_cache::CommandCache;
    use botticelli_security::{
        ContentFilterConfig, DiscordValidator, PermissionConfig, RateLimit, ResourcePermission,
    };
    use crate::BotCommandExecutor;

    // Mock executor for testing
    struct MockExecutor;

    #[async_trait]
    impl BotCommandExecutor for MockExecutor {
        fn platform(&self) -> &str {
            "mock"
        }

        async fn execute(
            &self,
            command: &str,
            _args: &HashMap<String, JsonValue>,
        ) -> BotCommandResult<JsonValue> {
            match command {
                "messages.send" => Ok(serde_json::json!({"status": "sent"})),
                _ => Err(BotCommandError::new(BotCommandErrorKind::CommandNotFound(
                    command.to_string(),
                ))),
            }
        }

        fn supports_command(&self, command: &str) -> bool {
            command == "messages.send"
        }

        fn supported_commands(&self) -> Vec<String> {
            vec!["messages.send".to_string()]
        }

        fn command_help(&self, _command: &str) -> Option<String> {
            None
        }
    }

    fn create_test_executor() -> SecureBotCommandExecutor<DiscordValidator> {
        let mut registry = BotCommandRegistryImpl::with_cache(CommandCache::default());
        registry.register(MockExecutor);

        let resource_perm = ResourcePermission::new()
            .with_allowed_ids(["123456789012345678".to_string()].into_iter().collect());

        let mut resources = HashMap::new();
        resources.insert("channel".to_string(), resource_perm);

        let perm_config = PermissionConfig::new()
            .with_allowed_commands(["mock.messages.send".to_string()].into_iter().collect())
            .with_resources(resources);

        let permission_checker = PermissionChecker::new(perm_config);
        let validator = DiscordValidator::new();
        let content_filter = ContentFilter::new(ContentFilterConfig::default()).unwrap();
        let mut rate_limiter = RateLimiter::new();
        rate_limiter.add_limit("mock.messages.send", RateLimit::strict(10, 60));
        let approval_workflow = ApprovalWorkflow::new();

        SecureBotCommandExecutor::new(
            registry,
            permission_checker,
            validator,
            content_filter,
            rate_limiter,
            approval_workflow,
        )
    }

    #[tokio::test]
    async fn test_secure_execution_success() {
        let mut executor = create_test_executor();
        let mut args = HashMap::new();
        args.insert(
            "channel_id".to_string(),
            JsonValue::String("123456789012345678".to_string()),
        );
        args.insert(
            "content".to_string(),
            JsonValue::String("Hello, world!".to_string()),
        );

        let result = executor
            .execute_secure("narrative1", "mock", "messages.send", &args)
            .await
            .unwrap();

        match result {
            ExecutionResult::Success(json) => {
                assert_eq!(json["status"], "sent");
            }
            ExecutionResult::ApprovalRequired(_) => panic!("Should not require approval"),
        }
    }

    #[tokio::test]
    async fn test_secure_execution_permission_denied() {
        let mut executor = create_test_executor();
        let args = HashMap::new();

        let result = executor
            .execute_secure("narrative1", "mock", "forbidden.command", &args)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind(),
            BotCommandErrorKind::PermissionDenied { .. }
        ));
    }

    #[tokio::test]
    async fn test_secure_execution_validation_failed() {
        let mut executor = create_test_executor();
        let mut args = HashMap::new();
        // Use a valid channel ID for permissions, but content that's too long
        args.insert(
            "channel_id".to_string(),
            JsonValue::String("123456789012345678".to_string()),
        );
        args.insert(
            "content".to_string(),
            JsonValue::String("x".repeat(2001)), // Exceeds 2000 char limit
        );

        let result = executor
            .execute_secure("narrative1", "mock", "messages.send", &args)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind(),
            BotCommandErrorKind::InvalidArgument { .. }
        ));
    }

    #[tokio::test]
    async fn test_secure_execution_content_violation() {
        let mut executor = create_test_executor();
        let mut args = HashMap::new();
        args.insert(
            "channel_id".to_string(),
            JsonValue::String("123456789012345678".to_string()),
        );
        args.insert(
            "content".to_string(),
            JsonValue::String("@everyone spam".to_string()),
        );

        let result = executor
            .execute_secure("narrative1", "mock", "messages.send", &args)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind(),
            BotCommandErrorKind::InvalidArgument { .. }
        ));
    }

    #[tokio::test]
    async fn test_secure_execution_rate_limit() {
        let mut executor = create_test_executor();
        let mut args = HashMap::new();
        args.insert(
            "channel_id".to_string(),
            JsonValue::String("123456789012345678".to_string()),
        );
        args.insert(
            "content".to_string(),
            JsonValue::String("Hello".to_string()),
        );

        // Exhaust rate limit
        for _ in 0..10 {
            executor
                .execute_secure("narrative1", "mock", "messages.send", &args)
                .await
                .unwrap();
        }

        // 11th should fail
        let result = executor
            .execute_secure("narrative1", "mock", "messages.send", &args)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind(),
            BotCommandErrorKind::RateLimitExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_secure_execution_approval_required() {
        let mut executor = create_test_executor();
        executor
            .approval_workflow()
            .set_requires_approval("mock.messages.send", true);

        let mut args = HashMap::new();
        args.insert(
            "channel_id".to_string(),
            JsonValue::String("123456789012345678".to_string()),
        );
        args.insert(
            "content".to_string(),
            JsonValue::String("Hello".to_string()),
        );

        let result = executor
            .execute_secure("narrative1", "mock", "messages.send", &args)
            .await
            .unwrap();

        match result {
            ExecutionResult::ApprovalRequired(action_id) => {
                assert!(!action_id.is_empty());
            }
            ExecutionResult::Success(_) => panic!("Should require approval"),
        }
    }
}
