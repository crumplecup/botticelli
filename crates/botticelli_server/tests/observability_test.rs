use botticelli_core::{init_observability, shutdown_observability};

fn init_succeeds_or_already_set() {
    match init_observability() {
        Ok(()) => {}
        Err(e) if e.to_string().contains("already been set") => {
            // A prior test already installed the global subscriber — that's fine.
        }
        Err(e) => panic!("init_observability failed unexpectedly: {e}"),
    }
}

#[test]
fn test_init_observability_without_metrics() {
    init_succeeds_or_already_set();
    shutdown_observability();
}

#[test]
#[cfg(feature = "metrics")]
fn test_init_observability_with_metrics() {
    init_succeeds_or_already_set();
    shutdown_observability();
}
