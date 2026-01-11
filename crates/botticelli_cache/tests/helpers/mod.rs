//! Test helpers for botticelli_cache tests.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize tracing for tests.
///
/// Safe to call multiple times - only initializes once.
/// Reads RUST_LOG from environment (including .env files).
pub fn init_test_tracing(fallback_level: &str) {
    INIT.call_once(|| {
        let _ = dotenvy::dotenv();

        let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| fallback_level.to_string());

        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_test_writer()
            .init();
    });
}
