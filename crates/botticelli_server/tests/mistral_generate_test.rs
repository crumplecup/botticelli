//! End-to-end test for local inference via MistralDriver.
//!
//! Loads Qwen/Qwen2.5-Coder-0.5B-Instruct from the local HuggingFace cache
//! and verifies that generate() returns a non-empty response.
//! This is the same path the TUI uses for `--provider server`.
//!
//! Run with: `just test-api`

#[cfg(feature = "mistral")]
use botticelli_core::{GenerateRequest, Input, Message, Role};
#[cfg(feature = "mistral")]
use botticelli_interface::BotticelliDriver;
#[cfg(feature = "mistral")]
use botticelli_server::{MistralConfigBuilder, MistralDriver};

#[cfg(feature = "mistral")]
const MODEL: &str = "Qwen/Qwen2.5-Coder-0.5B-Instruct";

#[tokio::test]
#[cfg(feature = "mistral")]
#[cfg_attr(not(feature = "api"), ignore)]
async fn test_local_model_loads_and_generates() {
    let config = MistralConfigBuilder::default()
        .model_path(MODEL.to_string())
        .model_id(MODEL.to_string())
        .build()
        .expect("valid config");

    let driver = MistralDriver::load(config)
        .await
        .expect("model should load from local HuggingFace cache");

    let msg = Message::new(Role::User, vec![Input::Text("Say hello.".to_string())]);
    let req = GenerateRequest::new(vec![msg]);

    let resp = driver
        .generate(&req)
        .await
        .expect("generate should succeed");

    let text: String = resp
        .outputs()
        .iter()
        .filter_map(|o| {
            if let botticelli_core::Output::Text(t) = o {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect();

    assert!(!text.is_empty(), "local model should return non-empty text");
}
