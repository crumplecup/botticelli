//! Tests for elicitation crate derive macros.
//!
//! Verifies that #[derive(Elicit)] works correctly with MockCommunicator.
//! These tests exercise the Select and Survey paradigms.

use elicitation::{ElicitCommunicator, ElicitError, Elicitation, ElicitationContext, StyleContext};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Simple enum to test Select paradigm.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, elicitation::Elicit,
)]
#[prompt("Choose an option:")]
enum TestChoice {
    OptionA,
    OptionB,
    OptionC,
}

/// Simple struct to test Survey paradigm with bool field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, elicitation::Elicit)]
#[prompt("Configure settings:")]
struct TestSettings {
    #[prompt("Enable feature?")]
    enabled: bool,
}

/// Mock communicator backed by a queue of preset responses.
#[derive(Clone)]
struct MockCommunicator {
    responses: Arc<Vec<String>>,
    call_count: Arc<AtomicUsize>,
    style_context: StyleContext,
    elicitation_context: ElicitationContext,
}

impl MockCommunicator {
    fn new(responses: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            responses: Arc::new(responses.into_iter().map(Into::into).collect()),
            call_count: Arc::new(AtomicUsize::new(0)),
            style_context: StyleContext::default(),
            elicitation_context: ElicitationContext::default(),
        }
    }

    fn single(response: impl Into<String>) -> Self {
        Self::new([response.into()])
    }
}

impl ElicitCommunicator for MockCommunicator {
    async fn send_prompt(&self, _prompt: &str) -> Result<String, ElicitError> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        let response = self.responses.get(idx).cloned().unwrap_or_default();
        Ok(response)
    }

    async fn call_tool(
        &self,
        _params: rmcp::model::CallToolRequestParams,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ServiceError> {
        unimplemented!("call_tool not used in these tests")
    }

    fn style_context(&self) -> &StyleContext {
        &self.style_context
    }

    fn with_style<
        T: 'static,
        S: elicitation::StyleMarker + elicitation::style::ElicitationStyle + 'static,
    >(
        &self,
        _style: S,
    ) -> Self {
        self.clone()
    }

    fn elicitation_context(&self) -> &ElicitationContext {
        &self.elicitation_context
    }
}

#[tokio::test]
async fn test_select_option_a() {
    let mock = MockCommunicator::single("1");
    let result = TestChoice::elicit(&mock)
        .await
        .expect("elicitation succeeded");
    assert_eq!(result, TestChoice::OptionA);
}

#[tokio::test]
async fn test_select_option_b() {
    let mock = MockCommunicator::single("2");
    let result = TestChoice::elicit(&mock)
        .await
        .expect("elicitation succeeded");
    assert_eq!(result, TestChoice::OptionB);
}

#[tokio::test]
async fn test_select_option_c() {
    let mock = MockCommunicator::single("3");
    let result = TestChoice::elicit(&mock)
        .await
        .expect("elicitation succeeded");
    assert_eq!(result, TestChoice::OptionC);
}

#[tokio::test]
async fn test_survey_enabled_true() {
    let mock = MockCommunicator::single("true");
    let result = TestSettings::elicit(&mock)
        .await
        .expect("elicitation succeeded");
    assert!(result.enabled);
}

#[tokio::test]
async fn test_survey_enabled_false() {
    let mock = MockCommunicator::single("false");
    let result = TestSettings::elicit(&mock)
        .await
        .expect("elicitation succeeded");
    assert!(!result.enabled);
}
