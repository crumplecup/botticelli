//! Discord bot command tests.
//!
//! This file contains two types of tests:
//! 1. Parse-only tests: Fast validation that narrative files are syntactically correct
//! 2. Integration tests: Full execution tests that actually call Discord APIs

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

// ============================================================================
// Parse-Only Tests - Fast validation without API calls
// ============================================================================

use botticelli_narrative::Narrative;

/// Helper to load a narrative file for validation.
fn load_narrative(relative_path: &str) -> Result<Narrative, Box<dyn std::error::Error>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!("{}/tests/narratives/{}", manifest_dir, relative_path);
    Ok(Narrative::from_file(&narrative_path)?)
}

macro_rules! parse_test {
    ($name:ident, $file:expr) => {
        #[test]
        fn $name() -> Result<(), Box<dyn std::error::Error>> {
            let narrative = load_narrative($file)?;
            assert!(!narrative.acts().is_empty());
            Ok(())
        }
    };
}

parse_test!(parse_channels_list, "discord/channels_list_test.toml");
parse_test!(parse_channels_get, "discord/channels_get_test.toml");
parse_test!(parse_channels_create, "discord/channels_create_test.toml");
parse_test!(parse_channels_delete, "discord/channels_delete_test.toml");
parse_test!(parse_messages_list, "discord/messages_list_test.toml");
parse_test!(parse_messages_get, "discord/messages_get_test.toml");
parse_test!(parse_messages_send, "discord/messages_send_test.toml");
parse_test!(parse_messages_edit, "discord/messages_edit_test.toml");
parse_test!(parse_messages_delete, "discord/messages_delete_test.toml");
parse_test!(parse_messages_pin, "discord/messages_pin_test.toml");
parse_test!(parse_messages_unpin, "discord/messages_unpin_test.toml");
parse_test!(parse_members_list, "discord/members_list_test.toml");
parse_test!(parse_members_get, "discord/members_get_test.toml");
parse_test!(parse_roles_list, "discord/roles_list_test.toml");
parse_test!(parse_roles_get, "discord/roles_get_test.toml");
parse_test!(parse_reactions_add, "discord/reactions_add_test.toml");
parse_test!(parse_reactions_remove, "discord/reactions_remove_test.toml");
parse_test!(parse_threads_list, "discord/threads_list_test.toml");
parse_test!(parse_threads_create, "discord/threads_create_test.toml");
parse_test!(parse_emojis_list, "discord/emojis_list_test.toml");
parse_test!(parse_invites_list, "discord/invites_list_test.toml");
parse_test!(parse_bans_list, "discord/bans_list_test.toml");
parse_test!(parse_stickers_list, "discord/stickers_list_test.toml");
parse_test!(parse_voice_regions_list, "discord/voice_regions_list_test.toml");
parse_test!(parse_events_list, "discord/events_list_test.toml");
parse_test!(parse_server_get_stats, "discord/server_get_stats_test.toml");
parse_test!(parse_webhooks_list, "discord/webhooks_list_test.toml");
parse_test!(parse_integrations_list, "discord/integrations_list_test.toml");
parse_test!(parse_threads_get, "discord/threads_get_test.toml");
parse_test!(parse_reactions_list, "discord/reactions_list_test.toml");
parse_test!(parse_events_get, "discord/events_get_test.toml");

// ============================================================================
// Integration Tests - Full execution with Discord API
// ============================================================================

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

macro_rules! integration_test {
    ($name:ident, $file:expr) => {
        #[tokio::test]
        #[cfg_attr(not(feature = "discord"), ignore)]
        async fn $name() {
            load_env();
            run_test_narrative($file)
                .await
                .expect(&format!("{} narrative failed", $file));
        }
    };
}

integration_test!(test_channels_list, "channels_list_test");
integration_test!(test_channels_get, "channels_get_test");
integration_test!(test_messages_list, "messages_list_test");
integration_test!(test_messages_send, "messages_send_test");
integration_test!(test_members_list, "members_list_test");
integration_test!(test_server_get_stats, "server_get_stats_test");
integration_test!(test_webhooks_list, "webhooks_list_test");
integration_test!(test_integrations_list, "integrations_list_test");
