//! Tests for secure bot command executor.

mod helpers;

use async_trait::async_trait;
use botticelli_cache::CommandCache;
use botticelli_error::{BotCommandError, BotCommandErrorKind};
use botticelli_interface::BotCommandExecutor;
use botticelli_security::{
    ApprovalWorkflow, ContentFilter, ContentFilterConfig, DiscordValidator, PermissionChecker,
    PermissionConfig, RateLimit, RateLimiter, ResourcePermission,
};
use botticelli_social::{BotCommandRegistryImpl, ExecutionResult, SecureBotCommandExecutor};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

// Mock executor for testing
struct MockExecutor;

#[async_trait]
impl BotCommandExecutor for MockExecutor {
    type Error = BotCommandError;

    fn platform(&self) -> &str {
        "mock"
    }

    async fn execute(
        &self,
        command: &str,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
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

fn create_test_executor() -> anyhow::Result<SecureBotCommandExecutor<DiscordValidator>> {
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
    let content_filter = ContentFilter::new(ContentFilterConfig::default())?;
    let mut rate_limiter = RateLimiter::new();
    rate_limiter.add_limit("mock.messages.send", RateLimit::strict(10, 60));
    let approval_workflow = ApprovalWorkflow::new();

    Ok(SecureBotCommandExecutor::new(
        registry,
        permission_checker,
        validator,
        content_filter,
        rate_limiter,
        approval_workflow,
    ))
}

#[tokio::test]
async fn test_secure_execution_success() -> anyhow::Result<()> {
    helpers::init_test_tracing("debug");
    tracing::info!("Testing secure execution success");

    let mut executor = create_test_executor()?;
    let mut args = HashMap::new();
    args.insert(
        "channel_id".to_string(),
        JsonValue::String("123456789012345678".to_string()),
    );
    args.insert(
        "content".to_string(),
        JsonValue::String("Hello, world!".to_string()),
    );

    tracing::debug!(
        command = "messages.send",
        channel_id = "123456789012345678",
        "Executing secure command"
    );

    let result = executor
        .execute_secure("narrative1", "mock", "messages.send", &args)
        .await?;

    match result {
        ExecutionResult::Success(json) => {
            tracing::info!(?json, "Command succeeded");
            assert_eq!(json["status"], "sent");
        }
        ExecutionResult::ApprovalRequired(_) => panic!("Should not require approval"),
    }

    Ok(())
}

#[tokio::test]
async fn test_secure_execution_permission_denied() -> anyhow::Result<()> {
    helpers::init_test_tracing("debug");
    tracing::info!("Testing permission denied");

    let mut executor = create_test_executor()?;
    let args = HashMap::new();

    tracing::debug!(command = "forbidden.command", "Executing forbidden command");

    let result = executor
        .execute_secure("narrative1", "mock", "forbidden.command", &args)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    tracing::info!(?err, "Command denied as expected");
    assert!(matches!(
        err.kind(),
        BotCommandErrorKind::PermissionDenied { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_secure_execution_validation_failed() -> anyhow::Result<()> {
    helpers::init_test_tracing("debug");
    tracing::info!("Testing validation failure");

    let mut executor = create_test_executor()?;
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

    tracing::debug!(
        content_length = 2001,
        "Executing with invalid content length"
    );

    let result = executor
        .execute_secure("narrative1", "mock", "messages.send", &args)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    tracing::info!(?err, "Validation failed as expected");
    assert!(matches!(
        err.kind(),
        BotCommandErrorKind::InvalidArgument { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_secure_execution_content_violation() -> anyhow::Result<()> {
    helpers::init_test_tracing("debug");
    tracing::info!("Testing content violation");

    let mut executor = create_test_executor()?;
    let mut args = HashMap::new();
    args.insert(
        "channel_id".to_string(),
        JsonValue::String("123456789012345678".to_string()),
    );
    args.insert(
        "content".to_string(),
        JsonValue::String("@everyone spam".to_string()),
    );

    tracing::debug!(
        content = "@everyone spam",
        "Executing with forbidden content"
    );

    let result = executor
        .execute_secure("narrative1", "mock", "messages.send", &args)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    tracing::info!(?err, "Content violation detected as expected");
    assert!(matches!(
        err.kind(),
        BotCommandErrorKind::InvalidArgument { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_secure_execution_rate_limit() -> anyhow::Result<()> {
    helpers::init_test_tracing("debug");
    tracing::info!("Testing rate limit");

    let mut executor = create_test_executor()?;
    let mut args = HashMap::new();
    args.insert(
        "channel_id".to_string(),
        JsonValue::String("123456789012345678".to_string()),
    );
    args.insert(
        "content".to_string(),
        JsonValue::String("Hello".to_string()),
    );

    tracing::debug!("Exhausting rate limit (10 requests)");
    // Exhaust rate limit
    for i in 0..10 {
        tracing::trace!(request_num = i + 1, "Executing request");
        executor
            .execute_secure("narrative1", "mock", "messages.send", &args)
            .await?;
    }

    tracing::debug!("Attempting 11th request (should fail)");
    // 11th should fail
    let result = executor
        .execute_secure("narrative1", "mock", "messages.send", &args)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    tracing::info!(?err, "Rate limit triggered as expected");
    assert!(matches!(
        err.kind(),
        BotCommandErrorKind::RateLimitExceeded { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_secure_execution_approval_required() -> anyhow::Result<()> {
    helpers::init_test_tracing("debug");
    tracing::info!("Testing approval workflow");

    // Create executor with approval required for messages.send
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
    let content_filter = ContentFilter::new(ContentFilterConfig::default())?;
    let mut rate_limiter = RateLimiter::new();
    rate_limiter.add_limit("mock.messages.send", RateLimit::strict(10, 60));

    // Configure approval workflow to require approval for messages.send
    let mut approval_workflow = ApprovalWorkflow::new();
    approval_workflow.set_requires_approval("mock.messages.send", true);
    tracing::debug!("Configured approval workflow for messages.send");

    let mut executor = SecureBotCommandExecutor::new(
        registry,
        permission_checker,
        validator,
        content_filter,
        rate_limiter,
        approval_workflow,
    );

    let mut args = HashMap::new();
    args.insert(
        "channel_id".to_string(),
        JsonValue::String("123456789012345678".to_string()),
    );
    args.insert(
        "content".to_string(),
        JsonValue::String("Hello".to_string()),
    );

    tracing::debug!("Executing command requiring approval");
    let result = executor
        .execute_secure("narrative1", "mock", "messages.send", &args)
        .await?;

    match result {
        ExecutionResult::ApprovalRequired(action_id) => {
            tracing::info!(action_id = %action_id, "Approval required as expected");
            assert!(!action_id.is_empty());
        }
        ExecutionResult::Success(_) => panic!("Should require approval"),
    }

    Ok(())
}
