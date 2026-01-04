/// Test helper utilities for botticelli_models
pub mod test_doubles;

pub use test_doubles::*;

/// Initialize environment for tests (load .env)
pub fn init_test_env() {
    let _ = dotenvy::dotenv();
}
