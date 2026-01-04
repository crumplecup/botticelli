/// Test helper utilities for botticelli_models
pub mod mock_gemini;

pub use mock_gemini::*;

/// Initialize environment for tests (load .env)
pub fn init_test_env() {
    let _ = dotenvy::dotenv();
}
