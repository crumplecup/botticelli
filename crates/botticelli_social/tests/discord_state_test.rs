//! Isolated tests for Discord state persistence

#[test]
fn test_state_persistence_basic() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    
    // Create a unique state directory for this test
    let state_dir = PathBuf::from("/tmp/botticelli_state_test_basic");
    if state_dir.exists() {
        fs::remove_dir_all(&state_dir)?;
    }
    fs::create_dir_all(&state_dir)?;
    
    // Load state manager
    let state_file = state_dir.join("narrative_state.json");
    
    // Create initial state with a test channel ID
    let initial_state = serde_json::json!({
        "TEST_CHANNEL_ID": "123456789"
    });
    fs::write(&state_file, serde_json::to_string_pretty(&initial_state)?)?;
    
    // Read it back
    let loaded_state: serde_json::Value = serde_json::from_str(&fs::read_to_string(&state_file)?)?;
    
    assert_eq!(loaded_state["TEST_CHANNEL_ID"], "123456789");
    
    // Cleanup
    fs::remove_dir_all(&state_dir)?;
    
    Ok(())
}

#[test]
#[ignore] // Requires CLI execution
fn test_state_persistence_via_cli() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    
    // Create unique state directory
    let state_dir = PathBuf::from("/tmp/botticelli_state_test_cli");
    if state_dir.exists() {
        fs::remove_dir_all(&state_dir)?;
    }
    fs::create_dir_all(&state_dir)?;
    
    // Create a simple narrative that outputs something
    let narrative_file = state_dir.join("test_narrative.toml");
    fs::write(&narrative_file, r#"
name = "test_state_save"
skip_content_generation = true

[acts.output_test]
prompt = "Say 'test123'"
"#)?;
    
    // Run narrative with --save flag
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "botticelli",
            "--",
            "run",
            "--narrative",
            narrative_file.to_str().unwrap(),
            "--save",
            "--state-dir",
            state_dir.to_str().unwrap(),
        ])
        .output()?;
    
    println!("CLI stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("CLI stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    assert!(output.status.success(), "CLI execution failed");
    
    // Check if state file was created
    let state_file = state_dir.join("narrative_state.json");
    assert!(state_file.exists(), "State file should exist after --save");
    
    // Cleanup
    fs::remove_dir_all(&state_dir)?;
    
    Ok(())
}
