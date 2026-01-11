//! Test helper functions for botticelli_rate_limit tests.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize tracing for tests.
///
/// This function sets up a tracing subscriber that:
/// - Reads RUST_LOG from environment (including .env files)
/// - Falls back to the provided level if not set
/// - Only initializes once per test run (safe to call multiple times)
/// - Uses test writer for better test output integration
///
/// # Arguments
///
/// * `fallback_level` - Default log level if RUST_LOG is not set
///
/// # Example
///
/// ```
/// helpers::init_test_tracing("info");
/// ```
pub fn init_test_tracing(fallback_level: &str) {
    INIT.call_once(|| {
        // Load .env file if present (for local development)
        let _ = dotenvy::dotenv();

        let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| fallback_level.to_string());

        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_test_writer()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(true)
            .init();
    });
}
