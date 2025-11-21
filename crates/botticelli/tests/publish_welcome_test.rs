//! Integration test for publish_welcome narrative.

use botticelli::{
    BotCommandRegistryImpl, DatabaseTableQueryRegistry, DiscordCommandExecutor, GeminiClient,
    Narrative, NarrativeExecutor, TableQueryExecutor, establish_connection,
};
use dotenvy::dotenv;
use std::{env, sync::{Arc, Mutex}};

#[tokio::test]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_publish_welcome() {
    // Load environment variables
    dotenv().ok();

    // Get required environment variables
    let gemini_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    // Set up database connection
    let conn = establish_connection().expect("Failed to connect to database");
    let conn = Arc::new(Mutex::new(conn));

    // Load the narrative (tests run from workspace root)
    let narrative_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/botticelli_narrative/narratives/discord/publish_welcome.toml");
    
    println!("Loading narrative from: {}", narrative_path.display());
    let narrative = Narrative::from_file(&narrative_path)
        .expect("Failed to load publish_welcome narrative");

    // Create the driver
    let driver = GeminiClient::new().expect("Failed to create Gemini client");

    // Create bot registry with Discord support
    let mut bot_registry = BotCommandRegistryImpl::new();
    let discord_executor = DiscordCommandExecutor::new(&discord_token);
    bot_registry.register(discord_executor);

    // Create table query registry
    let table_executor = TableQueryExecutor::new(conn.clone());
    let table_registry = DatabaseTableQueryRegistry::new(table_executor);

    // Create executor with bot and table support
    let executor = NarrativeExecutor::new(driver)
        .with_bot_registry(Box::new(bot_registry))
        .with_table_registry(Box::new(table_registry));

    // Execute the narrative
    let result = executor
        .execute(&narrative)
        .await
        .expect("Failed to execute publish_welcome narrative");

    // Verify we got results
    assert!(!result.act_executions.is_empty(), "Should have act executions");

    println!("Narrative executed successfully:");
    for act in &result.act_executions {
        println!("  Act {}: {}", act.sequence_number, act.act_name);
        println!("    Response length: {} chars", act.response.len());
    }
}
