//! Discord command narrative tests.
//!
//! These tests execute Discord bot commands through narrative files
//! to verify end-to-end functionality.

use botticelli_error::BotticelliError;
use botticelli_narrative::{Narrative, NarrativeExecutor};
use botticelli_social::{BotCommandRegistryImpl, DiscordCommandExecutor};
use std::env;
use std::path::PathBuf;

/// Helper to run a Discord command test narrative.
fn run_discord_command_test(narrative_name: &str) -> Result<(), BotticelliError> {
    dotenvy::dotenv().ok();
    
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    let discord_executor = DiscordCommandExecutor::new(&token);
    
    let mut bot_registry = BotCommandRegistryImpl::new();
    bot_registry.register_discord(discord_executor);
    
    let narrative_path = PathBuf::from(format!("tests/narratives/{}.toml", narrative_name));
    let narrative = Narrative::from_file(&narrative_path)?;
    
    let executor = NarrativeExecutor::builder()
        .narrative(narrative)
        .bot_registry(bot_registry)
        .build()
        .expect("Failed to build executor");
    
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(executor.execute())?;
    
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_members_list() -> Result<(), BotticelliError> {
    run_discord_command_test("members_list")
}

#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_roles_list() -> Result<(), BotticelliError> {
    run_discord_command_test("roles_list")
}

#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_channels_list() -> Result<(), BotticelliError> {
    run_discord_command_test("channels_list")
}
