//! Integration tests for Discord bot command execution.
//!
//! These tests require:
//! - DISCORD_TOKEN environment variable (bot token)
//! - TEST_GUILD_ID environment variable (Discord server ID where bot is a member)
//!
//! Run with: cargo test -p botticelli --test discord_bot_commands_test --features discord

#![cfg(feature = "discord")]

use botticelli_narrative::{Narrative, NarrativeExecutor};
use botticelli_social::{BotCommandExecutor, BotCommandRegistryImpl, DiscordCommandExecutor};
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
async fn test_discord_command_executor_channels_get() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    // First get list of channels to get a valid channel_id
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let channels = executor
        .execute("channels.list", &args)
        .await
        .expect("Failed to list channels");

    let channels_array = channels.as_array().unwrap();
    assert!(!channels_array.is_empty(), "Should have at least one channel");

    // Get the first channel's ID
    let first_channel = &channels_array[0];
    let channel_id = first_channel["id"].as_str().unwrap();

    // Now test channels.get with that channel_id
    args.insert("channel_id".to_string(), serde_json::json!(channel_id));

    let result = executor
        .execute("channels.get", &args)
        .await
        .expect("Failed to execute channels.get command");

    // Verify result structure
    assert!(result.is_object(), "Result should be a JSON object");
    let obj = result.as_object().unwrap();
    assert!(obj.contains_key("id"), "Result should have id");
    assert!(obj.contains_key("name"), "Result should have name");
    assert!(obj.contains_key("type"), "Result should have type");

    println!("Channel details: {}", serde_json::to_string_pretty(&result).unwrap());
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_members_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));
    args.insert("limit".to_string(), serde_json::json!(10)); // Small limit for testing

    let result = executor
        .execute("members.list", &args)
        .await
        .expect("Failed to execute members.list command");

    // Verify result is an array
    assert!(result.is_array(), "Result should be a JSON array");
    let members = result.as_array().unwrap();

    println!("Found {} members", members.len());
    
    // Verify member structure if any members exist
    if let Some(member) = members.first() {
        assert!(member.is_object(), "Member should be a JSON object");
        let member_obj = member.as_object().unwrap();
        assert!(member_obj.contains_key("user_id"), "Member should have user_id");
        assert!(member_obj.contains_key("username"), "Member should have username");
        assert!(member_obj.contains_key("roles"), "Member should have roles");
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_members_get() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    // First list members to get a valid user_id
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));
    args.insert("limit".to_string(), serde_json::json!(1));

    let members = executor
        .execute("members.list", &args)
        .await
        .expect("Failed to list members");

    let members_array = members.as_array().unwrap();
    if members_array.is_empty() {
        println!("No members in guild, skipping test");
        return;
    }

    // Get the first member's user_id
    let first_member = &members_array[0];
    let user_id = first_member["user_id"].as_str().unwrap();

    // Now test members.get with that user_id
    args.insert("user_id".to_string(), serde_json::json!(user_id));

    let result = executor
        .execute("members.get", &args)
        .await
        .expect("Failed to execute members.get command");

    // Verify result structure
    assert!(result.is_object(), "Result should be a JSON object");
    let obj = result.as_object().unwrap();
    assert!(obj.contains_key("user_id"), "Result should have user_id");
    assert!(obj.contains_key("username"), "Result should have username");
    assert!(obj.contains_key("roles"), "Result should have roles");

    println!("Member details: {}", serde_json::to_string_pretty(&result).unwrap());
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_roles_get() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    // First list roles to get a valid role_id
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let roles = executor
        .execute("roles.list", &args)
        .await
        .expect("Failed to list roles");

    let roles_array = roles.as_array().unwrap();
    assert!(!roles_array.is_empty(), "Should have at least one role");

    // Get the first role's ID
    let first_role = &roles_array[0];
    let role_id = first_role["id"].as_str().unwrap();

    // Now test roles.get with that role_id
    args.insert("role_id".to_string(), serde_json::json!(role_id));

    let result = executor
        .execute("roles.get", &args)
        .await
        .expect("Failed to execute roles.get command");

    // Verify result structure
    assert!(result.is_object(), "Result should be a JSON object");
    let obj = result.as_object().unwrap();
    assert!(obj.contains_key("id"), "Result should have id");
    assert!(obj.contains_key("name"), "Result should have name");
    assert!(obj.contains_key("permissions"), "Result should have permissions");

    println!("Role details: {}", serde_json::to_string_pretty(&result).unwrap());
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_emojis_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("emojis.list", &args)
        .await
        .expect("Failed to execute emojis.list command");

    // Verify result is an array
    assert!(result.is_array(), "Result should be a JSON array");
    let emojis = result.as_array().unwrap();

    println!("Found {} custom emojis", emojis.len());
    
    // Verify emoji structure if any emojis exist
    if let Some(emoji) = emojis.first() {
        assert!(emoji.is_object(), "Emoji should be a JSON object");
        let emoji_obj = emoji.as_object().unwrap();
        assert!(emoji_obj.contains_key("id"), "Emoji should have id");
        assert!(emoji_obj.contains_key("name"), "Emoji should have name");
        assert!(emoji_obj.contains_key("animated"), "Emoji should have animated");
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_events_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("events.list", &args)
        .await
        .expect("Failed to execute events.list command");

    // Verify result is an array
    assert!(result.is_array(), "Result should be a JSON array");
    let events = result.as_array().unwrap();

    println!("Found {} scheduled events", events.len());
    
    // Verify event structure if any events exist
    if let Some(event) = events.first() {
        assert!(event.is_object(), "Event should be a JSON object");
        let event_obj = event.as_object().unwrap();
        assert!(event_obj.contains_key("id"), "Event should have id");
        assert!(event_obj.contains_key("name"), "Event should have name");
        assert!(event_obj.contains_key("start_time"), "Event should have start_time");
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_stickers_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("stickers.list", &args)
        .await
        .expect("Failed to execute stickers.list command");

    assert!(result.is_array(), "Result should be a JSON array");
    let stickers = result.as_array().unwrap();

    println!("Found {} custom stickers", stickers.len());
    
    if let Some(sticker) = stickers.first() {
        assert!(sticker.is_object());
        let sticker_obj = sticker.as_object().unwrap();
        assert!(sticker_obj.contains_key("id"));
        assert!(sticker_obj.contains_key("name"));
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_invites_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor.execute("invites.list", &args).await;
    
    match result {
        Ok(result) => {
            assert!(result.is_array(), "Result should be a JSON array");
            let invites = result.as_array().unwrap();
            println!("Found {} invites", invites.len());
            
            if let Some(invite) = invites.first() {
                assert!(invite.is_object());
                let invite_obj = invite.as_object().unwrap();
                assert!(invite_obj.contains_key("code"));
                assert!(invite_obj.contains_key("uses"));
            }
        }
        Err(e) => {
            // Requires MANAGE_GUILD permission - may not be available
            if e.to_string().contains("Missing Permissions") {
                println!("Skipped: Bot lacks MANAGE_GUILD permission for invites.list");
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_webhooks_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor.execute("webhooks.list", &args).await;
    
    match result {
        Ok(result) => {
            assert!(result.is_array(), "Result should be a JSON array");
            let webhooks = result.as_array().unwrap();
            println!("Found {} webhooks", webhooks.len());
            
            if let Some(webhook) = webhooks.first() {
                assert!(webhook.is_object());
                let webhook_obj = webhook.as_object().unwrap();
                assert!(webhook_obj.contains_key("id"));
                assert!(webhook_obj.contains_key("name"));
            }
        }
        Err(e) => {
            // Requires MANAGE_WEBHOOKS permission - may not be available
            if e.to_string().contains("Missing Permissions") {
                println!("Skipped: Bot lacks MANAGE_WEBHOOKS permission for webhooks.list");
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_bans_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor.execute("bans.list", &args).await;
    
    match result {
        Ok(result) => {
            assert!(result.is_array(), "Result should be a JSON array");
            let bans = result.as_array().unwrap();
            println!("Found {} bans", bans.len());
            
            if let Some(ban) = bans.first() {
                assert!(ban.is_object());
                let ban_obj = ban.as_object().unwrap();
                assert!(ban_obj.contains_key("user_id"));
                assert!(ban_obj.contains_key("username"));
            }
        }
        Err(e) => {
            // Requires BAN_MEMBERS permission - may not be available
            if e.to_string().contains("Missing Permissions") {
                println!("Skipped: Bot lacks BAN_MEMBERS permission for bans.list");
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_discord_command_executor_voice_regions_list() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    let executor = DiscordCommandExecutor::new(&token);

    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), serde_json::json!(guild_id));

    let result = executor
        .execute("voice_regions.list", &args)
        .await
        .expect("Failed to execute voice_regions.list command");

    assert!(result.is_array(), "Result should be a JSON array");
    let regions = result.as_array().unwrap();

    println!("Found {} voice regions", regions.len());
    assert!(!regions.is_empty(), "Should have at least one voice region");
    
    let region = &regions[0];
    assert!(region.is_object());
    let region_obj = region.as_object().unwrap();
    assert!(region_obj.contains_key("id"));
    assert!(region_obj.contains_key("name"));
    assert!(region_obj.contains_key("optimal"));
}

#[tokio::test]
#[ignore]
async fn test_bot_command_registry_with_discord() {
    dotenvy::dotenv().ok();
    
    let token = get_discord_token();
    let guild_id = get_test_guild_id();

    // Create executor and registry
    let executor = DiscordCommandExecutor::new(&token);
    let mut registry = BotCommandRegistryImpl::new();
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
    let mut bot_registry = BotCommandRegistryImpl::new();
    bot_registry.register(executor);

    // Create a simple mock LLM driver for testing
    use botticelli_core::{GenerateRequest, GenerateResponse, Output};
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

        fn rate_limits(&self) -> &botticelli::RateLimitConfig {
            use botticelli::TierConfig;
            static DEFAULT_CONFIG: std::sync::OnceLock<botticelli::RateLimitConfig> = std::sync::OnceLock::new();
            DEFAULT_CONFIG.get_or_init(|| {
                // Create a minimal tier config for testing
                let tier = TierConfig {
                    name: "test".to_string(),
                    rpm: Some(60),
                    tpm: Some(1_000_000),
                    rpd: Some(1500),
                    max_concurrent: Some(5),
                    daily_quota_usd: None,
                    cost_per_million_input_tokens: Some(0.0),
                    cost_per_million_output_tokens: Some(0.0),
                    models: Default::default(),
                };
                botticelli::RateLimitConfig::from_tier(&tier)
            })
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

    // Establish database connection
    use botticelli_database::establish_connection;
    let mut conn = establish_connection().expect("Failed to connect to database");

    // Create narrative executor with bot registry
    let executor = NarrativeExecutor::new(MockDriver)
        .with_bot_registry(Box::new(bot_registry));

    // Create a test TOML narrative file
    let narrative_toml = format!(
        r#"
[metadata]
name = "test_bot_command"
description = "Test Discord bot commands"

[toc]
order = ["get_stats"]

[[act]]
name = "get_stats"

[[act.input]]
type = "bot_command"
platform = "discord"
command = "server.get_stats"
required = true

[act.input.args]
guild_id = "{}"
"#,
        guild_id
    );

    let temp_file = std::env::temp_dir().join("test_bot_command.toml");
    std::fs::write(&temp_file, narrative_toml).expect("Failed to write temp file");

    // Load and execute the narrative
    let narrative = Narrative::from_file_with_db(temp_file.to_str().unwrap(), &mut conn)
        .expect("Failed to load narrative");

    let result = executor
        .execute(&narrative)
        .await
        .expect("Failed to execute narrative with bot commands");

    // Clean up
    std::fs::remove_file(&temp_file).ok();

    // Verify execution
    assert_eq!(result.act_executions.len(), 1);
    assert_eq!(result.act_executions[0].act_name, "get_stats");

    println!("Narrative execution with bot commands successful!");
}
