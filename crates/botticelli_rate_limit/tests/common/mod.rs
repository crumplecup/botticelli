use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize tracing subscriber for tests.
/// Reads RUST_LOG from environment, defaults to "debug".
/// Safe to call multiple times - only initializes once.
pub fn init_tracing() {
    INIT.call_once(|| {
        let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_string());

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
