//! Integration tests for state management with bot commands.

/// Helper to run a narrative using `just narrate`
fn run_narrative_with_just(narrative_name: &str) -> Result<(), String> {
    let narrative_path = format!(
        "crates/botticelli_social/tests/narratives/discord/{}.toml",
        narrative_name
    );

    let output = std::process::Command::new("just")
        .args(["narrate", &narrative_path])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| format!("Failed to execute just narrate: {}", e))?;

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
#[ignore = "TODO: Replace narrative source - test files don't exist in expected location"]
fn test_state_integration_lifecycle() {
    // This test requires DISCORD_TOKEN and TEST_GUILD_ID environment variables
    dotenvy::dotenv().ok();

    // Step 1: Create a channel - should save channel_id to state
    println!("Creating channel...");
    run_narrative_with_just("state_test_create").expect("Failed to create channel");

    // Step 2: Use the channel_id from state to send a message
    println!("Sending message using state...");
    run_narrative_with_just("state_test_use").expect("Failed to send message with state ID");

    // Step 3: Clean up - delete the channel using state ID
    println!("Cleaning up channel...");
    run_narrative_with_just("state_test_cleanup").expect("Failed to cleanup channel");

    println!("✅ State integration test passed!");
}
