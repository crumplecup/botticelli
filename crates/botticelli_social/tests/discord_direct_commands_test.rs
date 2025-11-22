//! Direct Discord command tests without narrative executor.
//!
//! These tests invoke Discord bot commands directly to verify functionality
//! in isolation. They require DISCORD_TOKEN and TEST_GUILD_ID environment
//! variables.
//!
//! Run with: `cargo test --features discord --test discord_direct_commands_test -- --ignored`

use botticelli_social::{BotCommandExecutor, DiscordCommandExecutor};
use std::collections::HashMap;
use std::env;
use serde_json::Value;

/// Helper to create Discord bot executor.
fn create_discord_executor() -> DiscordCommandExecutor {
    dotenvy::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    DiscordCommandExecutor::new(&token)
}

/// Helper to get test guild ID.
fn test_guild_id() -> String {
    env::var("TEST_GUILD_ID").expect("TEST_GUILD_ID not set")
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_server_get_stats() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), Value::String(guild_id.clone()));
    
    let result = executor.execute("server.get_stats", &args).await.expect("server.get_stats failed");
    
    // Verify result contains expected fields
    assert!(result.get("guild_id").is_some());
    assert!(result.get("name").is_some());
    assert!(result.get("member_count").is_some());
    assert_eq!(result["guild_id"].as_str().unwrap(), guild_id);
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_channels_list() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), Value::String(guild_id.clone()));
    
    let result = executor.execute("channels.list", &args).await.expect("channels.list failed");
    
    // Verify result structure
    assert!(result.get("channels").is_some());
    assert!(result["channels"].is_array());
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_channels_get() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    // First, list channels to get a valid channel ID
    let mut list_args = HashMap::new();
    list_args.insert("guild_id".to_string(), Value::String(guild_id.clone()));
    let list_result = executor.execute("channels.list", &list_args).await
        .expect("channels.list failed");
    
    let channels = list_result["channels"].as_array()
        .expect("channels is not an array");
    
    if let Some(first_channel) = channels.first() {
        let channel_id = first_channel["channel_id"].as_str()
            .expect("channel_id not found");
        
        // Now get the specific channel
        let mut get_args = HashMap::new();
        get_args.insert("guild_id".to_string(), Value::String(guild_id));
        get_args.insert("channel_id".to_string(), Value::String(channel_id.to_string()));
        
        let result = executor.execute("channels.get", &get_args).await
            .expect("channels.get failed");
        
        assert!(result.get("channel_id").is_some());
        assert_eq!(result["channel_id"].as_str().unwrap(), channel_id);
    }
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_members_list() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), Value::String(guild_id));
    args.insert("limit".to_string(), Value::Number(10.into()));
    
    let result = executor.execute("members.list", &args).await.expect("members.list failed");
    
    assert!(result.get("members").is_some());
    assert!(result["members"].is_array());
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_roles_list() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), Value::String(guild_id));
    
    let result = executor.execute("roles.list", &args).await.expect("roles.list failed");
    
    assert!(result.get("roles").is_some());
    assert!(result["roles"].is_array());
}

// Argument validation tests

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_server_get_stats_missing_guild_id() {
    let executor = create_discord_executor();
    let args = HashMap::new(); // Missing guild_id
    
    let result = executor.execute("server.get_stats", &args).await;
    assert!(result.is_err(), "Should fail with missing guild_id");
    assert!(result.unwrap_err().to_string().contains("guild_id"));
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_channels_get_missing_channel_id() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), Value::String(guild_id));
    // Missing channel_id
    
    let result = executor.execute("channels.get", &args).await;
    assert!(result.is_err(), "Should fail with missing channel_id");
    assert!(result.unwrap_err().to_string().contains("channel_id"));
}

#[tokio::test]
#[ignore] // Requires Discord API access
async fn test_invalid_command() {
    let executor = create_discord_executor();
    let args = HashMap::new();
    
    let result = executor.execute("invalid.command", &args).await;
    assert!(result.is_err(), "Should fail with invalid command");
}
