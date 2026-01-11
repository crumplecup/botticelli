//! Secure bot command execution with integrated security framework.
//!
//! This module wraps the bot command registry with the security framework
//! to provide permission checking, input validation, content filtering,
//! rate limiting, and approval workflows.

use crate::BotCommandRegistryImpl;
use async_trait::async_trait;
use botticelli_error::{BotCommandError, BotCommandErrorKind, BotCommandResult};
use botticelli_interface::BotCommandRegistry;
use botticelli_security::{
    ApprovalWorkflow, CommandValidator, ContentFilter, PermissionChecker, RateLimiter,
    SecureExecutor, SecurityError, SecurityErrorKind,
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
#[derive(Getters, derive_setters::Setters)]
#[setters(prefix = "with_")]
pub struct SecureBotCommandExecutor<V: CommandValidator> {
    /// Bot command registry for executing commands.
    #[setters(doc = "Sets the bot command registry")]
    registry: BotCommandRegistryImpl,

    /// Security executor for security pipeline.
    #[setters(doc = "Sets the security executor")]
    security: SecureExecutor<V>,
}

impl<V: CommandValidator> SecureBotCommandExecutor<V> {
    /// Create a new secure bot command executor.
    /// Create a new secure bot command executor.
    #[instrument(skip(registry, permission_checker, validator, content_filter, rate_limiter, approval_workflow))]
    pub fn new(
        registry: BotCommandRegistryImpl,
        permission_checker: PermissionChecker,
        validator: V,
        content_filter: ContentFilter,
        rate_limiter: RateLimiter,
        approval_workflow: ApprovalWorkflow,
    ) -> Self {
        debug!("Creating secure bot command executor");
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

    /// Get immutable access to the approval workflow.
    #[instrument(skip(self))]
    pub fn approval_workflow(&self) -> &ApprovalWorkflow {
        self.security.approval_workflow()
    }

    /// Get immutable access to the rate limiter.
    #[instrument(skip(self))]
    pub fn rate_limiter(&self) -> &RateLimiter {
        self.security.rate_limiter()
    }

    /// Convert JSON arguments to string arguments for security pipeline.
    #[instrument(skip(args), fields(arg_count = args.len()))]
    fn convert_args_to_strings(
        args: &HashMap<String, JsonValue>,
    ) -> BotCommandResult<HashMap<String, String>> {
        debug!("Converting JSON args to string args");
        let mut string_args = HashMap::new();
        for (key, value) in args {
            let string_value = match value {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Null => "null".to_string(),
                other => serde_json::to_string(other)
                    .map_err(BotCommandError::from_serialization_error)?,
            };
            string_args.insert(key.clone(), string_value);
        }
        debug!(string_arg_count = string_args.len(), "Converted args");
        Ok(string_args)
    }

    /// Convert security error to bot command error.
    #[instrument(skip(error), fields(command = command_name, error_kind = ?error.kind))]
    fn convert_security_error(error: SecurityError, command_name: &str) -> BotCommandError {
        debug!("Converting security error to bot command error");
        match error.kind {
            SecurityErrorKind::PermissionDenied { command, reason } => {
                BotCommandError::new(BotCommandErrorKind::PermissionDenied { command, reason })
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
            #[cfg(feature = "database")]
            SecurityErrorKind::Database(msg) => {
                BotCommandError::new(BotCommandErrorKind::ApiError {
                    command: command_name.to_string(),
                    reason: format!("Database error: {}", msg),
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
    type Error = BotCommandError;

    async fn execute(
        &self,
        platform: &str,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        // Note: This implementation bypasses security for backward compatibility
        // Use execute_secure() for secured execution
        self.registry.execute(platform, command, args).await
    }
}
