//! Integration tests for Discord bot commands using test narratives.

use std::{env, path::PathBuf};

/// Helper to load environment variables from .env
fn load_env() {
    dotenvy::dotenv().ok();
}

/// Helper to get path to test narrative
fn get_test_narrative_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/narratives/discord")
        .join(format!("{}.toml", name))
}

/// Helper to run a test narrative
async fn run_test_narrative(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let narrative_path = get_test_narrative_path(name);
    
    // Use botticelli CLI to run the narrative
    let output = tokio::process::Command::new("cargo")
        .args(&[
            "run",
            "-p",
            "botticelli",
            "--bin",
            "botticelli",
            "--features",
            "gemini,discord,database",
            "--",
            "run",
            "--narrative",
            narrative_path.to_str().unwrap(),
            "--process-discord",
        ])
        .output()
        .await?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Narrative {} failed:\n{}", name, stderr);
        return Err(format!("Narrative execution failed: {}", stderr).into());
    }
    
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_channels() {
    load_env();
    run_test_narrative("test_channels")
        .await
        .expect("test_channels narrative failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_roles() {
    load_env();
    run_test_narrative("test_roles")
        .await
        .expect("test_roles narrative failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_members() {
    load_env();
    run_test_narrative("test_members")
        .await
        .expect("test_members narrative failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_server_stats() {
    load_env();
    run_test_narrative("test_server_stats")
        .await
        .expect("test_server_stats narrative failed");
}

#[tokio::test]
#[cfg_attr(not(feature = "discord"), ignore)]
async fn test_messages() {
    load_env();
    run_test_narrative("test_messages")
        .await
        .expect("test_messages narrative failed");
}
