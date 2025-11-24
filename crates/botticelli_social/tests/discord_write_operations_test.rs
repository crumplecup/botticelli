//! Integration tests for Discord write operations
//!
//! These tests create, modify, and delete Discord resources to verify command functionality.

mod discord_write_test_helpers;

use discord_write_test_helpers::{WriteOperationTest, narrative_path};

#[test]
#[cfg_attr(not(feature = "discord"), ignore)]
#[ignore = "TODO: Replace narrative source - test files don't exist in expected location"]
fn test_channel_update() {
    WriteOperationTest::new(
        narrative_path("write_tests/channel_create_setup"),
        narrative_path("write_tests/channel_update_test"),
    )
    .with_teardown(narrative_path("write_tests/channel_create_teardown"))
    .run()
    .expect("Channel update test failed");
}
