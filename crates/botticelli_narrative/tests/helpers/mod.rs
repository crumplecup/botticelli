//! Test helpers for botticelli_narrative tests.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing for tests with environment-aware configuration.
///
/// Reads RUST_LOG from environment (including .env file), falling back to
/// the provided level if not set. Safe to call multiple times - subsequent
/// calls are no-ops.
///
/// # Arguments
/// * `fallback_level` - Log level to use if RUST_LOG not set (e.g., "info", "debug")
///
/// # Example
/// ```no_run
/// use helpers::init_test_tracing;
///
/// #[test]
/// fn my_test() {
///     init_test_tracing("info");
///     // Test code with tracing...
/// }
/// ```
pub fn init_test_tracing(fallback_level: &str) {
    let _ = dotenvy::dotenv();

    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| fallback_level.to_string())
        .parse::<EnvFilter>()
        .unwrap_or_else(|_| EnvFilter::new(fallback_level));

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_test_writer())
        .try_init();
}
