//! Discord command narrative validation tests.
//!
//! These tests validate that Discord command narrative files can be loaded and parsed.
//! Actual execution tests should be in the botticelli crate's integration tests.

use botticelli_narrative::Narrative;

/// Helper to load a narrative file for validation.
fn load_narrative(relative_path: &str) -> Result<Narrative, Box<dyn std::error::Error>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!("{}/tests/narratives/{}", manifest_dir, relative_path);
    Ok(Narrative::from_file(&narrative_path)?)
}

#[test]
fn test_members_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("members_list.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_roles_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("roles_list.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_channels_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("channels_list.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_test_channels_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("discord/test_channels.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_channels_create_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("channels_create_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_messages_send_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("messages_send_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_messages_delete_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("messages_delete_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_channels_delete_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("channels_delete_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_messages_pin_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("messages_pin_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_messages_unpin_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("messages_unpin_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_reactions_remove_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("reactions_remove_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_threads_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("threads_list_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_emojis_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("emojis_list_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_invites_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("invites_list_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}

#[test]
fn test_bans_list_narrative_loads() -> Result<(), Box<dyn std::error::Error>> {
    let narrative = load_narrative("bans_list_test.toml")?;
    assert!(!narrative.acts().is_empty());
    Ok(())
}
