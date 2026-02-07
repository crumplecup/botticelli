//! Security-aware bot command executor.
//!
//! This module integrates the security framework with bot command execution,
//! providing a secure wrapper around platform-specific executors.

use async_trait::async_trait;
use botticelli_error::{BotCommandError, BotCommandResult};
use botticelli_interface::BotCommandExecutor;
use botticelli_security::{
    ApprovalWorkflow, CommandValidator, ContentFilter, PermissionChecker, RateLimiter,
    SecureExecutor,
};
use rmcp::tool;
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
    #[tool]
    #[instrument(
        skip(
            inner,
            permission_checker,
            validator,
            content_filter,
            rate_limiter,
            approval_workflow
        ),
        fields(narrative_id)
    )]
    pub fn new(
        inner: E,
        permission_checker: PermissionChecker,
        validator: V,
        content_filter: ContentFilter,
        rate_limiter: RateLimiter,
        approval_workflow: ApprovalWorkflow,
        narrative_id: String,
    ) -> Self {
        debug!("Creating secure bot executor");
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
    #[tool]
    #[instrument(skip(self))]
    pub fn inner(&self) -> &E {
        &self.inner
    }

    /// Get mutable reference to inner executor.
    #[tool]
    #[instrument(skip(self))]
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
    type Error = BotCommandError;

    #[instrument(skip(self, args), fields(platform = self.inner.platform(), command, narrative_id = %self.narrative_id))]
    async fn execute(
        &self,
        command: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
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
                BotCommandError::from_security_error(e)
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

        // Wrapper converts inner error to BotCommandError (legitimate conversion)
        let result = self
            .inner
            .execute(command, args)
            .await
            .map_err(|e| BotCommandError::from_api_error(command, e))?;

        info!("Command executed successfully");
        Ok(result)
    }

    #[instrument(skip(self))]
    fn platform(&self) -> &str {
        let platform = self.inner.platform();
        debug!(platform, "Getting platform");
        platform
    }

    #[instrument(skip(self))]
    fn supported_commands(&self) -> Vec<String> {
        let commands = self.inner.supported_commands();
        debug!(count = commands.len(), "Getting supported commands");
        commands
    }

    #[instrument(skip(self), fields(command))]
    fn supports_command(&self, command: &str) -> bool {
        let supported = self.inner.supports_command(command);
        debug!(command, supported, "Checking command support");
        supported
    }

    #[instrument(skip(self), fields(command))]
    fn command_help(&self, command: &str) -> Option<String> {
        let help = self.inner.command_help(command);
        debug!(command, has_help = help.is_some(), "Getting command help");
        help
    }
}

/// Convert HashMap<String, JsonValue> to HashMap<String, String> for security checks.
#[tool]
#[instrument(skip(args), fields(arg_count = args.len()))]
pub fn hashmap_to_params(
    args: &HashMap<String, JsonValue>,
) -> BotCommandResult<HashMap<String, String>> {
    debug!("Converting args to params");
    let mut params = HashMap::new();

    for (key, value) in args {
        let value_str = match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => continue,
            _ => serde_json::to_string(value).map_err(BotCommandError::from_serialization_error)?,
        };
        params.insert(key.clone(), value_str);
    }

    debug!(param_count = params.len(), "Converted args to params");
    Ok(params)
}
