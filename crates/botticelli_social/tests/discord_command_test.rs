//! Discord bot command tests.
//!
//! This file contains two types of tests:
//! 1. Parse-only tests: Fast validation that narrative files are syntactically correct
//! 2. Integration tests: Full execution tests that actually call Discord APIs

mod helpers;

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
fn load_narrative(relative_path: &str) -> anyhow::Result<Narrative> {
    tracing::debug!(path = %relative_path, "Loading narrative");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let narrative_path = format!("{}/tests/narratives/{}", manifest_dir, relative_path);
    let narrative = Narrative::from_file(&narrative_path)?;
    tracing::debug!(acts = narrative.acts().len(), "Narrative loaded");
    Ok(narrative)
}

macro_rules! parse_test {
    ($name:ident, $file:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            helpers::init_test_tracing("info");
            tracing::info!(test = stringify!($name), file = $file, "Starting parse test");
            let narrative = load_narrative($file)?;
            assert!(!narrative.acts().is_empty());
            tracing::debug!(acts = narrative.acts().len(), "Validation passed");
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
parse_test!(
    parse_voice_regions_list,
    "discord/voice_regions_list_test.toml"
);
parse_test!(parse_events_list, "discord/events_list_test.toml");
parse_test!(parse_server_get_stats, "discord/server_get_stats_test.toml");
parse_test!(parse_webhooks_list, "discord/webhooks_list_test.toml");
parse_test!(
    parse_integrations_list,
    "discord/integrations_list_test.toml"
);
parse_test!(parse_threads_get, "discord/threads_get_test.toml");
parse_test!(parse_reactions_list, "discord/reactions_list_test.toml");
parse_test!(parse_events_get, "discord/events_get_test.toml");

// ============================================================================
// Integration Tests - Full execution with Discord API
// ============================================================================

/// Helper to run a test narrative
async fn run_test_narrative(name: &str) -> anyhow::Result<()> {
    tracing::debug!(narrative = %name, "Running test narrative");
    let narrative_path = get_test_narrative_path(name);
    let narrative_str = narrative_path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid narrative path"))?;

    tracing::debug!(path = %narrative_str, "Executing botticelli CLI");
    
    // Use botticelli CLI to run the narrative
    let output = tokio::process::Command::new("cargo")
        .args([
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
            narrative_str,
            "--process-discord",
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!(narrative = %name, stderr = %stderr, "Narrative execution failed");
        anyhow::bail!("Narrative execution failed: {}", stderr);
    }

    tracing::debug!(narrative = %name, "Narrative execution succeeded");
    Ok(())
}

macro_rules! integration_test {
    ($name:ident, $file:expr) => {
        #[tokio::test]
        #[cfg_attr(not(feature = "discord"), ignore)]
        #[cfg_attr(not(feature = "api"), ignore)]
        async fn $name() -> anyhow::Result<()> {
            helpers::init_test_tracing("info");
            tracing::info!(test = stringify!($name), narrative = $file, "Starting integration test");
            load_env();
            run_test_narrative($file).await?;
            tracing::info!(test = stringify!($name), "Integration test passed");
            Ok(())
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

// TODO: Re-enable once write_channel_lifecycle_test narrative is created
// #[tokio::test]
// async fn test_write_channel_lifecycle() {
//     load_env();
//     run_test_narrative("write_channel_lifecycle_test").await.expect("write_channel_lifecycle_test failed");
// }
