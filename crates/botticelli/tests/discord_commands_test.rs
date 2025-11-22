//! Integration tests for Discord bot commands using narrative-based testing.
//!
//! These tests execute narrative TOML files to validate Discord command functionality.

use botticelli::{
    BotCommandRegistryImpl, DiscordCommandExecutor, Narrative, NarrativeExecutor,
};
use botticelli_models::GeminiClient;
use std::env;
use std::path::PathBuf;

/// Helper to get path to test narrative
fn get_test_narrative_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("narratives")
        .join("discord")
        .join(format!("{}.toml", name))
}

/// Test guilds.get command
#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_guilds_get() {
    dotenvy::dotenv().ok();

    let gemini_client = GeminiClient::new().expect("Failed to create Gemini client");

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
    let discord_executor = DiscordCommandExecutor::new(discord_token);
    
    let mut bot_registry = BotCommandRegistryImpl::new();
    bot_registry.register(discord_executor);

    let executor = NarrativeExecutor::new(gemini_client)
        .with_bot_registry(Box::new(bot_registry));

    let path = get_test_narrative_path("test_guilds_get");
    assert!(path.exists(), "Test narrative not found: {:?}", path);

    let narrative = Narrative::from_file(&path).expect("Failed to load narrative");
    let result = executor.execute(&narrative).await;
    
    assert!(result.is_ok(), "Failed to execute guilds.get: {:?}", result.err());
}

/// Test channels.list command
#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_channels_list() {
    dotenvy::dotenv().ok();

    let gemini_client = GeminiClient::new().expect("Failed to create Gemini client");

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
    let discord_executor = DiscordCommandExecutor::new(discord_token);
    
    let mut bot_registry = BotCommandRegistryImpl::new();
    bot_registry.register(discord_executor);

    let executor = NarrativeExecutor::new(gemini_client)
        .with_bot_registry(Box::new(bot_registry));

    let path = get_test_narrative_path("test_channels_list");
    assert!(path.exists(), "Test narrative not found: {:?}", path);

    let narrative = Narrative::from_file(&path).expect("Failed to load narrative");
    let result = executor.execute(&narrative).await;
    
    assert!(result.is_ok(), "Failed to execute channels.list: {:?}", result.err());
}

/// Test messages.send command
#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_messages_send() {
    dotenvy::dotenv().ok();

    let gemini_client = GeminiClient::new().expect("Failed to create Gemini client");

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
    let discord_executor = DiscordCommandExecutor::new(discord_token);
    
    let mut bot_registry = BotCommandRegistryImpl::new();
    bot_registry.register(discord_executor);

    let executor = NarrativeExecutor::new(gemini_client)
        .with_bot_registry(Box::new(bot_registry));

    let path = get_test_narrative_path("test_messages_send");
    assert!(path.exists(), "Test narrative not found: {:?}", path);

    let narrative = Narrative::from_file(&path).expect("Failed to load narrative");
    let result = executor.execute(&narrative).await;
    
    assert!(result.is_ok(), "Failed to execute messages.send: {:?}", result.err());
}
