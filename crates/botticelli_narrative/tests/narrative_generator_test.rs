//! Tests for NarrativeGenerator elicitation via a buffered communicator.
//!
//! These tests verify that the wizard answer sequence matches what
//! `NarrativeGenerator::elicit` actually consumes so bugs are caught at
//! compile time, not at first user interaction.

use botticelli_narrative::{NarrativeGenerator, TomlAct};
use elicitation::{Elicitation, Generator};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tracing::instrument;

// ── TestCommunicator ──────────────────────────────────────────────────────────

/// Minimal communicator that drains pre-loaded answers from a queue.
///
/// Mirrors the `BufferedCommunicator` in `botticelli_tui::controller` so the
/// same answer sequences work in both contexts.
#[derive(Clone)]
struct TestCommunicator {
    answers: Arc<Vec<String>>,
    idx: Arc<AtomicUsize>,
    style_context: elicitation::StyleContext,
    elicitation_context: elicitation::ElicitationContext,
}

impl TestCommunicator {
    fn new(answers: Vec<String>) -> Self {
        Self {
            answers: Arc::new(answers),
            idx: Arc::new(AtomicUsize::new(0)),
            style_context: elicitation::StyleContext::default(),
            elicitation_context: elicitation::ElicitationContext::default(),
        }
    }

    fn with_string_style_human(self) -> Self {
        let mut new = self.clone();
        if let Err(e) = new
            .style_context
            .set_style::<String, elicitation::StringStyle>(elicitation::StringStyle::Human)
        {
            panic!("set_style failed: {e}");
        }
        new
    }
}

impl elicitation::ElicitCommunicator for TestCommunicator {
    #[instrument(skip(self, _prompt))]
    async fn send_prompt(&self, _prompt: &str) -> elicitation::ElicitResult<String> {
        let i = self.idx.fetch_add(1, Ordering::SeqCst);
        Ok(self.answers.get(i).cloned().unwrap_or_default())
    }

    async fn call_tool(
        &self,
        _params: elicitation::rmcp::model::CallToolRequestParams,
    ) -> Result<elicitation::rmcp::model::CallToolResult, elicitation::rmcp::ServiceError> {
        Err(elicitation::rmcp::ServiceError::McpError(
            elicitation::rmcp::ErrorData::internal_error(
                "TestCommunicator::call_tool is not used in narrative elicitation".to_string(),
                None,
            ),
        ))
    }

    fn style_context(&self) -> &elicitation::StyleContext {
        &self.style_context
    }

    fn with_style<
        T: 'static,
        S: elicitation::StyleMarker + elicitation::style::ElicitationStyle + 'static,
    >(
        &self,
        style: S,
    ) -> Self {
        let mut new = self.clone();
        if let Err(e) = new.style_context.set_style::<T, S>(style) {
            tracing::error!(error = %e, "TestCommunicator::with_style failed");
        }
        new
    }

    fn elicitation_context(&self) -> &elicitation::ElicitationContext {
        &self.elicitation_context
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn elicit_minimal_narrative_one_prompt_act() {
    // Answer sequence for NarrativeGenerator with one Prompt act and no model.
    //
    // Protocol order:
    //   name        → String::elicit → send_prompt → "test_narrative"
    //   description → String::elicit → send_prompt → "A test narrative"
    //   acts (Vec)  → bool::elicit   → send_prompt → "true"  (add first item)
    //   acts[0].name → String::elicit → "first_act"
    //   acts[0].content (enum) → send_prompt with options → "Prompt"
    //   acts[0].content.Prompt inner → String::elicit → "Write a poem"
    //   acts (Vec)  → bool::elicit   → send_prompt → "false" (no more items)
    //   model (Option) → bool::elicit → send_prompt → "false" (skip)
    let answers = vec![
        "test_narrative".to_string(),
        "A test narrative".to_string(),
        "true".to_string(),
        "first_act".to_string(),
        "Prompt".to_string(),
        "Write a poem".to_string(),
        "false".to_string(),
        "false".to_string(),
    ];

    let comm = TestCommunicator::new(answers).with_string_style_human();

    let generator = NarrativeGenerator::elicit(&comm)
        .await
        .expect("elicitation should succeed with valid answer sequence");

    assert_eq!(generator.name, "test_narrative");
    assert_eq!(generator.description, "A test narrative");
    assert_eq!(generator.acts.len(), 1);
    assert_eq!(generator.acts[0].name, "first_act");
    assert!(
        matches!(
            &generator.acts[0].content,
            botticelli_narrative::NarrativeActContent::Prompt(p) if p == "Write a poem"
        ),
        "expected Prompt(\"Write a poem\"), got {:?}",
        generator.acts[0].content
    );
    assert_eq!(generator.model, None);
}

#[tokio::test]
async fn elicit_narrative_two_acts_different_types() {
    // Name, description, then two acts: one Prompt, one NarrativeRef.
    // Vec loop fires "true" twice (once per act) then "false" to stop.
    // Protocol:
    //   name → "multi_act"
    //   description → "Multi-act test"
    //   Vec loop 1 → "true"
    //   acts[0].name → "draft"
    //   acts[0].content variant → "Prompt"
    //   acts[0].content.Prompt → "Draft the content"
    //   Vec loop 2 → "true"
    //   acts[1].name → "review"
    //   acts[1].content variant → "NarrativeRef"
    //   acts[1].content.NarrativeRef → "editorial"
    //   Vec loop 3 → "false" (stop)
    //   model → "false" (skip)
    let answers = vec![
        "multi_act".to_string(),
        "Multi-act test".to_string(),
        "true".to_string(),
        "draft".to_string(),
        "Prompt".to_string(),
        "Draft the content".to_string(),
        "true".to_string(),
        "review".to_string(),
        "NarrativeRef".to_string(),
        "editorial".to_string(),
        "false".to_string(),
        "false".to_string(),
    ];

    let comm = TestCommunicator::new(answers).with_string_style_human();

    let generator = NarrativeGenerator::elicit(&comm)
        .await
        .expect("elicitation should succeed");

    assert_eq!(generator.name, "multi_act");
    assert_eq!(generator.acts.len(), 2);
    assert!(matches!(
        &generator.acts[0].content,
        botticelli_narrative::NarrativeActContent::Prompt(p) if p == "Draft the content"
    ));
    assert!(matches!(
        &generator.acts[1].content,
        botticelli_narrative::NarrativeActContent::NarrativeRef(k) if k == "editorial"
    ));
    assert_eq!(generator.model, None);
}

#[tokio::test]
async fn generate_produces_correct_toml_structure() {
    // End-to-end: elicit → generate → verify TOML structure.
    let answers = vec![
        "poem_bot".to_string(),
        "Writes poems on request".to_string(),
        "true".to_string(),
        "write".to_string(),
        "Prompt".to_string(),
        "Write a short poem about {topic}".to_string(),
        "false".to_string(),
        "false".to_string(),
    ];

    let comm = TestCommunicator::new(answers).with_string_style_human();

    let generator = NarrativeGenerator::elicit(&comm)
        .await
        .expect("elicitation should succeed");

    let file = generator.generate();

    assert!(
        file.acts.contains_key("write"),
        "generated file should have a 'write' act"
    );
    assert!(
        matches!(file.acts.get("write"), Some(TomlAct::Simple(p)) if p.contains("poem")),
        "write act should be a Simple prompt containing 'poem'"
    );
}

#[tokio::test]
async fn elicit_narrative_with_model() {
    // Verify that the Option<String> model field is elicited when "true" is given.
    // Protocol:
    //   name → "model_test"
    //   description → "With model"
    //   Vec loop → "false" (no acts)
    //   model → "true"  (provide a value)
    //   model value → "claude-sonnet-4-6"
    let answers = vec![
        "model_test".to_string(),
        "With model".to_string(),
        "false".to_string(),
        "true".to_string(),
        "claude-sonnet-4-6".to_string(),
    ];

    let comm = TestCommunicator::new(answers).with_string_style_human();

    let generator = NarrativeGenerator::elicit(&comm)
        .await
        .expect("elicitation should succeed");

    assert_eq!(generator.model, Some("claude-sonnet-4-6".to_string()));
    assert!(generator.acts.is_empty());
}
