//! Test helper functions for botticelli_core tests.

use botticelli_core::{ExporterBackend, ObservabilityConfig, init_observability_with_config};
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize tracing for tests using botticelli_core observability.
///
/// This function sets up OpenTelemetry observability:
/// - Uses stdout exporter for test output
/// - Reads RUST_LOG from environment (including .env files)
/// - Falls back to the provided level if not set
/// - Only initializes once per test run (safe to call multiple times)
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

        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| fallback_level.to_string());

        let _ = ObservabilityConfig::builder()
            .service_name("core-tests")
            .exporter(ExporterBackend::Stdout)
            .enable_metrics(false)
            .log_level(log_level)
            .build()
            .ok()
            .and_then(|config| init_observability_with_config(config).ok());
    });
}
