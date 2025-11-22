//! Integration tests for Discord bot commands using narrative-based testing.

use botticelli_narrative::{Narrative, NarrativeExecutor};
use botticelli_social::BotRegistry;
use std::env;
use std::path::PathBuf;

/// Helper to load environment variables from .env
fn load_env() {
    dotenvy::dotenv().ok();
}

/// Helper to get test guild ID from environment
fn get_test_guild_id() -> String {
    env::var("TEST_GUILD_ID").expect("TEST_GUILD_ID not set in environment")
}

/// Helper to get test channel ID from environment
fn get_test_channel_id() -> String {
    env::var("TEST_CHANNEL_ID").expect("TEST_CHANNEL_ID not set in environment")
}

/// Helper to get Discord token from environment
fn get_discord_token() -> String {
    env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set in environment")
}

/// Helper to create a bot registry with Discord configured
fn create_bot_registry() -> BotRegistry {
    let token = get_discord_token();
    let mut registry = BotRegistry::new();
    registry.register_discord(token).unwrap();
    registry
}

/// Helper to run a test narrative
async fn run_test_narrative(narrative_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    
    // Build path to test narrative
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/narratives/discord");
    path.push(format!("{}.toml", narrative_name));
    
    // Set environment variables for template substitution
    env::set_var("TEST_GUILD_ID", get_test_guild_id());
    if narrative_name.contains("message") {
        env::set_var("TEST_CHANNEL_ID", get_test_channel_id());
    }
    
    // Load narrative
    let narrative = Narrative::from_file(&path)?;
    
    // Create executor with bot registry
    let bot_registry = create_bot_registry();
    let mut executor = NarrativeExecutor::new(narrative);
    executor.with_bot_registry(bot_registry);
    
    // Execute narrative
    executor.execute().await?;
    
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_channel_commands() {
    run_test_narrative("test_channels")
        .await
        .expect("Channel commands test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_message_commands() {
    run_test_narrative("test_messages")
        .await
        .expect("Message commands test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_role_commands() {
    run_test_narrative("test_roles")
        .await
        .expect("Role commands test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_emojis_list() {
    run_test_narrative("test_emojis_list")
        .await
        .expect("Emojis list test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_stickers_list() {
    run_test_narrative("test_stickers_list")
        .await
        .expect("Stickers list test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_voice_regions_list() {
    run_test_narrative("test_voice_regions_list")
        .await
        .expect("Voice regions list test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_events_list() {
    run_test_narrative("test_events_list")
        .await
        .expect("Events list test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_integrations_list() {
    run_test_narrative("test_integrations_list")
        .await
        .expect("Integrations list test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_invites_list() {
    run_test_narrative("test_invites_list")
        .await
        .expect("Invites list test failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_webhooks_list() {
    run_test_narrative("test_webhooks_list")
        .await
        .expect("Webhooks list test failed");
}
