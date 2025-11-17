//! Integration tests for narrative processor pipeline.
//!
//! These tests verify the complete flow from narrative execution through
//! processor pipeline to database insertion.

#![cfg(all(feature = "database", feature = "discord"))]

use boticelli::{
    BoticelliDriver, DiscordChannelProcessor, DiscordGuildProcessor, DiscordRepository,
    DiscordUserProcessor, GenerateRequest, GenerateResponse, Narrative, NarrativeExecutor, Output,
    ProcessorRegistry,
};
use std::sync::Arc;

/// Mock LLM driver that returns predefined responses.
struct MockDriver {
    responses: Vec<String>,
    call_count: std::sync::Mutex<usize>,
}

impl MockDriver {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl BoticelliDriver for MockDriver {
    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    async fn generate(
        &self,
        _request: &GenerateRequest,
    ) -> boticelli::BoticelliResult<GenerateResponse> {
        let mut count = self.call_count.lock().unwrap();
        let response = self.responses.get(*count).cloned().unwrap_or_default();
        *count += 1;

        Ok(GenerateResponse {
            outputs: vec![Output::Text(response)],
        })
    }
}

/// Create test database connection.
fn create_test_db() -> diesel::PgConnection {
    use diesel::Connection;
    
    // Load .env file
    let _ = dotenvy::dotenv();
    
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");

    diesel::PgConnection::establish(&database_url).expect("Failed to connect to test database")
}

/// Clean up test guilds.
fn cleanup_guilds(conn: &mut diesel::PgConnection, guild_ids: &[i64]) {
    use boticelli::discord_guilds;
    use diesel::prelude::*;

    for guild_id in guild_ids {
        diesel::delete(discord_guilds::table.filter(discord_guilds::id.eq(guild_id)))
            .execute(conn)
            .ok();
    }
}

/// Clean up test users.
fn cleanup_users(conn: &mut diesel::PgConnection, user_ids: &[i64]) {
    use boticelli::discord_users;
    use diesel::prelude::*;

    for user_id in user_ids {
        diesel::delete(discord_users::table.filter(discord_users::id.eq(user_id)))
            .execute(conn)
            .ok();
    }
}

/// Clean up test channels.
fn cleanup_channels(conn: &mut diesel::PgConnection, channel_ids: &[i64]) {
    use boticelli::discord_channels;
    use diesel::prelude::*;

    for channel_id in channel_ids {
        diesel::delete(discord_channels::table.filter(discord_channels::id.eq(channel_id)))
            .execute(conn)
            .ok();
    }
}

#[tokio::test]
async fn test_discord_guild_processor_integration() {
    // Setup
    let mut conn = create_test_db();
    let test_guild_id = 999888777666555444i64;
    cleanup_guilds(&mut conn, &[test_guild_id]);

    // Create mock driver with guild JSON response
    let mock_responses = vec![
        r#"{
        "id": 999888777666555444,
        "name": "Integration Test Guild",
        "owner_id": 111222333444555666,
        "description": "Created by integration test",
        "member_count": 42,
        "verification_level": 2
    }"#
        .to_string(),
    ];

    let driver = MockDriver::new(mock_responses);

    // Setup processor pipeline
    let repo = Arc::new(DiscordRepository::new(create_test_db()));
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(repo.clone())));

    let executor = NarrativeExecutor::with_processors(driver, registry);

    // Create test narrative
    let narrative_toml = r#"
[narration]
name = "test_guild_generation"
description = "Test narrative for guild processor"

[toc]
order = ["generate_guild"]

[acts]
generate_guild = "Generate guild JSON"
"#;

    let narrative: Narrative = narrative_toml.parse().expect("Failed to parse narrative");

    // Execute narrative (processors run automatically)
    let _execution = executor
        .execute(&narrative)
        .await
        .expect("Narrative execution failed");

    // Verify data was inserted into database
    use boticelli::discord_guilds;
    use diesel::prelude::*;

    let guild = discord_guilds::table
        .find(test_guild_id)
        .first::<boticelli::GuildRow>(&mut conn)
        .expect("Guild not found in database");

    assert_eq!(guild.id, test_guild_id);
    assert_eq!(guild.name, "Integration Test Guild");
    assert_eq!(guild.owner_id, 111222333444555666);
    assert_eq!(
        guild.description,
        Some("Created by integration test".to_string())
    );
    assert_eq!(guild.member_count, Some(42));

    // Cleanup
    cleanup_guilds(&mut conn, &[test_guild_id]);
}

#[tokio::test]
async fn test_discord_user_processor_integration() {
    // Setup
    let mut conn = create_test_db();
    let test_user_id = 888777666555444333i64;
    cleanup_users(&mut conn, &[test_user_id]);

    // Create mock driver with user JSON response
    let mock_responses = vec![
        r#"{
        "id": 888777666555444333,
        "username": "testuser",
        "discriminator": "1234",
        "global_name": "Test User",
        "bot": false
    }"#
        .to_string(),
    ];

    let driver = MockDriver::new(mock_responses);

    // Setup processor pipeline
    let repo = Arc::new(DiscordRepository::new(create_test_db()));
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordUserProcessor::new(repo.clone())));

    let executor = NarrativeExecutor::with_processors(driver, registry);

    // Create test narrative
    let narrative_toml = r#"
[narration]
name = "test_user_generation"
description = "Test narrative for user processor"

[toc]
order = ["generate_user"]

[acts]
generate_user = "Generate user JSON"
"#;

    let narrative: Narrative = narrative_toml.parse().expect("Failed to parse narrative");

    // Execute narrative
    let _execution = executor
        .execute(&narrative)
        .await
        .expect("Narrative execution failed");

    // Verify data was inserted
    use boticelli::discord_users;
    use diesel::prelude::*;

    let user = discord_users::table
        .find(test_user_id)
        .first::<boticelli::UserRow>(&mut conn)
        .expect("User not found in database");

    assert_eq!(user.id, test_user_id);
    assert_eq!(user.username, "testuser");
    assert_eq!(user.discriminator, Some("1234".to_string()));
    assert_eq!(user.global_name, Some("Test User".to_string()));
    assert_eq!(user.bot, Some(false));

    // Cleanup
    cleanup_users(&mut conn, &[test_user_id]);
}

#[tokio::test]
async fn test_multiple_processors_integration() {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::TRACE)
        .try_init();

    // Setup
    let mut conn = create_test_db();
    let test_guild_id = 777666555444333222i64;
    let test_channel_id = 666555444333222111i64;
    cleanup_guilds(&mut conn, &[test_guild_id]);
    cleanup_channels(&mut conn, &[test_channel_id]);

    // Create mock driver with both guild and channel JSON
    let mock_responses = vec![
        r#"{
        "id": 777666555444333222,
        "name": "Multi-Processor Guild",
        "owner_id": 111222333444555666
    }"#
        .to_string(),
        r#"{
        "id": 666555444333222111,
        "guild_id": 777666555444333222,
        "name": "general",
        "channel_type": "guild_text",
        "position": 0
    }"#
        .to_string(),
    ];

    let driver = MockDriver::new(mock_responses);

    // Setup processor pipeline with multiple processors
    let repo = Arc::new(DiscordRepository::new(create_test_db()));
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(repo.clone())));
    registry.register(Box::new(DiscordChannelProcessor::new(repo.clone())));

    let executor = NarrativeExecutor::with_processors(driver, registry);

    // Create test narrative with two acts
    let narrative_toml = r#"
[narration]
name = "test_multi_processor"
description = "Test narrative with multiple processors"

[toc]
order = ["generate_guild", "generate_channel"]

[acts]
generate_guild = "Generate guild JSON"
generate_channel = "Generate channel JSON"
"#;

    let narrative: Narrative = narrative_toml.parse().expect("Failed to parse narrative");

    // Execute narrative
    let _execution = executor
        .execute(&narrative)
        .await
        .expect("Narrative execution failed");

    // Verify both entities were inserted
    use boticelli::{discord_channels, discord_guilds};
    use diesel::prelude::*;

    let guild = discord_guilds::table
        .find(test_guild_id)
        .first::<boticelli::GuildRow>(&mut conn)
        .expect("Guild not found in database");

    assert_eq!(guild.id, test_guild_id);
    assert_eq!(guild.name, "Multi-Processor Guild");

    let channel = discord_channels::table
        .find(test_channel_id)
        .first::<boticelli::ChannelRow>(&mut conn)
        .expect("Channel not found in database");

    assert_eq!(channel.id, test_channel_id);
    assert_eq!(channel.guild_id, Some(test_guild_id));
    assert_eq!(channel.name, Some("general".to_string()));

    // Cleanup
    cleanup_channels(&mut conn, &[test_channel_id]);
    cleanup_guilds(&mut conn, &[test_guild_id]);
}

#[tokio::test]
async fn test_processor_handles_invalid_json() {
    // Setup
    let conn = create_test_db();

    // Create mock driver with invalid JSON
    let mock_responses = vec![
        "This is not JSON at all".to_string(),
        r#"{ "invalid": "missing required fields" }"#.to_string(),
    ];

    let driver = MockDriver::new(mock_responses);

    // Setup processor pipeline
    let repo = Arc::new(DiscordRepository::new(conn));
    let mut registry = ProcessorRegistry::new();
    registry.register(Box::new(DiscordGuildProcessor::new(repo.clone())));

    let executor = NarrativeExecutor::with_processors(driver, registry);

    // Create test narrative
    let narrative_toml = r#"
[narration]
name = "test_error_handling"
description = "Test narrative with invalid responses"

[toc]
order = ["act1", "act2"]

[acts]
act1 = "Generate invalid response 1"
act2 = "Generate invalid response 2"
"#;

    let narrative: Narrative = narrative_toml.parse().expect("Failed to parse narrative");

    // Execute narrative - should NOT fail even with invalid JSON
    let result = executor.execute(&narrative).await;

    // Execution should succeed (processors log errors but don't fail narrative)
    assert!(
        result.is_ok(),
        "Narrative should succeed despite processor errors"
    );

    let execution = result.unwrap();
    assert_eq!(
        execution.act_executions.len(),
        2,
        "Both acts should complete"
    );
}
