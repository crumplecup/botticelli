//! Isolated tests for Discord state persistence

mod helpers;

#[test]
fn test_state_persistence_basic() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!(test = "test_state_persistence_basic", "Starting state persistence test");
    use std::fs;
    use std::path::PathBuf;

    // Create a unique state directory for this test
    let state_dir = PathBuf::from("/tmp/botticelli_state_test_basic");
    tracing::debug!(path = ?state_dir, "Creating state directory");
    if state_dir.exists() {
        fs::remove_dir_all(&state_dir)?;
    }
    fs::create_dir_all(&state_dir)?;

    // Load state manager
    let state_file = state_dir.join("narrative_state.json");
    tracing::debug!(path = ?state_file, "State file path");

    // Create initial state with a test channel ID
    let initial_state = serde_json::json!({
        "TEST_CHANNEL_ID": "123456789"
    });
    fs::write(&state_file, serde_json::to_string_pretty(&initial_state)?)?;

    // Read it back
    tracing::debug!("Reading state back from file");
    let loaded_state: serde_json::Value = serde_json::from_str(&fs::read_to_string(&state_file)?)?;

    assert_eq!(loaded_state["TEST_CHANNEL_ID"], "123456789");
    tracing::debug!(channel_id = %loaded_state["TEST_CHANNEL_ID"], "State loaded successfully");

    // Cleanup
    tracing::debug!("Cleaning up test directory");
    fs::remove_dir_all(&state_dir)?;

    Ok(())
}

#[test]
#[cfg(feature = "discord")]
#[ignore = "TODO: Replace narrative source - test files don't exist in expected location"]
fn test_state_persistence_across_narratives() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!(test = "test_state_persistence_across_narratives", "Starting cross-narrative persistence test");
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    // Create unique state directory
    let state_dir = PathBuf::from("/tmp/botticelli_state_test_persistence");
    tracing::debug!(path = ?state_dir, "Creating state directory");
    if state_dir.exists() {
        fs::remove_dir_all(&state_dir)?;
    }
    fs::create_dir_all(&state_dir)?;

    let state_file = state_dir.join("global.json"); // StateManager uses "global.json" for global scope
    tracing::debug!(path = ?state_file, "State file path");

    // Run setup narrative to create and store channel ID
    tracing::debug!("Running setup narrative");
    let state_dir_str = state_dir.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid state directory path"))?;
    let setup_output = Command::new("just")
        .args([
            "narrate",
            "crates/botticelli_social/tests/narratives/discord/state_test_create.toml",
        ])
        .env("BOTTICELLI_STATE_DIR", state_dir_str)
        .output()?;

    println!(
        "Setup stdout: {}",
        String::from_utf8_lossy(&setup_output.stdout)
    );
    println!(
        "Setup stderr: {}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    assert!(setup_output.status.success(), "Setup narrative failed");

    // Verify state file exists and contains channel ID
    assert!(state_file.exists(), "State file should exist after setup");
    let state_content = fs::read_to_string(&state_file)?;
    let state: serde_json::Value = serde_json::from_str(&state_content)?;
    // The state has a "data" wrapper, so we need to look in data.id
    let data = state.get("data").expect("State should have data field");
    assert!(data.get("id").is_some(), "id should be in state data");
    println!(
        "State after setup: {}",
        serde_json::to_string_pretty(&state)?
    );

    // Run use narrative that reads the channel ID from state
    tracing::debug!("Running use narrative");
    let use_output = Command::new("just")
        .args([
            "narrate",
            "crates/botticelli_social/tests/narratives/discord/state_test_use.toml",
        ])
        .env("BOTTICELLI_STATE_DIR", state_dir_str)
        .output()?;

    println!(
        "Use stdout: {}",
        String::from_utf8_lossy(&use_output.stdout)
    );
    println!(
        "Use stderr: {}",
        String::from_utf8_lossy(&use_output.stderr)
    );

    assert!(
        use_output.status.success(),
        "Use narrative should succeed with persisted state"
    );

    // Run cleanup narrative
    tracing::debug!("Running cleanup narrative");
    let cleanup_output = Command::new("just")
        .args([
            "narrate",
            "crates/botticelli_social/tests/narratives/discord/state_test_cleanup.toml",
        ])
        .env("BOTTICELLI_STATE_DIR", state_dir_str)
        .output()?;

    println!(
        "Cleanup stdout: {}",
        String::from_utf8_lossy(&cleanup_output.stdout)
    );
    println!(
        "Cleanup stderr: {}",
        String::from_utf8_lossy(&cleanup_output.stderr)
    );

    // Cleanup test directory
    tracing::debug!("Cleaning up test directory");
    fs::remove_dir_all(&state_dir)?;

    tracing::info!("Test completed successfully");
    Ok(())
}
