//! Test helpers for database tests.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test tracing with dotenvy support.
///
/// This function can be called multiple times safely - it will only
/// initialize tracing once. It loads .env files via dotenvy and respects
/// the RUST_LOG environment variable.
///
/// # Arguments
/// * `fallback_level` - The tracing level to use if RUST_LOG is not set (e.g., "info", "debug")
///
/// # Example
/// ```
/// use helpers;
///
/// #[tokio::test]
/// async fn my_test() -> anyhow::Result<()> {
///     helpers::init_test_tracing("info");
///     // Test code here
///     Ok(())
/// }
/// ```
pub fn init_test_tracing(fallback_level: &str) {
    INIT.call_once(|| {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        // Initialize tracing subscriber
        let filter = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| fallback_level.to_string());

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .init();
    });
}
