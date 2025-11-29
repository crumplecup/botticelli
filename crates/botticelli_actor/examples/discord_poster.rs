//! Discord content poster actor example.
//!
//! This example demonstrates how to use the botticelli_actor system to create
//! a bot that periodically posts approved content to a Discord channel.
//!
//! # Environment Variables
//!
//! - `DATABASE_URL`: PostgreSQL connection string
//! - `DISCORD_TOKEN`: Discord bot token
//! - `DISCORD_CHANNEL_ID`: Target channel ID (numeric)
//!
//! # Usage
//!
//! ```bash
//! export DATABASE_URL="postgresql://user:pass@localhost/botticelli"
//! export DISCORD_TOKEN="your_bot_token"
//! export DISCORD_CHANNEL_ID="1234567890"
//! cargo run --example discord_poster --features discord
//! ```

use botticelli_actor::skills::{
    ContentFormatterSkill, ContentSchedulingSkill, ContentSelectionSkill, DuplicateCheckSkill,
    RateLimitingSkill,
};
use botticelli_actor::{
    Actor, ActorConfigBuilder, DiscordPlatform, ExecutionConfigBuilder, Skill, SkillRegistry,
};
use botticelli_database::create_pool;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .init();

    info!("Starting Discord content poster actor");

    // Load environment variables
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    let channel_id: u64 = std::env::var("DISCORD_CHANNEL_ID")
        .expect("DISCORD_CHANNEL_ID not set")
        .parse()
        .expect("DISCORD_CHANNEL_ID must be numeric");

    info!(channel_id = %channel_id, "Loaded configuration");

    // Connect to database
    let pool = create_pool().expect("Failed to create database pool");
    info!("Connected to database");

    // Create actor configuration
    let execution_config = ExecutionConfigBuilder::default()
        .continue_on_error(true)
        .stop_on_unrecoverable(true)
        .max_retries(3)
        .build()
        .expect("Valid execution config");

    let knowledge = vec![
        "content".to_string(),
        "post_history".to_string(),
        "actor_preferences".to_string(),
    ];

    let skills = vec![
        "content_selection".to_string(),
        "content_scheduling".to_string(),
        "rate_limiting".to_string(),
        "duplicate_check".to_string(),
        "content_formatter".to_string(),
    ];

    let config = ActorConfigBuilder::default()
        .name("discord_poster".to_string())
        .description("Posts approved content to Discord channel".to_string())
        .execution(execution_config)
        .knowledge(knowledge)
        .skills(skills)
        .build()
        .expect("Valid actor config");

    info!(actor = %config.name(), "Created actor configuration");

    // Create Discord platform
    let platform = Arc::new(DiscordPlatform::new(token, channel_id.to_string())?);
    info!("Created Discord platform adapter");

    // Register skills
    let mut registry = SkillRegistry::new();
    registry.register(Arc::new(ContentSelectionSkill::default()) as Arc<dyn Skill>);
    registry.register(Arc::new(ContentSchedulingSkill::default()) as Arc<dyn Skill>);
    registry.register(Arc::new(RateLimitingSkill::default()) as Arc<dyn Skill>);
    registry.register(Arc::new(DuplicateCheckSkill::default()) as Arc<dyn Skill>);
    registry.register(Arc::new(ContentFormatterSkill::default()) as Arc<dyn Skill>);

    info!("Registered skills");

    // Build actor
    let actor = Actor::builder()
        .config(config)
        .skills(registry)
        .platform(platform)
        .build()
        .expect("Valid actor");

    info!("Built actor, executing...");

    // Execute actor
    match actor.execute(&pool).await {
        Ok(result) => {
            info!("Actor execution completed");
            info!("  Succeeded: {}", result.succeeded.len());
            info!("  Failed: {}", result.failed.len());
            info!("  Skipped: {}", result.skipped.len());

            if !result.succeeded.is_empty() {
                info!("Successfully executed skills:");
                for output in &result.succeeded {
                    info!("  ✓ {}", output.skill_name);
                }
            }

            if !result.failed.is_empty() {
                error!("Failed skills:");
                for (skill_name, error) in &result.failed {
                    error!("  ✗ {}: {}", skill_name, error);
                }
            }

            if !result.skipped.is_empty() {
                info!("Skipped skills:");
                for skill_name in &result.skipped {
                    info!("  ⊘ {}", skill_name);
                }
            }

            Ok(())
        }
        Err(e) => {
            error!(error = ?e, "Actor execution failed");
            Err(e.into())
        }
    }
}
