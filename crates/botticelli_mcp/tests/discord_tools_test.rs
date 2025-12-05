//! Tests for Discord MCP tools.

#[cfg(feature = "discord")]
use botticelli_mcp::{
    DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool,
    DiscordPostMessageTool, McpTool,
};

#[cfg(feature = "discord")]
use serde_json::json;

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_tools_registration() {
    // Test that tools require DISCORD_TOKEN to be available
    let has_token = std::env::var("DISCORD_TOKEN").is_ok();

    let post_result = DiscordPostMessageTool::new();
    let get_messages_result = DiscordGetMessagesTool::new();
    let get_guild_result = DiscordGetGuildInfoTool::new();
    let get_channels_result = DiscordGetChannelsTool::new();

    // All results should match token availability
    assert_eq!(
        post_result.is_ok(),
        has_token,
        "Post message tool availability should match token presence"
    );
    assert_eq!(
        get_messages_result.is_ok(),
        has_token,
        "Get messages tool availability should match token presence"
    );
    assert_eq!(
        get_guild_result.is_ok(),
        has_token,
        "Get guild info tool availability should match token presence"
    );
    assert_eq!(
        get_channels_result.is_ok(),
        has_token,
        "Get channels tool availability should match token presence"
    );
}

#[cfg(feature = "discord")]
#[test]
fn test_discord_tool_schemas() {
    // Skip if no token available
    if std::env::var("DISCORD_TOKEN").is_err() {
        return;
    }

    let post_tool = DiscordPostMessageTool::new().expect("Tool should create with token");
    let get_messages_tool = DiscordGetMessagesTool::new().expect("Tool should create with token");
    let get_guild_tool = DiscordGetGuildInfoTool::new().expect("Tool should create with token");
    let get_channels_tool = DiscordGetChannelsTool::new().expect("Tool should create with token");

    // Verify tool names
    assert_eq!(post_tool.name(), "discord_post_message");
    assert_eq!(get_messages_tool.name(), "discord_get_messages");
    assert_eq!(get_guild_tool.name(), "discord_get_guild_info");
    assert_eq!(get_channels_tool.name(), "discord_get_channels");

    // Verify schemas have required fields
    let post_schema = post_tool.input_schema();
    assert!(post_schema.get("properties").is_some());
    assert!(post_schema
        .get("properties")
        .and_then(|p| p.get("channel_id"))
        .is_some());
    assert!(post_schema
        .get("properties")
        .and_then(|p| p.get("content"))
        .is_some());

    let get_messages_schema = get_messages_tool.input_schema();
    assert!(get_messages_schema
        .get("properties")
        .and_then(|p| p.get("channel_id"))
        .is_some());
    assert!(get_messages_schema
        .get("properties")
        .and_then(|p| p.get("limit"))
        .is_some());

    let get_guild_schema = get_guild_tool.input_schema();
    assert!(get_guild_schema
        .get("properties")
        .and_then(|p| p.get("guild_id"))
        .is_some());

    let get_channels_schema = get_channels_tool.input_schema();
    assert!(get_channels_schema
        .get("properties")
        .and_then(|p| p.get("guild_id"))
        .is_some());

    // Clean up mock token
    std::env::remove_var("DISCORD_TOKEN");
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_post_message_validation() {
    std::env::set_var("DISCORD_TOKEN", "test_token");
    let tool = DiscordPostMessageTool::new().expect("Tool should create");

    // Test missing channel_id
    let result = tool.execute(json!({"content": "test"})).await;
    assert!(result.is_err(), "Should fail without channel_id");

    // Test missing content
    let result = tool
        .execute(json!({"channel_id": "123456789"}))
        .await;
    assert!(result.is_err(), "Should fail without content");

    // Test content too long
    let long_content = "a".repeat(2001);
    let result = tool
        .execute(json!({
            "channel_id": "123456789",
            "content": long_content
        }))
        .await;
    assert!(
        result.is_err(),
        "Should fail with content over 2000 characters"
    );

    std::env::remove_var("DISCORD_TOKEN");
}
