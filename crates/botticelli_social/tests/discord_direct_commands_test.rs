//! Direct Discord command tests without narrative executor.
//!
//! These tests invoke Discord bot commands directly to verify functionality
//! in isolation. They require DISCORD_TOKEN and TEST_GUILD_ID environment
//! variables.
//!
//! Run with: `cargo test --features discord --test discord_direct_commands_test -- --ignored`

use botticelli_social::BotCommandExecutor;
use std::collections::HashMap;
use std::env;

/// Helper to create Discord bot executor.
fn create_discord_executor() -> impl BotCommandExecutor {
    dotenvy::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    botticelli_social::discord::DiscordBotExecutor::new(&token)
        .expect("Failed to create Discord bot executor")
}

/// Helper to get test guild ID.
fn test_guild_id() -> String {
    env::var("TEST_GUILD_ID").expect("TEST_GUILD_ID not set")
}

#[test]
#[ignore] // Requires Discord API access
fn test_guilds_get() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), guild_id.clone());
    
    let result = executor.execute("guilds.get", args).expect("guilds.get failed");
    
    // Verify result contains expected fields
    assert!(result.contains("guild_id"));
    assert!(result.contains("name"));
    
    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("Result is not valid JSON");
    assert_eq!(parsed["guild_id"].as_str().unwrap(), guild_id);
}

#[test]
#[ignore] // Requires Discord API access
fn test_channels_list() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), guild_id.clone());
    
    let result = executor.execute("channels.list", args).expect("channels.list failed");
    
    // Verify result structure
    assert!(result.contains("channels"));
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("Result is not valid JSON");
    assert!(parsed["channels"].is_array());
}

#[test]
#[ignore] // Requires Discord API access
fn test_channels_get() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    // First, list channels to get a valid channel ID
    let mut list_args = HashMap::new();
    list_args.insert("guild_id".to_string(), guild_id.clone());
    let list_result = executor.execute("channels.list", list_args)
        .expect("channels.list failed");
    
    let parsed: serde_json::Value = serde_json::from_str(&list_result)
        .expect("Result is not valid JSON");
    let channels = parsed["channels"].as_array()
        .expect("channels is not an array");
    
    if let Some(first_channel) = channels.first() {
        let channel_id = first_channel["channel_id"].as_str()
            .expect("channel_id not found");
        
        // Now get the specific channel
        let mut get_args = HashMap::new();
        get_args.insert("guild_id".to_string(), guild_id);
        get_args.insert("channel_id".to_string(), channel_id.to_string());
        
        let result = executor.execute("channels.get", get_args)
            .expect("channels.get failed");
        
        assert!(result.contains("channel_id"));
        let parsed: serde_json::Value = serde_json::from_str(&result)
            .expect("Result is not valid JSON");
        assert_eq!(parsed["channel_id"].as_str().unwrap(), channel_id);
    }
}

#[test]
#[ignore] // Requires Discord API access
fn test_members_list() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), guild_id);
    args.insert("limit".to_string(), "10".to_string());
    
    let result = executor.execute("members.list", args).expect("members.list failed");
    
    assert!(result.contains("members"));
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("Result is not valid JSON");
    assert!(parsed["members"].is_array());
}

#[test]
#[ignore] // Requires Discord API access
fn test_roles_list() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), guild_id);
    
    let result = executor.execute("roles.list", args).expect("roles.list failed");
    
    assert!(result.contains("roles"));
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("Result is not valid JSON");
    assert!(parsed["roles"].is_array());
}

// Argument validation tests

#[test]
#[ignore] // Requires Discord API access
fn test_guilds_get_missing_guild_id() {
    let executor = create_discord_executor();
    let args = HashMap::new(); // Missing guild_id
    
    let result = executor.execute("guilds.get", args);
    assert!(result.is_err(), "Should fail with missing guild_id");
    assert!(result.unwrap_err().to_string().contains("guild_id"));
}

#[test]
#[ignore] // Requires Discord API access
fn test_channels_get_missing_channel_id() {
    let executor = create_discord_executor();
    let guild_id = test_guild_id();
    
    let mut args = HashMap::new();
    args.insert("guild_id".to_string(), guild_id);
    // Missing channel_id
    
    let result = executor.execute("channels.get", args);
    assert!(result.is_err(), "Should fail with missing channel_id");
    assert!(result.unwrap_err().to_string().contains("channel_id"));
}

#[test]
#[ignore] // Requires Discord API access
fn test_invalid_command() {
    let executor = create_discord_executor();
    let args = HashMap::new();
    
    let result = executor.execute("invalid.command", args);
    assert!(result.is_err(), "Should fail with invalid command");
}
