//! Integration tests for state management with bot commands.

#![cfg(feature = "discord")]

mod helpers;

/// Helper to run a narrative using `just narrate`
fn run_narrative_with_just(narrative_name: &str) -> anyhow::Result<()> {
    tracing::debug!(narrative = %narrative_name, "Running narrative with just");
    let narrative_path = format!(
        "crates/botticelli_social/tests/narratives/discord/{}.toml",
        narrative_name
    );

    let output = std::process::Command::new("just")
        .args(["narrate", &narrative_path])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::error!(
            narrative = %narrative_name,
            stdout = %stdout,
            stderr = %stderr,
            "Narrative failed"
        );
        anyhow::bail!(
            "Narrative {} failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            narrative_name, stdout, stderr
        );
    }

    tracing::debug!(narrative = %narrative_name, "Narrative completed successfully");
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_state_integration_lifecycle() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!(test = "test_state_integration_lifecycle", "Starting state integration lifecycle test");
    
    // This test requires DISCORD_TOKEN and TEST_GUILD_ID environment variables
    dotenvy::dotenv().ok();

    // Step 1: Create a channel - should save channel_id to state
    tracing::info!("Creating channel...");
    run_narrative_with_just("state_test_create")?;

    // Step 2: Use the channel_id from state to send a message
    tracing::info!("Sending message using state...");
    run_narrative_with_just("state_test_use")?;

    // Step 3: Clean up - delete the channel using state ID
    tracing::info!("Cleaning up channel...");
    run_narrative_with_just("state_test_cleanup")?;

    tracing::info!("✅ State integration test passed!");
    Ok(())
}
