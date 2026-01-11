//! Tests for bot command infrastructure.

use async_trait::async_trait;
use botticelli_interface::BotCommandExecutor;
use botticelli_social::{
    BotCommandError, BotCommandErrorKind, BotCommandRegistryImpl,
};
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

    async fn messages_bulk_delete(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"deleted": 5}))
    }

    async fn threads_create(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"thread_id": "123456"}))
    }

    async fn threads_list(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"threads": []}))
    }

    async fn threads_get(&self, _args: &HashMap<String, JsonValue>) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"thread_id": "123456"}))
    }

    async fn threads_edit(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn threads_delete(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn threads_join(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn threads_leave(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn threads_add_member(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn threads_remove_member(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn reactions_list(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"reactions": []}))
    }

    async fn reactions_clear(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }

    async fn reactions_clear_emoji(
        &self,
        _args: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, Self::Error> {
        Ok(serde_json::json!({"success": true}))
    }
}

#[tokio::test]
async fn test_bot_command_execution() {
    let executor = MockBotCommandExecutor::new("mock");
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!("1234567890"));

    let result = executor.execute("server.get_stats", &args).await.unwrap();

    assert_eq!(result["member_count"], 100);
    assert_eq!(result["channel_count"], 10);
}

#[tokio::test]
async fn test_bot_command_registry() {
    let mut registry = BotCommandRegistryImpl::new();
    registry.register(MockBotCommandExecutor::new("mock"));

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!("1234567890"));

    let result = registry
        .execute("mock", "server.get_stats", &args)
        .await
        .unwrap();

    assert_eq!(result["member_count"], 100);
}

#[tokio::test]
async fn test_unknown_platform() {
    let registry = BotCommandRegistryImpl::new();
    let args = HashMap::new();

    let result = registry.execute("unknown", "test", &args).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        BotCommandErrorKind::PlatformNotFound(_)
    ));
}

#[tokio::test]
async fn test_unknown_command() {
    let executor = MockBotCommandExecutor::new("mock");
    let args = HashMap::new();

    let result = executor.execute("unknown.command", &args).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        BotCommandErrorKind::CommandNotFound(_)
    ));
}

#[tokio::test]
async fn test_registry_platforms() {
    let mut registry = BotCommandRegistryImpl::new();
    registry.register(MockBotCommandExecutor::new("discord"));
    registry.register(MockBotCommandExecutor::new("slack"));

    let platforms = registry.platforms();
    assert_eq!(platforms.len(), 2);
    assert!(platforms.contains(&"discord".to_string()));
    assert!(platforms.contains(&"slack".to_string()));
}

#[tokio::test]
async fn test_registry_has_platform() {
    let mut registry = BotCommandRegistryImpl::new();
    registry.register(MockBotCommandExecutor::new("discord"));

    assert!(registry.has_platform("discord"));
    assert!(!registry.has_platform("slack"));
}
