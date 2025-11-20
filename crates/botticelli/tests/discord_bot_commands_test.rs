//! Integration tests for Discord bot command execution.
//!
//! These tests require:
//! - DISCORD_TOKEN environment variable (bot token)
//! - TEST_GUILD_ID environment variable (Discord server ID where bot is a member)
//!
//! Run with: cargo test -p botticelli --test discord_bot_commands_test --features discord

use botticelli_narrative::NarrativeExecutor;
use botticelli_social::{BotCommandExecutor, BotCommandRegistry, DiscordCommandExecutor};
use std::collections::HashMap;

/// Helper to get test guild ID from environment
fn get_test_guild_id() -> String {
    std::env::var("TEST_GUILD_ID").unwrap_or_else(|_| {
        // Try to read from .env file
        dotenvy::dotenv().ok();
        std::env::var("TEST_GUILD_ID")
            .expect("TEST_GUILD_ID environment variable must be set for Discord integration tests")
    })
}

/// Helper to get Discord token from environment
fn get_discord_token() -> String {
    std::env::var("DISCORD_TOKEN").unwrap_or_else(|_| {
        // Try to read from .env file
        dotenvy::dotenv().ok();
        std::env::var("DISCORD_TOKEN")
            .expect("DISCORD_TOKEN environment variable must be set for Discord integration tests")
    })
}

#[tokio::test]
#[ignore] // Only run with --ignored flag to avoid hitting Discord API in normal test runs
async fn test_discord_command_executor_server_stats() {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    // Create Discord command executor
    let executor = DiscordCommandExecutor::new(&token);

    // Execute server.get_stats command
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("server.get_stats", &args)
        .await
        .expect("Failed to execute server.get_stats command");

    // Verify result structure
    assert!(result.is_object(), "Result should be a JSON object");
    let obj = result.as_object().unwrap();

    // Check required fields
    assert!(obj.contains_key("guild_id"), "Result should have guild_id");
    assert!(obj.contains_key("name"), "Result should have name");
    assert!(
        obj.contains_key("member_count"),
        "Result should have member_count"
    );

    println!("Guild name: {}", obj["name"]);
    println!("Member count: {}", obj["member_count"]);
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_channels_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("channels.list", &args)
        .await
        .expect("Failed to execute channels.list command");

    // Verify result is an array
    assert!(result.is_array(), "Result should be a JSON array");
    let channels = result.as_array().unwrap();

    println!("Found {} channels", channels.len());
    
    // Verify channel structure if any channels exist
    if let Some(channel) = channels.first() {
        assert!(channel.is_object(), "Channel should be a JSON object");
        let chan_obj = channel.as_object().unwrap();
        assert!(chan_obj.contains_key("id"), "Channel should have id");
        assert!(chan_obj.contains_key("name"), "Channel should have name");
        assert!(chan_obj.contains_key("type"), "Channel should have type");
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_roles_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("roles.list", &args)
        .await
        .expect("Failed to execute roles.list command");

    // Verify result is an array
    assert!(result.is_array(), "Result should be a JSON array");
    let roles = result.as_array().unwrap();

    println!("Found {} roles", roles.len());
    
    // Every Discord server has at least @everyone role
    assert!(roles.len() >= 1, "Should have at least one role (@everyone)");
    
    // Verify role structure
    if let Some(role) = roles.first() {
        assert!(role.is_object(), "Role should be a JSON object");
        let role_obj = role.as_object().unwrap();
        assert!(role_obj.contains_key("id"), "Role should have id");
        assert!(role_obj.contains_key("name"), "Role should have name");
        assert!(role_obj.contains_key("color"), "Role should have color");
        assert!(role_obj.contains_key("permissions"), "Role should have permissions");
    }
}

#[tokio::test]
#[ignore]
async fn test_bot_command_registry_with_discord() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    // Create executor and registry
    let executor = DiscordCommandExecutor::new(&token);
    let mut registry = BotCommandRegistry::new();
    registry.register(executor);

    // Execute via registry
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = registry
        .execute("discord", "server.get_stats", &args)
        .await
        .expect("Failed to execute command via registry");

    assert!(result.is_object(), "Result should be a JSON object");
    println!("Registry execution successful");
}

#[tokio::test]
#[ignore]
async fn test_narrative_with_bot_commands() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    // Create Discord executor and registry
    let executor = DiscordCommandExecutor::new(&token);
    let mut bot_registry = BotCommandRegistry::new();
    bot_registry.register(executor);

    // Create a simple mock LLM driver for testing
    use botticelli_core::{GenerateRequest, GenerateResponse, Input, Output};
    use botticelli_interface::BotticelliDriver;
    use async_trait::async_trait;

    struct MockDriver;

    #[async_trait]
    impl BotticelliDriver for MockDriver {
        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }

        async fn generate(
            &self,
            _request: &GenerateRequest,
        ) -> botticelli_error::BotticelliResult<GenerateResponse> {
            // Just return a mock response
            Ok(GenerateResponse {
                outputs: vec![Output::Text("Mock LLM response".to_string())],
            })
        }
    }

    // Create narrative executor with bot registry
    let executor = NarrativeExecutor::new(MockDriver)
        .with_bot_registry(Box::new(bot_registry));

    // Create a simple in-memory narrative with bot command
    use botticelli_core::Input as CoreInput;
    use botticelli_narrative::{ActConfig, Narrative, NarrativeMetadata, NarrativeToc};
    use std::collections::HashMap as StdHashMap;

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let mut acts = StdHashMap::new();
    acts.insert(
        "fetch_stats".to_string(),
        ActConfig {
            inputs: vec![CoreInput::BotCommand {
                platform: "discord".to_string(),
                command: "server.get_stats".to_string(),
                args,
                required: true,
                cache_duration: None,
            }],
            model: None,
            temperature: None,
            max_tokens: None,
        },
    );

    let narrative = Narrative {
        metadata: NarrativeMetadata {
            name: "test_narrative".to_string(),
            description: "Test narrative with bot command".to_string(),
            template: None,
            skip_content_generation: false,
        },
        toc: NarrativeToc {
            order: vec!["fetch_stats".to_string()],
        },
        acts,
    };

    // Execute the narrative
    let result = executor
        .execute(&narrative)
        .await
        .expect("Failed to execute narrative with bot commands");

    // Verify execution
    assert_eq!(result.act_executions.len(), 1);
    assert_eq!(result.act_executions[0].act_name, "fetch_stats");
    
    // The input should have been processed (bot command executed and converted to text)
    assert_eq!(result.act_executions[0].inputs.len(), 1);
    match &result.act_executions[0].inputs[0] {
        CoreInput::Text(text) => {
            // Should contain JSON from Discord API
            assert!(text.contains("guild_id") || text.contains("name"), 
                    "Processed input should contain Discord API response");
            println!("Bot command was executed and converted to text:");
            println!("{}", text);
        }
        _ => panic!("Expected bot command to be converted to text input"),
    }

    println!("Narrative execution with bot commands successful!");
}
