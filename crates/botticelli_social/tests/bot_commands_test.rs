//! Tests for bot command infrastructure.

mod helpers;

use async_trait::async_trait;
use botticelli_error::{BotCommandError, BotCommandErrorKind};
use botticelli_interface::BotCommandExecutor;
use botticelli_social::BotCommandRegistryImpl;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Mock executor for testing
struct MockBotCommandExecutor {
    platform_name: String,
    responses: HashMap<String, JsonValue>,
}

impl MockBotCommandExecutor {
    fn new(platform: &str) -> Self {
        let mut responses = HashMap::new();

        // Mock server.get_stats response
        responses.insert(
            "server.get_stats".to_string(),
            serde_json::json!({
                "guild_id": "1234567890",
                "name": "Test Server",
                "member_count": 100,
                "channel_count": 10
            }),
        );

        Self {
            platform_name: platform.to_string(),
            responses,
        }
    }
}

#[async_trait]
impl BotCommandExecutor for MockBotCommandExecutor {
    type Error = BotCommandError;

    fn platform(&self) -> &str {
        &self.platform_name
    }

    async fn execute(
        &self,
        command: &str,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        self.responses.get(command).cloned().ok_or_else(|| {
            BotCommandError::new(BotCommandErrorKind::CommandNotFound(command.to_string()))
        })
    }

    fn supports_command(&self, command: &str) -> bool {
        self.responses.contains_key(command)
    }

    fn supported_commands(&self) -> Vec<String> {
        self.responses.keys().cloned().collect()
    }

    fn command_help(&self, _command: &str) -> Option<String> {
        None
    }
}

#[tokio::test]
async fn test_bot_command_execution() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing bot command execution");

    let executor = MockBotCommandExecutor::new("mock");
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!("1234567890"));

    tracing::debug!(platform = "mock", command = "server.get_stats", "Executing command");

    let result = executor.execute("server.get_stats", &args).await?;

    tracing::info!(
        member_count = %result["member_count"],
        channel_count = %result["channel_count"],
        "Command executed successfully"
    );

    assert_eq!(result["member_count"], 100);
    assert_eq!(result["channel_count"], 10);

    Ok(())
}

#[tokio::test]
async fn test_bot_command_registry() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing bot command registry");

    let mut registry = BotCommandRegistryImpl::new();
    registry.register(MockBotCommandExecutor::new("mock"));

    tracing::debug!(platforms = registry.platforms().len(), "Registry created");

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!("1234567890"));

    let result = registry
        .execute("mock", "server.get_stats", &args)
        .await?;

    tracing::info!(?result, "Registry executed command");

    assert_eq!(result["member_count"], 100);

    Ok(())
}

#[tokio::test]
async fn test_unknown_platform() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing unknown platform error");

    let registry = BotCommandRegistryImpl::new();
    let args = HashMap::new();

    tracing::debug!(platform = "unknown", "Attempting to execute on unknown platform");

    let result = registry.execute("unknown", "test", &args).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    tracing::info!(?err, "Got expected error");
    assert!(matches!(
        err.kind(),
        BotCommandErrorKind::PlatformNotFound(_)
    ));

    Ok(())
}

#[tokio::test]
async fn test_unknown_command() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing unknown command error");

    let executor = MockBotCommandExecutor::new("mock");
    let args = HashMap::new();

    tracing::debug!(command = "unknown.command", "Attempting unknown command");

    let result = executor.execute("unknown.command", &args).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    tracing::info!(?err, "Got expected error");
    assert!(matches!(
        err.kind(),
        BotCommandErrorKind::CommandNotFound(_)
    ));

    Ok(())
}

#[tokio::test]
async fn test_registry_platforms() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing registry platforms listing");

    let mut registry = BotCommandRegistryImpl::new();
    registry.register(MockBotCommandExecutor::new("discord"));
    registry.register(MockBotCommandExecutor::new("slack"));

    let platforms = registry.platforms();
    tracing::debug!(?platforms, "Listed platforms");

    assert_eq!(platforms.len(), 2);
    assert!(platforms.contains(&"discord".to_string()));
    assert!(platforms.contains(&"slack".to_string()));

    tracing::info!("Platform listing verified");

    Ok(())
}

#[tokio::test]
async fn test_registry_has_platform() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing registry platform detection");

    let mut registry = BotCommandRegistryImpl::new();
    registry.register(MockBotCommandExecutor::new("discord"));

    tracing::debug!(platform = "discord", has = registry.has_platform("discord"), "Checking platform");
    assert!(registry.has_platform("discord"));

    tracing::debug!(platform = "slack", has = registry.has_platform("slack"), "Checking platform");
    assert!(!registry.has_platform("slack"));

    tracing::info!("Platform detection verified");

    Ok(())
}
