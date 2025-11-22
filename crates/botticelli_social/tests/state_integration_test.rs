//! Integration tests for state management with bot commands.

use std::path::PathBuf;

/// Helper to run a narrative with state management enabled
fn run_narrative_with_state(narrative_name: &str) -> Result<(), String> {
    let narrative_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/narratives/discord")
        .join(format!("{}.toml", narrative_name));
    
    let state_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/test_state");
    
    let output = std::process::Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "botticelli",
            "--",
            "run",
            "--narrative",
            narrative_path.to_str().unwrap(),
            "--state-dir",
            state_dir.to_str().unwrap(),
            "--process-discord",
        ])
        .output()
        .map_err(|e| format!("Failed to execute narrative: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Narrative {} failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            narrative_name, stdout, stderr
        ));
    }
    
    Ok(())
}

#[test]
#[ignore] // Run with: cargo test --features discord state_integration -- --ignored
fn test_state_integration_lifecycle() {
    // This test requires DISCORD_TOKEN and TEST_GUILD_ID environment variables
    dotenvy::dotenv().ok();
    
    // Step 1: Create a channel - should save channel_id to state
    println!("Creating channel...");
    run_narrative_with_state("state_test_create")
        .expect("Failed to create channel");
    
    // Step 2: Use the channel_id from state to send a message
    println!("Sending message using state...");
    run_narrative_with_state("state_test_use")
        .expect("Failed to send message with state ID");
    
    // Step 3: Clean up - delete the channel using state ID
    println!("Cleaning up channel...");
    run_narrative_with_state("state_test_cleanup")
        .expect("Failed to cleanup channel");
    
    println!("✅ State integration test passed!");
}
