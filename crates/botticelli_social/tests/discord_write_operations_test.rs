//! Integration tests for Discord write operations
//!
//! These tests create, modify, and delete Discord resources to verify command functionality.

#![cfg(feature = "discord")]

mod discord_write_test_helpers;
mod helpers;

use discord_write_test_helpers::{WriteOperationTest, narrative_path};

#[test]
#[cfg_attr(not(feature = "api"), ignore)]
fn test_channel_update() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!(test = "test_channel_update", "Starting channel update test");

    WriteOperationTest::new(
        narrative_path("write_tests/channel_create_setup"),
        narrative_path("write_tests/channel_update_test"),
    )
    .with_teardown(narrative_path("write_tests/channel_create_teardown"))
    .run()?;

    tracing::info!("Channel update test completed successfully");
    Ok(())
}
