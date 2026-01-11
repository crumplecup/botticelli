//! Integration tests for Discord narratives.

mod helpers;

use botticelli_narrative::Narrative;

#[tokio::test]
async fn test_welcome_content_generation_loads() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing welcome content generation narrative loads");

    dotenvy::dotenv().ok();

    // Load narrative from file (relative to workspace root)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!(
        "{}/narratives/discord/welcome_content_generation.toml",
        manifest_dir
    );
    tracing::debug!(path = %narrative_path, "Loading narrative from file");

    let narrative = Narrative::from_file(&narrative_path)?;

    // Verify basic structure
    assert_eq!(narrative.metadata().name(), "welcome_messages");
    assert!(!narrative.acts().is_empty());
    tracing::debug!(
        name = narrative.metadata().name(),
        act_count = narrative.acts().len(),
        "Loaded narrative"
    );

    tracing::info!("Welcome content generation narrative loads test passed");
    Ok(())
}

#[tokio::test]
async fn test_publish_welcome_loads() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing publish welcome narrative loads");

    dotenvy::dotenv().ok();

    // Load narrative from file (relative to workspace root)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!("{}/narratives/discord/publish_welcome.toml", manifest_dir);
    tracing::debug!(path = %narrative_path, "Loading narrative from file");

    let narrative = Narrative::from_file(&narrative_path)?;

    // Verify basic structure
    assert_eq!(narrative.metadata().name(), "publish_welcome");
    assert!(!narrative.acts().is_empty());
    tracing::debug!(
        name = narrative.metadata().name(),
        act_count = narrative.acts().len(),
        "Loaded narrative"
    );

    tracing::info!("Publish welcome narrative loads test passed");
    Ok(())
}
