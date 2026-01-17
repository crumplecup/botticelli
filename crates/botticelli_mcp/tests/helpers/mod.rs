/// Test helper utilities for botticelli_mcp tests.

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
/// mod helpers;
///
/// #[test]
/// fn my_test() -> anyhow::Result<()> {
///     helpers::init_test_tracing("debug");
///     tracing::info!("Test starting");
///     // Test code with tracing enabled
///     Ok(())
/// }
/// ```
///
/// # With RUST_LOG
///
/// ```bash
/// RUST_LOG=debug cargo test -- --nocapture
/// RUST_LOG=botticelli_mcp=trace,info cargo test -- --nocapture
/// ```
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
