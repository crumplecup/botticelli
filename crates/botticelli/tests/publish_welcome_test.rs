//! Integration test for publish_welcome narrative.
//!
//! This test requires multiple features to be enabled:
//! - `gemini` - For LLM generation
//! - `discord` - For bot command execution
//! - `database` - For table queries
//!
//! Run with: `cargo test --package botticelli --test publish_welcome_test --features gemini,discord,database,api`

#![cfg(all(feature = "gemini", feature = "discord", feature = "database"))]

use botticelli_database::{BotStorageTableQueryRegistry, RedbStorage};
use botticelli_interface::BotStorage;
use botticelli_models::GeminiClient;
use botticelli_narrative::{Narrative, NarrativeExecutor};
use botticelli_social::{BotCommandRegistryImpl, DiscordCommandExecutor};
use dotenvy::dotenv;
use std::{env, sync::Arc};

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_publish_welcome() {
    // Load environment variables
    dotenv().ok();

    // Get required environment variables
    let _gemini_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    // Set up in-memory storage for table queries
    let storage: Arc<dyn BotStorage> =
        Arc::new(RedbStorage::in_memory().expect("in-memory redb for test"));

    // Get narratives directory (tests run from workspace root)
    let narratives_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/botticelli_narrative/narratives/discord");

    // Create the driver
    let driver = GeminiClient::new().expect("Failed to create Gemini client");

    // Create bot registry with Discord support
    let mut bot_registry = BotCommandRegistryImpl::new();
    let discord_executor = DiscordCommandExecutor::new(&discord_token);
    bot_registry.register(discord_executor);

    // Create table query registry backed by storage
    let table_registry = BotStorageTableQueryRegistry::new(Arc::clone(&storage));

    // Create executor with bot and table support
    let executor = NarrativeExecutor::new(driver)
        .with_bot_registry(Box::new(bot_registry))
        .with_table_registry(Box::new(table_registry));

    // First, run welcome_content_generation to populate the table
    let gen_path = narratives_dir.join("welcome_content_generation.toml");
    println!(
        "Loading welcome_content_generation from: {}",
        gen_path.display()
    );
    let gen_narrative = Narrative::from_file(&gen_path)
        .expect("Failed to load welcome_content_generation narrative");

    println!("Executing welcome_content_generation narrative...");
    let gen_result = executor
        .execute(&gen_narrative)
        .await
        .expect("Failed to execute welcome_content_generation narrative");

    println!(
        "welcome_content_generation completed: {} acts",
        gen_result.act_executions().len()
    );

    // Now run publish_welcome which references the generated content
    let publish_path = narratives_dir.join("publish_welcome.toml");
    println!("\nLoading publish_welcome from: {}", publish_path.display());
    let publish_narrative =
        Narrative::from_file(&publish_path).expect("Failed to load publish_welcome narrative");

    println!("Executing publish_welcome narrative...");
    let result = executor
        .execute(&publish_narrative)
        .await
        .expect("Failed to execute publish_welcome narrative");

    // Verify we got results
    assert!(
        !result.act_executions().is_empty(),
        "Should have act executions"
    );

    println!("\nPublish_welcome executed successfully:");
    for act in result.act_executions() {
        println!("  Act {}: {}", act.sequence_number(), act.act_name());
        println!("    Response length: {} chars", act.response().len());
    }
}
