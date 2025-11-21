//! Integration tests for Discord narratives.

use botticelli_narrative::Narrative;

/// Helper to load .env file for tests
fn load_env() {
    dotenvy::dotenv().ok();
}

#[tokio::test]
async fn test_welcome_content_generation_loads() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    
    // Load narrative from file (relative to workspace root)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!("{}/narratives/discord/welcome_content_generation.toml", manifest_dir);
    let narrative = Narrative::from_file(&narrative_path)?;
    
    // Verify basic structure
    assert_eq!(narrative.metadata.name, "welcome_messages");
    assert!(!narrative.acts.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_publish_welcome_loads() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    
    // Load narrative from file (relative to workspace root)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!("{}/narratives/discord/publish_welcome.toml", manifest_dir);
    let narrative = Narrative::from_file(&narrative_path)?;
    
    // Verify basic structure
    assert_eq!(narrative.metadata.name, "publish_welcome");
    assert!(!narrative.acts.is_empty());
    
    Ok(())
}
