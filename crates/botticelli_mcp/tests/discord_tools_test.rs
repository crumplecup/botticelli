//! Tests for Discord MCP tools.

#[cfg(feature = "discord")]
mod helpers;

#[cfg(feature = "discord")]
use botticelli_mcp::ToolRegistry;

#[cfg(feature = "discord")]
use serde_json::json;

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_tools_available_with_token() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Discord tools availability with token");

    // Test that tools work when DISCORD_TOKEN is available
    let has_token = std::env::var("DISCORD_TOKEN").is_ok();

    if !has_token {
        tracing::info!("Skipping - no DISCORD_TOKEN available");
        return Ok(());
    }

    let registry = ToolRegistry::default();
    let definitions = registry.tool_definitions().await?;

    // Check that Discord tools are registered
    let discord_tools: Vec<_> = definitions
        .iter()
        .filter(|def| def.name.starts_with("discord_"))
        .collect();

    assert!(
        !discord_tools.is_empty(),
        "Should have Discord tools registered"
    );

    // Verify expected tools exist
    let tool_names: Vec<_> = discord_tools.iter().map(|def| def.name.as_str()).collect();
    assert!(tool_names.contains(&"discord_post_message"));
    assert!(tool_names.contains(&"discord_get_messages"));
    assert!(tool_names.contains(&"discord_get_guild_info"));
    assert!(tool_names.contains(&"discord_get_channels"));

    tracing::info!("Discord tools registration test passed");
    Ok(())
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_tool_schemas() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Discord tool schemas");

    // Skip if no token available
    if std::env::var("DISCORD_TOKEN").is_err() {
        tracing::info!("Skipping - no DISCORD_TOKEN available");
        return Ok(());
    }

    let registry = ToolRegistry::default();
    let definitions = registry.tool_definitions().await?;

    // Find Discord tool definitions
    let post_def = definitions
        .iter()
        .find(|d| d.name == "discord_post_message");
    let get_messages_def = definitions
        .iter()
        .find(|d| d.name == "discord_get_messages");
    let get_guild_def = definitions
        .iter()
        .find(|d| d.name == "discord_get_guild_info");
    let get_channels_def = definitions
        .iter()
        .find(|d| d.name == "discord_get_channels");

    // Verify tool names exist
    assert!(post_def.is_some(), "Should have discord_post_message tool");
    assert!(
        get_messages_def.is_some(),
        "Should have discord_get_messages tool"
    );
    assert!(
        get_guild_def.is_some(),
        "Should have discord_get_guild_info tool"
    );
    assert!(
        get_channels_def.is_some(),
        "Should have discord_get_channels tool"
    );

    // Verify schemas have required fields
    let post_schema = &post_def.unwrap().input_schema;
    assert!(post_schema.get("properties").is_some());
    assert!(
        post_schema
            .get("properties")
            .and_then(|p| p.get("channel_id"))
            .is_some(),
        "discord_post_message should require channel_id"
    );
    assert!(
        post_schema
            .get("properties")
            .and_then(|p| p.get("content"))
            .is_some(),
        "discord_post_message should require content"
    );

    let get_messages_schema = &get_messages_def.unwrap().input_schema;
    assert!(
        get_messages_schema
            .get("properties")
            .and_then(|p| p.get("channel_id"))
            .is_some(),
        "discord_get_messages should require channel_id"
    );

    let get_guild_schema = &get_guild_def.unwrap().input_schema;
    assert!(
        get_guild_schema
            .get("properties")
            .and_then(|p| p.get("guild_id"))
            .is_some(),
        "discord_get_guild_info should require guild_id"
    );

    let get_channels_schema = &get_channels_def.unwrap().input_schema;
    assert!(
        get_channels_schema
            .get("properties")
            .and_then(|p| p.get("guild_id"))
            .is_some(),
        "discord_get_channels should require guild_id"
    );

    tracing::info!("Discord tool schemas test passed");
    Ok(())
}

#[cfg(feature = "discord")]
#[tokio::test]
async fn test_discord_post_message_validation() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Discord post message validation");

    // Skip if no token
    if std::env::var("DISCORD_TOKEN").is_err() {
        tracing::info!("Skipping - no DISCORD_TOKEN available");
        return Ok(());
    }

    let registry = ToolRegistry::default();

    // Test missing channel_id
    let result = registry
        .execute("discord_post_message", json!({"content": "test"}))
        .await;
    assert!(result.is_err(), "Should fail without channel_id");
    tracing::debug!("Correctly rejected missing channel_id");

    // Test missing content
    let result = registry
        .execute("discord_post_message", json!({"channel_id": "123456789"}))
        .await;
    assert!(result.is_err(), "Should fail without content");
    tracing::debug!("Correctly rejected missing content");

    // Test content too long
    let long_content = "a".repeat(2001);
    let result = registry
        .execute(
            "discord_post_message",
            json!({
                "channel_id": "123456789",
                "content": long_content
            }),
        )
        .await;
    assert!(
        result.is_err(),
        "Should fail with content over 2000 characters"
    );
    tracing::debug!("Correctly rejected content over 2000 chars");

    tracing::info!("Discord validation test passed");
    Ok(())
}
