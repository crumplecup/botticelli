//! Integration tests for Discord bot commands using narrative-based testing.
//!
//! These tests execute narrative TOML files to validate Discord command functionality.

use botticelli::{
    BotRegistry, DiscordBotClient, NarrativeExecutor, NarrativeRepository,
    PostgresNarrativeRepository,
};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::env;
use std::path::PathBuf;

/// Test helper to set up executor with Discord bot
fn setup_executor() -> (NarrativeExecutor, Pool<ConnectionManager<PgConnection>>) {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create pool");

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
    let discord_client = DiscordBotClient::new(discord_token);

    let mut bot_registry = BotRegistry::new();
    bot_registry.register_discord(discord_client);

    let repository = PostgresNarrativeRepository::new(pool.clone());

    let mut executor = NarrativeExecutor::new(Box::new(repository));
    executor.with_bot_registry(bot_registry);

    (executor, pool)
}

/// Helper to get path to test narrative
fn get_test_narrative_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("narratives")
        .join("discord")
        .join(format!("{}.toml", name))
}

/// Test server commands
#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_server_commands() {
    let (mut executor, _pool) = setup_executor();
    let path = get_test_narrative_path("test_server_commands");

    let result = executor.execute_from_file(&path);
    assert!(
        result.is_ok(),
        "Server commands test failed: {:?}",
        result.err()
    );
}

/// Test channel commands (create, list, get, edit, delete)
#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_channel_commands() {
    let (mut executor, _pool) = setup_executor();
    let path = get_test_narrative_path("test_channel_commands");

    let result = executor.execute_from_file(&path);
    assert!(
        result.is_ok(),
        "Channel commands test failed: {:?}",
        result.err()
    );
}

/// Test role commands (create, list, get, edit, delete)
#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_role_commands() {
    let (mut executor, _pool) = setup_executor();
    let path = get_test_narrative_path("test_role_commands");

    let result = executor.execute_from_file(&path);
    assert!(
        result.is_ok(),
        "Role commands test failed: {:?}",
        result.err()
    );
}

/// Test member commands (list, get)
#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_member_commands() {
    let (mut executor, _pool) = setup_executor();
    let path = get_test_narrative_path("test_member_commands");

    let result = executor.execute_from_file(&path);
    assert!(
        result.is_ok(),
        "Member commands test failed: {:?}",
        result.err()
    );
}

/// Test message commands (send, get, list, edit, pin, unpin, delete)
#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_message_commands() {
    let (mut executor, _pool) = setup_executor();
    let path = get_test_narrative_path("test_message_commands");

    let result = executor.execute_from_file(&path);
    assert!(
        result.is_ok(),
        "Message commands test failed: {:?}",
        result.err()
    );
}

/// Test reaction commands (add, list, remove, clear)
#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
fn test_reaction_commands() {
    let (mut executor, _pool) = setup_executor();
    let path = get_test_narrative_path("test_reaction_commands");

    let result = executor.execute_from_file(&path);
    assert!(
        result.is_ok(),
        "Reaction commands test failed: {:?}",
        result.err()
    );
}
