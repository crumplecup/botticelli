//! Debug utilities for profiling TUI performance.

use std::time::Instant;
use tracing::{info, warn};

/// Measures and logs the time taken for an operation.
pub struct OperationTimer {
    name: &'static str,
    start: Instant,
}

impl OperationTimer {
    pub fn new(name: &'static str) -> Self {
        let start = Instant::now();
        Self { name, start }
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if elapsed.as_millis() > 10 {
            warn!("{} took {:?} (SLOW!)", self.name, elapsed);
        } else {
            info!("{} took {:?}", self.name, elapsed);
        }
    }
}
