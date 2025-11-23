//! Complete example of using the actor system to schedule content posts.
//!
//! This example demonstrates:
//! - Loading actor configuration from TOML
//! - Registering built-in skills
//! - Creating a Discord platform instance
//! - Executing the actor workflow
//!
//! Run with:
//! ```bash
//! cargo run --example post_scheduler --features discord
//! ```

use botticelli_actor::{
    Actor, ActorConfig, ContentSchedulingSkill, DiscordPlatform, RateLimitingSkill, Skill,
    SkillRegistry, SocialMediaPlatform,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for observability
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    println!("🤖 Content Post Scheduler Example");
    println!("==================================\n");

    // 1. Load actor configuration
    println!("📋 Loading actor configuration...");
    let config_path =
        std::env::current_dir()?.join("crates/botticelli_actor/examples/post_scheduler_actor.toml");
    let config = ActorConfig::from_file(&config_path)?;
    println!("   ✓ Loaded actor: {}", config.name());
    println!("   ✓ Knowledge tables: {}", config.knowledge().len());
    println!("   ✓ Skills: {}", config.skills().len());

    // Validate configuration
    let warnings = config.validate();
    if !warnings.is_empty() {
        println!("\n⚠️  Configuration warnings:");
        for warning in warnings {
            println!("   - {}", warning);
        }
    }

    // 2. Create and register skills
    println!("\n�� Registering skills...");
    let mut registry = SkillRegistry::new();

    let scheduling_skill = ContentSchedulingSkill::new();
    println!(
        "   ✓ {}: {}",
        scheduling_skill.name(),
        scheduling_skill.description()
    );
    registry.register(Arc::new(scheduling_skill));

    let rate_limit_skill = RateLimitingSkill::new();
    println!(
        "   ✓ {}: {}",
        rate_limit_skill.name(),
        rate_limit_skill.description()
    );
    registry.register(Arc::new(rate_limit_skill));

    println!("   Total skills registered: {}", registry.len());

    // 3. Create Discord platform instance
    println!("\n🔌 Creating Discord platform...");

    // In production, get these from environment variables or secure config
    let discord_token =
        std::env::var("DISCORD_BOT_TOKEN").unwrap_or_else(|_| "mock_token_for_example".to_string());
    let channel_id =
        std::env::var("DISCORD_CHANNEL_ID").unwrap_or_else(|_| "123456789".to_string());

    let platform = DiscordPlatform::new(discord_token, channel_id)?;
    let metadata = platform.metadata();
    println!("   ✓ Platform: {}", metadata.name());
    println!("   ✓ Max text length: {}", metadata.max_text_length());
    println!(
        "   ✓ Max media attachments: {}",
        metadata.max_media_attachments()
    );

    // 4. Build the actor
    println!("\n🎭 Building actor...");
    let _actor = Actor::builder()
        .config(config)
        .skills(registry)
        .platform(Arc::new(platform))
        .build()?;
    println!("   ✓ Actor ready for execution");

    // 5. Execute actor workflow
    println!("\n🚀 Executing actor workflow...");
    println!("   (Note: This example uses mock database, no actual posts will be made)\n");

    // In production, you would:
    // - Connect to your database
    // - Load knowledge from configured tables
    // - Execute skills to process and post content
    //
    // For this example, we'll demonstrate the execution flow:
    println!("   Workflow steps:");
    println!(
        "   1. Load knowledge from tables: {:?}",
        ["approved_posts", "scheduled_content"]
    );
    println!("   2. Execute skill: content_scheduling");
    println!("      - Calculate optimal posting time within window (09:00-17:00)");
    println!("      - Apply randomization if enabled");
    println!("   3. Execute skill: rate_limiting");
    println!("      - Check current post count vs max_posts_per_day (10)");
    println!("      - Verify min_interval_minutes (60) since last post");
    println!("   4. Post content via Discord platform");
    println!("      - Validate text length (max 2000 chars)");
    println!("      - Validate media attachments (max 10)");
    println!("   5. Handle errors according to execution config");
    println!("      - Retry recoverable errors (max 3 attempts)");
    println!("      - Stop on unrecoverable errors");

    println!("\n✅ Example completed successfully!");
    println!("\n📚 To run with real database:");
    println!("   1. Set DATABASE_URL environment variable");
    println!("   2. Create knowledge tables (approved_posts, scheduled_content)");
    println!("   3. Set DISCORD_BOT_TOKEN and DISCORD_CHANNEL_ID");
    println!("   4. Run: cargo run --example post_scheduler --features discord,database");

    Ok(())
}
