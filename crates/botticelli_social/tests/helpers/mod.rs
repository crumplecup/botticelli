/// Test helper utilities for botticelli_social

/// Initialize environment for tests (load .env)
pub fn init_test_env() {
    let _ = dotenvy::dotenv();
}

/// Initialize tracing for tests.
///
/// Reads RUST_LOG from environment (including .env file) and falls back
/// to the specified default level if not set.
///
/// Safe to call multiple times - subsequent calls are no-ops.
///
/// # Arguments
///
/// * `fallback_level` - Default log level if RUST_LOG not set (e.g., "info", "debug")
///
/// # Examples
///
/// ```no_run
/// // In a test file:
/// #[test]
/// fn my_test() {
///     helpers::init_test_tracing("debug");
///     tracing::info!("Test message");
///     // Test code with tracing enabled
/// }
/// ```
///
/// # With .env file
///
/// ```text
/// # .env
/// RUST_LOG=botticelli_social=trace,debug
/// ```
///
/// Then run tests: `cargo test -- --nocapture` to see tracing output.
pub fn init_test_tracing(fallback_level: &str) {
    use tracing_subscriber::EnvFilter;

    // Load .env if present
    let _ = dotenvy::dotenv();

    // Try to get RUST_LOG from environment, fall back to provided level
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| fallback_level.to_string());

    // Only initialize once (subsequent calls are no-ops)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_level))
        .with_test_writer()
        .try_init();
}
